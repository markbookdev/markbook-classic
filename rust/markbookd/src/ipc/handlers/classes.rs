use crate::ipc::error::{err, ok};
use crate::ipc::types::{AppState, Request};
use rusqlite::OptionalExtension;
use serde_json::json;
use uuid::Uuid;

fn handle_classes_list(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return ok(&req.id, json!({ "classes": [] }));
    };

    // Include basic counts so the UI can show a useful dashboard.
    // Use correlated subqueries to avoid double-counting from joins.
    let mut stmt = match conn.prepare(
        "SELECT
           c.id,
           c.name,
           (SELECT COUNT(*) FROM students s WHERE s.class_id = c.id) AS student_count,
           (SELECT COUNT(*) FROM mark_sets ms WHERE ms.class_id = c.id) AS mark_set_count
         FROM classes c
         ORDER BY c.name",
    ) {
        Ok(s) => s,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    let rows = stmt
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let student_count: i64 = row.get(2)?;
            let mark_set_count: i64 = row.get(3)?;
            Ok(json!({
                "id": id,
                "name": name,
                "studentCount": student_count,
                "markSetCount": mark_set_count
            }))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>());

    match rows {
        Ok(classes) => ok(&req.id, json!({ "classes": classes })),
        Err(e) => err(&req.id, "db_query_failed", e.to_string(), None),
    }
}

fn handle_classes_create(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let name = match req.params.get("name").and_then(|v| v.as_str()) {
        Some(v) => v.trim().to_string(),
        None => return err(&req.id, "bad_params", "missing name", None),
    };
    if name.is_empty() {
        return err(&req.id, "bad_params", "name must not be empty", None);
    }

    let class_id = Uuid::new_v4().to_string();
    if let Err(e) = conn.execute(
        "INSERT INTO classes(id, name) VALUES(?, ?)",
        (&class_id, &name),
    ) {
        return err(
            &req.id,
            "db_insert_failed",
            e.to_string(),
            Some(json!({ "table": "classes" })),
        );
    }

    ok(&req.id, json!({ "classId": class_id, "name": name }))
}

fn handle_classes_delete(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };

    let exists: Option<i64> = match conn
        .query_row("SELECT 1 FROM classes WHERE id = ?", [&class_id], |r| {
            r.get(0)
        })
        .optional()
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    if exists.is_none() {
        return err(&req.id, "not_found", "class not found", None);
    }

    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return err(&req.id, "db_tx_failed", e.to_string(), None),
    };

    // Explicitly delete in dependency order (no ON DELETE CASCADE).
    // NOTE: additional tables will be added over time; keep this list updated.
    if let Err(e) = tx.execute(
        "DELETE FROM scores
         WHERE assessment_id IN (
           SELECT a.id
           FROM assessments a
           JOIN mark_sets ms ON ms.id = a.mark_set_id
           WHERE ms.class_id = ?
         )",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "scores" })),
        );
    }

    if let Err(e) = tx.execute(
        "DELETE FROM comment_set_remarks
         WHERE comment_set_index_id IN (
           SELECT csi.id
           FROM comment_set_indexes csi
           WHERE csi.class_id = ?
         )",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "comment_set_remarks" })),
        );
    }

    if let Err(e) = tx.execute(
        "DELETE FROM comment_set_indexes WHERE class_id = ?",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "comment_set_indexes" })),
        );
    }

    if let Err(e) = tx.execute(
        "DELETE FROM attendance_student_months WHERE class_id = ?",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "attendance_student_months" })),
        );
    }

    if let Err(e) = tx.execute(
        "DELETE FROM attendance_months WHERE class_id = ?",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "attendance_months" })),
        );
    }

    if let Err(e) = tx.execute(
        "DELETE FROM attendance_settings WHERE class_id = ?",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "attendance_settings" })),
        );
    }

    if let Err(e) = tx.execute(
        "DELETE FROM seating_assignments WHERE class_id = ?",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "seating_assignments" })),
        );
    }

    if let Err(e) = tx.execute("DELETE FROM seating_plans WHERE class_id = ?", [&class_id]) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "seating_plans" })),
        );
    }

    if let Err(e) = tx.execute("DELETE FROM loaned_items WHERE class_id = ?", [&class_id]) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "loaned_items" })),
        );
    }

    if let Err(e) = tx.execute(
        "DELETE FROM student_device_map WHERE class_id = ?",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "student_device_map" })),
        );
    }

    if let Err(e) = tx.execute(
        "DELETE FROM assessments
         WHERE mark_set_id IN (SELECT id FROM mark_sets WHERE class_id = ?)",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "assessments" })),
        );
    }

    if let Err(e) = tx.execute(
        "DELETE FROM categories
         WHERE mark_set_id IN (SELECT id FROM mark_sets WHERE class_id = ?)",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "categories" })),
        );
    }

    if let Err(e) = tx.execute("DELETE FROM mark_sets WHERE class_id = ?", [&class_id]) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "mark_sets" })),
        );
    }

    if let Err(e) = tx.execute("DELETE FROM student_notes WHERE class_id = ?", [&class_id]) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "student_notes" })),
        );
    }

    if let Err(e) = tx.execute(
        "DELETE FROM learning_skills_cells WHERE class_id = ?",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "learning_skills_cells" })),
        );
    }

    if let Err(e) = tx.execute("DELETE FROM students WHERE class_id = ?", [&class_id]) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "students" })),
        );
    }

    if let Err(e) = tx.execute("DELETE FROM classes WHERE id = ?", [&class_id]) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "classes" })),
        );
    }

    if let Err(e) = tx.commit() {
        return err(&req.id, "db_commit_failed", e.to_string(), None);
    }

    ok(&req.id, json!({ "ok": true }))
}

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "classes.list" => Some(handle_classes_list(state, req)),
        "classes.create" => Some(handle_classes_create(state, req)),
        "classes.delete" => Some(handle_classes_delete(state, req)),
        _ => None,
    }
}
