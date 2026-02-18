use crate::ipc::error::{err, ok};
use crate::ipc::types::{AppState, Request};
use rusqlite::types::Value;
use rusqlite::{params_from_iter, OptionalExtension};
use serde_json::json;
use std::collections::HashSet;
use uuid::Uuid;

fn handle_students_list(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };

    let mut stmt = match conn.prepare(
        "SELECT id, last_name, first_name, student_no, birth_date, active, sort_order
         FROM students
         WHERE class_id = ?
         ORDER BY sort_order",
    ) {
        Ok(s) => s,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    let rows = stmt
        .query_map([&class_id], |row| {
            let id: String = row.get(0)?;
            let last_name: String = row.get(1)?;
            let first_name: String = row.get(2)?;
            let student_no: Option<String> = row.get(3)?;
            let birth_date: Option<String> = row.get(4)?;
            let active: i64 = row.get(5)?;
            let sort_order: i64 = row.get(6)?;

            let display_name = format!("{}, {}", last_name, first_name);
            let student_no = student_no.and_then(|s| {
                let t = s.trim().to_string();
                if t.is_empty() {
                    None
                } else {
                    Some(t)
                }
            });
            let birth_date = birth_date.and_then(|s| {
                let t = s.trim().to_string();
                if t.is_empty() {
                    None
                } else {
                    Some(t)
                }
            });

            Ok(json!({
                "id": id,
                "lastName": last_name,
                "firstName": first_name,
                "displayName": display_name,
                "studentNo": student_no,
                "birthDate": birth_date,
                "active": active != 0,
                "sortOrder": sort_order
            }))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>());

    match rows {
        Ok(students) => ok(&req.id, json!({ "students": students })),
        Err(e) => err(&req.id, "db_query_failed", e.to_string(), None),
    }
}

fn handle_students_create(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };

    let last_name = match req.params.get("lastName").and_then(|v| v.as_str()) {
        Some(v) => v.trim().to_string(),
        None => return err(&req.id, "bad_params", "missing lastName", None),
    };
    let first_name = match req.params.get("firstName").and_then(|v| v.as_str()) {
        Some(v) => v.trim().to_string(),
        None => return err(&req.id, "bad_params", "missing firstName", None),
    };
    if last_name.is_empty() || first_name.is_empty() {
        return err(
            &req.id,
            "bad_params",
            "firstName/lastName must not be empty",
            None,
        );
    }

    let student_no = req
        .params
        .get("studentNo")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .and_then(|s| if s.is_empty() { None } else { Some(s) });
    let birth_date = req
        .params
        .get("birthDate")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .and_then(|s| if s.is_empty() { None } else { Some(s) });
    let active = req
        .params
        .get("active")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let active_i = if active { 1 } else { 0 };

    let class_exists: Option<i64> = match conn
        .query_row("SELECT 1 FROM classes WHERE id = ?", [&class_id], |r| {
            r.get(0)
        })
        .optional()
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    if class_exists.is_none() {
        return err(&req.id, "not_found", "class not found", None);
    }

    let sort_order: i64 = match conn.query_row(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM students WHERE class_id = ?",
        [&class_id],
        |r| r.get(0),
    ) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    let student_id = Uuid::new_v4().to_string();
    if let Err(e) = conn.execute(
        "INSERT INTO students(
           id,
           class_id,
           last_name,
           first_name,
           student_no,
           birth_date,
           active,
           sort_order,
           raw_line,
           mark_set_mask,
           updated_at
         ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%SZ','now'))",
        (
            &student_id,
            &class_id,
            &last_name,
            &first_name,
            student_no.as_deref(),
            birth_date.as_deref(),
            active_i,
            sort_order,
            "",
            "TBA",
        ),
    ) {
        return err(
            &req.id,
            "db_insert_failed",
            e.to_string(),
            Some(json!({ "table": "students" })),
        );
    }

    ok(&req.id, json!({ "studentId": student_id }))
}

fn handle_students_update(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };
    let student_id = match req.params.get("studentId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing studentId", None),
    };

    let Some(patch) = req.params.get("patch").and_then(|v| v.as_object()) else {
        return err(&req.id, "bad_params", "missing/invalid patch", None);
    };

    let mut set_parts: Vec<String> = Vec::new();
    let mut bind_values: Vec<Value> = Vec::new();

    if let Some(v) = patch.get("lastName") {
        let Some(s) = v.as_str() else {
            return err(
                &req.id,
                "bad_params",
                "patch.lastName must be a string",
                None,
            );
        };
        let s = s.trim().to_string();
        if s.is_empty() {
            return err(&req.id, "bad_params", "lastName must not be empty", None);
        }
        set_parts.push("last_name = ?".into());
        bind_values.push(Value::Text(s));
    }

    if let Some(v) = patch.get("firstName") {
        let Some(s) = v.as_str() else {
            return err(
                &req.id,
                "bad_params",
                "patch.firstName must be a string",
                None,
            );
        };
        let s = s.trim().to_string();
        if s.is_empty() {
            return err(&req.id, "bad_params", "firstName must not be empty", None);
        }
        set_parts.push("first_name = ?".into());
        bind_values.push(Value::Text(s));
    }

    if let Some(v) = patch.get("studentNo") {
        if v.is_null() {
            set_parts.push("student_no = ?".into());
            bind_values.push(Value::Null);
        } else if let Some(s) = v.as_str() {
            let t = s.trim().to_string();
            set_parts.push("student_no = ?".into());
            if t.is_empty() {
                bind_values.push(Value::Null);
            } else {
                bind_values.push(Value::Text(t));
            }
        } else {
            return err(
                &req.id,
                "bad_params",
                "patch.studentNo must be a string or null",
                None,
            );
        }
    }

    if let Some(v) = patch.get("birthDate") {
        if v.is_null() {
            set_parts.push("birth_date = ?".into());
            bind_values.push(Value::Null);
        } else if let Some(s) = v.as_str() {
            let t = s.trim().to_string();
            set_parts.push("birth_date = ?".into());
            if t.is_empty() {
                bind_values.push(Value::Null);
            } else {
                bind_values.push(Value::Text(t));
            }
        } else {
            return err(
                &req.id,
                "bad_params",
                "patch.birthDate must be a string or null",
                None,
            );
        }
    }

    if let Some(v) = patch.get("active") {
        let Some(b) = v.as_bool() else {
            return err(
                &req.id,
                "bad_params",
                "patch.active must be a boolean",
                None,
            );
        };
        set_parts.push("active = ?".into());
        bind_values.push(Value::Integer(if b { 1 } else { 0 }));
    }

    if set_parts.is_empty() {
        return err(
            &req.id,
            "bad_params",
            "patch must include at least one field",
            None,
        );
    }

    set_parts.push("updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')".into());

    let sql = format!(
        "UPDATE students SET {} WHERE id = ? AND class_id = ?",
        set_parts.join(", ")
    );
    bind_values.push(Value::Text(student_id.clone()));
    bind_values.push(Value::Text(class_id.clone()));

    let changed = match conn.execute(&sql, params_from_iter(bind_values)) {
        Ok(v) => v,
        Err(e) => {
            return err(
                &req.id,
                "db_update_failed",
                e.to_string(),
                Some(json!({ "table": "students" })),
            )
        }
    };

    if changed == 0 {
        return err(&req.id, "not_found", "student not found", None);
    }

    ok(&req.id, json!({ "ok": true }))
}

fn handle_students_reorder(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };

    let Some(arr) = req
        .params
        .get("orderedStudentIds")
        .and_then(|v| v.as_array())
    else {
        return err(
            &req.id,
            "bad_params",
            "missing/invalid orderedStudentIds",
            None,
        );
    };

    let mut ordered: Vec<String> = Vec::with_capacity(arr.len());
    for v in arr {
        let Some(s) = v.as_str() else {
            return err(
                &req.id,
                "bad_params",
                "orderedStudentIds must be strings",
                None,
            );
        };
        ordered.push(s.to_string());
    }

    let mut stmt =
        match conn.prepare("SELECT id FROM students WHERE class_id = ? ORDER BY sort_order") {
            Ok(s) => s,
            Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
        };

    let current_ids: Vec<String> = match stmt
        .query_map([&class_id], |row| row.get::<_, String>(0))
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    if ordered.len() != current_ids.len() {
        return err(
            &req.id,
            "bad_params",
            "orderedStudentIds must be a permutation of the class students",
            Some(json!({ "expected": current_ids.len(), "got": ordered.len() })),
        );
    }

    let current_set: HashSet<String> = current_ids.into_iter().collect();
    let mut seen: HashSet<String> = HashSet::new();
    for id in &ordered {
        if !seen.insert(id.clone()) {
            return err(
                &req.id,
                "bad_params",
                "orderedStudentIds contains duplicates",
                Some(json!({ "studentId": id })),
            );
        }
        if !current_set.contains(id) {
            return err(
                &req.id,
                "bad_params",
                "orderedStudentIds contains unknown studentId",
                Some(json!({ "studentId": id })),
            );
        }
    }

    if seen.len() != current_set.len() {
        return err(
            &req.id,
            "bad_params",
            "orderedStudentIds must include every student exactly once",
            None,
        );
    }

    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return err(&req.id, "db_tx_failed", e.to_string(), None),
    };

    for (i, sid) in ordered.iter().enumerate() {
        if let Err(e) = tx.execute(
            "UPDATE students
             SET sort_order = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
             WHERE id = ? AND class_id = ?",
            (i as i64, sid, &class_id),
        ) {
            let _ = tx.rollback();
            return err(
                &req.id,
                "db_update_failed",
                e.to_string(),
                Some(json!({ "table": "students" })),
            );
        }
    }

    if let Err(e) = tx.commit() {
        return err(&req.id, "db_commit_failed", e.to_string(), None);
    }

    ok(&req.id, json!({ "ok": true }))
}

fn handle_students_delete(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };
    let student_id = match req.params.get("studentId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing studentId", None),
    };

    let sort_order: Option<i64> = match conn
        .query_row(
            "SELECT sort_order FROM students WHERE id = ? AND class_id = ?",
            (&student_id, &class_id),
            |r| r.get(0),
        )
        .optional()
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let Some(sort_order) = sort_order else {
        return err(&req.id, "not_found", "student not found", None);
    };

    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return err(&req.id, "db_tx_failed", e.to_string(), None),
    };

    if let Err(e) = tx.execute("DELETE FROM scores WHERE student_id = ?", [&student_id]) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "scores" })),
        );
    }

    if let Err(e) = tx.execute(
        "DELETE FROM student_notes WHERE class_id = ? AND student_id = ?",
        (&class_id, &student_id),
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "student_notes" })),
        );
    }

    if let Err(e) = tx.execute(
        "DELETE FROM attendance_student_months WHERE class_id = ? AND student_id = ?",
        (&class_id, &student_id),
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
        "DELETE FROM seating_assignments WHERE class_id = ? AND student_id = ?",
        (&class_id, &student_id),
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "seating_assignments" })),
        );
    }

    if let Err(e) = tx.execute(
        "DELETE FROM comment_set_remarks WHERE student_id = ?",
        (&student_id,),
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "comment_set_remarks" })),
        );
    }

    let changed = match tx.execute(
        "DELETE FROM students WHERE id = ? AND class_id = ?",
        (&student_id, &class_id),
    ) {
        Ok(v) => v,
        Err(e) => {
            let _ = tx.rollback();
            return err(
                &req.id,
                "db_delete_failed",
                e.to_string(),
                Some(json!({ "table": "students" })),
            );
        }
    };
    if changed == 0 {
        let _ = tx.rollback();
        return err(&req.id, "not_found", "student not found", None);
    }

    // Keep sort_order contiguous so grid row indices remain stable.
    if let Err(e) = tx.execute(
        "UPDATE students
         SET sort_order = sort_order - 1,
             updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
         WHERE class_id = ? AND sort_order > ?",
        (&class_id, sort_order),
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_update_failed",
            e.to_string(),
            Some(json!({ "table": "students" })),
        );
    }

    if let Err(e) = tx.commit() {
        return err(&req.id, "db_commit_failed", e.to_string(), None);
    }

    ok(&req.id, json!({ "ok": true }))
}

fn normalize_mark_set_mask(raw: Option<String>, mark_set_count: usize) -> String {
    if mark_set_count == 0 {
        return "".to_string();
    }
    let s = raw.unwrap_or_else(|| "TBA".to_string());
    let t = s.trim();
    if t.is_empty() {
        return "1".repeat(mark_set_count);
    }
    if t.eq_ignore_ascii_case("TBA") {
        return "1".repeat(mark_set_count);
    }
    let up = t.to_ascii_uppercase();
    if !up.chars().all(|ch| ch == '0' || ch == '1') {
        // Fail-open to match legacy "unknown string" safety.
        return "1".repeat(mark_set_count);
    }
    if up.len() >= mark_set_count {
        return up[..mark_set_count].to_string();
    }
    let mut out = up;
    out.push_str(&"1".repeat(mark_set_count - out.len()));
    out
}

fn handle_students_membership_get(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };

    let class_exists: Option<i64> = match conn
        .query_row("SELECT 1 FROM classes WHERE id = ?", [&class_id], |r| r.get(0))
        .optional()
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    if class_exists.is_none() {
        return err(&req.id, "not_found", "class not found", None);
    }

    let mut ms_stmt = match conn.prepare(
        "SELECT id, code, sort_order
         FROM mark_sets
         WHERE class_id = ?
         ORDER BY sort_order",
    ) {
        Ok(s) => s,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let mark_sets = match ms_stmt
        .query_map([&class_id], |r| {
            let id: String = r.get(0)?;
            let code: String = r.get(1)?;
            let sort_order: i64 = r.get(2)?;
            Ok(json!({ "id": id, "code": code, "sortOrder": sort_order }))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    let mark_set_count = mark_sets.len();

    let mut st_stmt = match conn.prepare(
        "SELECT id, last_name, first_name, active, sort_order, mark_set_mask
         FROM students
         WHERE class_id = ?
         ORDER BY sort_order",
    ) {
        Ok(s) => s,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let students = match st_stmt
        .query_map([&class_id], |r| {
            let id: String = r.get(0)?;
            let last: String = r.get(1)?;
            let first: String = r.get(2)?;
            let active: i64 = r.get(3)?;
            let sort_order: i64 = r.get(4)?;
            let mask: Option<String> = r.get(5)?;
            let display_name = format!("{}, {}", last, first);
            let norm = normalize_mark_set_mask(mask, mark_set_count);
            Ok(json!({
                "id": id,
                "displayName": display_name,
                "active": active != 0,
                "sortOrder": sort_order,
                "mask": norm
            }))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    ok(
        &req.id,
        json!({
            "markSets": mark_sets,
            "students": students
        }),
    )
}

fn handle_students_membership_set(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };
    let student_id = match req.params.get("studentId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing studentId", None),
    };
    let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing markSetId", None),
    };
    let enabled = match req.params.get("enabled").and_then(|v| v.as_bool()) {
        Some(v) => v,
        None => return err(&req.id, "bad_params", "missing enabled", None),
    };

    let (mark_set_sort_order, mark_set_count): (i64, i64) = match conn.query_row(
        "SELECT sort_order, (SELECT COUNT(*) FROM mark_sets WHERE class_id = ?)
         FROM mark_sets
         WHERE id = ? AND class_id = ?",
        (&class_id, &mark_set_id, &class_id),
        |r| Ok((r.get(0)?, r.get(1)?)),
    ) {
        Ok(v) => v,
        Err(_) => return err(&req.id, "not_found", "mark set not found", None),
    };

    let raw_mask: Option<String> = match conn
        .query_row(
            "SELECT mark_set_mask FROM students WHERE id = ? AND class_id = ?",
            (&student_id, &class_id),
            |r| r.get(0),
        )
        .optional()
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    if raw_mask.is_none() {
        return err(&req.id, "not_found", "student not found", None);
    }

    let Ok(ms_count) = usize::try_from(mark_set_count) else {
        return err(&req.id, "db_query_failed", "invalid mark set count", None);
    };
    let Ok(bit_idx) = usize::try_from(mark_set_sort_order) else {
        return err(&req.id, "db_query_failed", "invalid mark set sort order", None);
    };

    let mut norm = normalize_mark_set_mask(raw_mask, ms_count).into_bytes();
    if bit_idx < norm.len() {
        norm[bit_idx] = if enabled { b'1' } else { b'0' };
    }
    let new_mask = String::from_utf8_lossy(&norm).to_string();

    if let Err(e) = conn.execute(
        "UPDATE students
         SET mark_set_mask = ?,
             updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
         WHERE id = ? AND class_id = ?",
        (&new_mask, &student_id, &class_id),
    ) {
        return err(
            &req.id,
            "db_update_failed",
            e.to_string(),
            Some(json!({ "table": "students" })),
        );
    }

    ok(&req.id, json!({ "ok": true, "mask": new_mask }))
}

fn handle_notes_get(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };

    let class_exists: Option<i64> = match conn
        .query_row("SELECT 1 FROM classes WHERE id = ?", [&class_id], |r| {
            r.get(0)
        })
        .optional()
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    if class_exists.is_none() {
        return err(&req.id, "not_found", "class not found", None);
    }

    let mut stmt =
        match conn.prepare("SELECT student_id, note FROM student_notes WHERE class_id = ?") {
            Ok(s) => s,
            Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
        };

    let rows = stmt
        .query_map([&class_id], |row| {
            let student_id: String = row.get(0)?;
            let note: String = row.get(1)?;
            Ok(json!({ "studentId": student_id, "note": note }))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>());

    match rows {
        Ok(notes) => ok(&req.id, json!({ "notes": notes })),
        Err(e) => err(&req.id, "db_query_failed", e.to_string(), None),
    }
}

fn handle_notes_update(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };
    let student_id = match req.params.get("studentId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing studentId", None),
    };
    let note = match req.params.get("note").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing note", None),
    };

    let student_exists: Option<i64> = match conn
        .query_row(
            "SELECT 1 FROM students WHERE id = ? AND class_id = ?",
            (&student_id, &class_id),
            |r| r.get(0),
        )
        .optional()
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    if student_exists.is_none() {
        return err(&req.id, "not_found", "student not found", None);
    }

    let trimmed = note.trim().to_string();
    if trimmed.is_empty() {
        if let Err(e) = conn.execute(
            "DELETE FROM student_notes WHERE class_id = ? AND student_id = ?",
            (&class_id, &student_id),
        ) {
            return err(
                &req.id,
                "db_delete_failed",
                e.to_string(),
                Some(json!({ "table": "student_notes" })),
            );
        }
        return ok(&req.id, json!({ "ok": true }));
    }

    let note_id = Uuid::new_v4().to_string();
    if let Err(e) = conn.execute(
        "INSERT INTO student_notes(id, class_id, student_id, note)
         VALUES(?, ?, ?, ?)
         ON CONFLICT(class_id, student_id) DO UPDATE SET
           note = excluded.note",
        (&note_id, &class_id, &student_id, &note),
    ) {
        return err(
            &req.id,
            "db_insert_failed",
            e.to_string(),
            Some(json!({ "table": "student_notes" })),
        );
    }

    ok(&req.id, json!({ "ok": true }))
}

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "students.list" => Some(handle_students_list(state, req)),
        "students.create" => Some(handle_students_create(state, req)),
        "students.update" => Some(handle_students_update(state, req)),
        "students.reorder" => Some(handle_students_reorder(state, req)),
        "students.delete" => Some(handle_students_delete(state, req)),
        "students.membership.get" => Some(handle_students_membership_get(state, req)),
        "students.membership.set" => Some(handle_students_membership_set(state, req)),
        "notes.get" => Some(handle_notes_get(state, req)),
        "notes.update" => Some(handle_notes_update(state, req)),
        _ => None,
    }
}
