use crate::ipc::error::{err, ok};
use crate::ipc::types::{AppState, Request};
use rusqlite::types::Value;
use rusqlite::{params_from_iter, Connection, OptionalExtension};
use serde_json::json;
use std::collections::HashSet;
use uuid::Uuid;

struct HandlerErr {
    code: &'static str,
    message: String,
    details: Option<serde_json::Value>,
}

impl HandlerErr {
    fn response(self, id: &str) -> serde_json::Value {
        err(id, self.code, self.message, self.details)
    }
}

fn mark_set_exists(conn: &Connection, class_id: &str, mark_set_id: &str) -> Result<bool, HandlerErr> {
    conn.query_row(
        "SELECT 1 FROM mark_sets WHERE id = ? AND class_id = ?",
        (mark_set_id, class_id),
        |r| r.get::<_, i64>(0),
    )
    .optional()
    .map(|v| v.is_some())
    .map_err(|e| HandlerErr {
        code: "db_query_failed",
        message: e.to_string(),
        details: None,
    })
}

fn handle_categories_list(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };
    let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing markSetId", None),
    };

    match mark_set_exists(conn, &class_id, &mark_set_id) {
        Ok(true) => {}
        Ok(false) => return err(&req.id, "not_found", "mark set not found", None),
        Err(e) => return e.response(&req.id),
    }

    let mut stmt = match conn.prepare(
        "SELECT id, name, weight, sort_order FROM categories WHERE mark_set_id = ? ORDER BY sort_order",
    ) {
        Ok(s) => s,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let rows = stmt
        .query_map([&mark_set_id], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let weight: Option<f64> = row.get(2)?;
            let sort_order: i64 = row.get(3)?;
            Ok(json!({
                "id": id,
                "name": name,
                "weight": weight,
                "sortOrder": sort_order
            }))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>());

    match rows {
        Ok(categories) => ok(&req.id, json!({ "categories": categories })),
        Err(e) => err(&req.id, "db_query_failed", e.to_string(), None),
    }
}

fn handle_categories_create(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };
    let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing markSetId", None),
    };
    let name = match req.params.get("name").and_then(|v| v.as_str()) {
        Some(v) => v.trim().to_string(),
        None => return err(&req.id, "bad_params", "missing name", None),
    };
    if name.is_empty() {
        return err(&req.id, "bad_params", "name must not be empty", None);
    }
    let weight = req.params.get("weight").and_then(|v| v.as_f64());

    match mark_set_exists(conn, &class_id, &mark_set_id) {
        Ok(true) => {}
        Ok(false) => return err(&req.id, "not_found", "mark set not found", None),
        Err(e) => return e.response(&req.id),
    }

    let sort_order: i64 = match conn.query_row(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM categories WHERE mark_set_id = ?",
        [&mark_set_id],
        |r| r.get(0),
    ) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    let category_id = Uuid::new_v4().to_string();
    if let Err(e) = conn.execute(
        "INSERT INTO categories(id, mark_set_id, name, weight, sort_order) VALUES(?, ?, ?, ?, ?)",
        (&category_id, &mark_set_id, &name, weight, sort_order),
    ) {
        return err(
            &req.id,
            "db_insert_failed",
            e.to_string(),
            Some(json!({ "table": "categories" })),
        );
    }

    ok(&req.id, json!({ "categoryId": category_id }))
}

fn handle_categories_update(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };
    let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing markSetId", None),
    };
    let category_id = match req.params.get("categoryId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing categoryId", None),
    };
    let Some(patch) = req.params.get("patch").and_then(|v| v.as_object()) else {
        return err(&req.id, "bad_params", "missing/invalid patch", None);
    };

    match mark_set_exists(conn, &class_id, &mark_set_id) {
        Ok(true) => {}
        Ok(false) => return err(&req.id, "not_found", "mark set not found", None),
        Err(e) => return e.response(&req.id),
    }

    let mut set_parts: Vec<String> = Vec::new();
    let mut bind_values: Vec<Value> = Vec::new();

    if let Some(v) = patch.get("name") {
        let Some(s) = v.as_str() else {
            return err(&req.id, "bad_params", "patch.name must be a string", None);
        };
        let s = s.trim().to_string();
        if s.is_empty() {
            return err(&req.id, "bad_params", "name must not be empty", None);
        }
        set_parts.push("name = ?".into());
        bind_values.push(Value::Text(s));
    }
    if let Some(v) = patch.get("weight") {
        if v.is_null() {
            set_parts.push("weight = ?".into());
            bind_values.push(Value::Null);
        } else if let Some(n) = v.as_f64() {
            set_parts.push("weight = ?".into());
            bind_values.push(Value::Real(n));
        } else {
            return err(&req.id, "bad_params", "patch.weight must be a number or null", None);
        }
    }

    if set_parts.is_empty() {
        return err(
            &req.id,
            "bad_params",
            "patch must include at least one field",
            None,
        );
    }

    let sql = format!(
        "UPDATE categories SET {} WHERE id = ? AND mark_set_id = ?",
        set_parts.join(", ")
    );
    bind_values.push(Value::Text(category_id.clone()));
    bind_values.push(Value::Text(mark_set_id.clone()));

    let changed = match conn.execute(&sql, params_from_iter(bind_values)) {
        Ok(v) => v,
        Err(e) => {
            return err(
                &req.id,
                "db_update_failed",
                e.to_string(),
                Some(json!({ "table": "categories" })),
            )
        }
    };
    if changed == 0 {
        return err(&req.id, "not_found", "category not found", None);
    }

    ok(&req.id, json!({ "ok": true }))
}

fn handle_categories_delete(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };
    let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing markSetId", None),
    };
    let category_id = match req.params.get("categoryId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing categoryId", None),
    };

    match mark_set_exists(conn, &class_id, &mark_set_id) {
        Ok(true) => {}
        Ok(false) => return err(&req.id, "not_found", "mark set not found", None),
        Err(e) => return e.response(&req.id),
    }

    let sort_order: Option<i64> = match conn
        .query_row(
            "SELECT sort_order FROM categories WHERE id = ? AND mark_set_id = ?",
            (&category_id, &mark_set_id),
            |r| r.get(0),
        )
        .optional()
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let Some(sort_order) = sort_order else {
        return err(&req.id, "not_found", "category not found", None);
    };

    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return err(&req.id, "db_tx_failed", e.to_string(), None),
    };

    let changed = match tx.execute(
        "DELETE FROM categories WHERE id = ? AND mark_set_id = ?",
        (&category_id, &mark_set_id),
    ) {
        Ok(v) => v,
        Err(e) => {
            let _ = tx.rollback();
            return err(
                &req.id,
                "db_delete_failed",
                e.to_string(),
                Some(json!({ "table": "categories" })),
            );
        }
    };
    if changed == 0 {
        let _ = tx.rollback();
        return err(&req.id, "not_found", "category not found", None);
    }

    // Keep sort_order contiguous.
    if let Err(e) = tx.execute(
        "UPDATE categories
         SET sort_order = sort_order - 1
         WHERE mark_set_id = ? AND sort_order > ?",
        (&mark_set_id, sort_order),
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_update_failed",
            e.to_string(),
            Some(json!({ "table": "categories" })),
        );
    }

    if let Err(e) = tx.commit() {
        return err(&req.id, "db_commit_failed", e.to_string(), None);
    }

    ok(&req.id, json!({ "ok": true }))
}

fn handle_assessments_list(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };
    let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing markSetId", None),
    };

    match mark_set_exists(conn, &class_id, &mark_set_id) {
        Ok(true) => {}
        Ok(false) => return err(&req.id, "not_found", "mark set not found", None),
        Err(e) => return e.response(&req.id),
    }

    let mut stmt = match conn.prepare(
        "SELECT id, idx, date, category_name, title, term, legacy_type, weight, out_of
         FROM assessments
         WHERE mark_set_id = ?
         ORDER BY idx",
    ) {
        Ok(s) => s,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let rows = stmt
        .query_map([&mark_set_id], |row| {
            let id: String = row.get(0)?;
            let idx: i64 = row.get(1)?;
            let date: Option<String> = row.get(2)?;
            let category_name: Option<String> = row.get(3)?;
            let title: String = row.get(4)?;
            let term: Option<i64> = row.get(5)?;
            let legacy_type: Option<i64> = row.get(6)?;
            let weight: Option<f64> = row.get(7)?;
            let out_of: Option<f64> = row.get(8)?;
            Ok(json!({
                "id": id,
                "idx": idx,
                "date": date,
                "categoryName": category_name,
                "title": title,
                "term": term,
                "legacyType": legacy_type,
                "weight": weight,
                "outOf": out_of
            }))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>());

    match rows {
        Ok(assessments) => ok(&req.id, json!({ "assessments": assessments })),
        Err(e) => err(&req.id, "db_query_failed", e.to_string(), None),
    }
}

fn handle_assessments_create(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };
    let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing markSetId", None),
    };
    let title = match req.params.get("title").and_then(|v| v.as_str()) {
        Some(v) => v.trim().to_string(),
        None => return err(&req.id, "bad_params", "missing title", None),
    };
    if title.is_empty() {
        return err(&req.id, "bad_params", "title must not be empty", None);
    }

    let idx_req = req.params.get("idx").and_then(|v| v.as_i64());
    let date = req
        .params
        .get("date")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .and_then(|s| if s.is_empty() { None } else { Some(s) });
    let category_name = req
        .params
        .get("categoryName")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .and_then(|s| if s.is_empty() { None } else { Some(s) });
    let term = req.params.get("term").and_then(|v| v.as_i64());
    let legacy_type = req.params.get("legacyType").and_then(|v| v.as_i64());
    let weight = req.params.get("weight").and_then(|v| v.as_f64());
    let out_of = req.params.get("outOf").and_then(|v| v.as_f64());

    match mark_set_exists(conn, &class_id, &mark_set_id) {
        Ok(true) => {}
        Ok(false) => return err(&req.id, "not_found", "mark set not found", None),
        Err(e) => return e.response(&req.id),
    }

    let append_idx: i64 = match conn.query_row(
        "SELECT COALESCE(MAX(idx), -1) + 1 FROM assessments WHERE mark_set_id = ?",
        [&mark_set_id],
        |r| r.get(0),
    ) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let idx = match idx_req {
        Some(v) if v >= 0 && v <= append_idx => v,
        Some(_) => {
            return err(
                &req.id,
                "bad_params",
                "idx out of range",
                Some(json!({ "max": append_idx })),
            )
        }
        None => append_idx,
    };

    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return err(&req.id, "db_tx_failed", e.to_string(), None),
    };

    // If inserting into the middle, shift existing idx values up by 1 (descending).
    if idx < append_idx {
        let mut stmt = match tx.prepare(
            "SELECT id, idx FROM assessments WHERE mark_set_id = ? AND idx >= ? ORDER BY idx DESC",
        ) {
            Ok(s) => s,
            Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
        };
        let rows: Vec<(String, i64)> = match stmt
            .query_map((&mark_set_id, idx), |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        {
            Ok(v) => v,
            Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
        };
        let mut up = match tx.prepare("UPDATE assessments SET idx = ? WHERE id = ?") {
            Ok(s) => s,
            Err(e) => {
                return err(
                    &req.id,
                    "db_update_failed",
                    e.to_string(),
                    Some(json!({ "table": "assessments" })),
                )
            }
        };
        for (aid, cur_idx) in rows {
            if let Err(e) = up.execute((cur_idx + 1, &aid)) {
                return err(
                    &req.id,
                    "db_update_failed",
                    e.to_string(),
                    Some(json!({ "table": "assessments" })),
                );
            }
        }
    }

    let assessment_id = Uuid::new_v4().to_string();
    if let Err(e) = tx.execute(
        "INSERT INTO assessments(
           id,
           mark_set_id,
           idx,
           date,
           category_name,
           title,
           term,
           legacy_type,
           weight,
           out_of
         ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        (
            &assessment_id,
            &mark_set_id,
            idx,
            date.as_deref(),
            category_name.as_deref(),
            &title,
            term,
            legacy_type,
            weight,
            out_of,
        ),
    ) {
        return err(
            &req.id,
            "db_insert_failed",
            e.to_string(),
            Some(json!({ "table": "assessments" })),
        );
    }

    if let Err(e) = tx.commit() {
        return err(&req.id, "db_commit_failed", e.to_string(), None);
    }

    ok(&req.id, json!({ "assessmentId": assessment_id }))
}

fn handle_assessments_update(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };
    let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing markSetId", None),
    };
    let assessment_id = match req.params.get("assessmentId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing assessmentId", None),
    };
    let Some(patch) = req.params.get("patch").and_then(|v| v.as_object()) else {
        return err(&req.id, "bad_params", "missing/invalid patch", None);
    };

    match mark_set_exists(conn, &class_id, &mark_set_id) {
        Ok(true) => {}
        Ok(false) => return err(&req.id, "not_found", "mark set not found", None),
        Err(e) => return e.response(&req.id),
    }

    let mut set_parts: Vec<String> = Vec::new();
    let mut bind_values: Vec<Value> = Vec::new();

    if let Some(v) = patch.get("date") {
        if v.is_null() {
            set_parts.push("date = ?".into());
            bind_values.push(Value::Null);
        } else if let Some(s) = v.as_str() {
            let t = s.trim().to_string();
            set_parts.push("date = ?".into());
            if t.is_empty() {
                bind_values.push(Value::Null);
            } else {
                bind_values.push(Value::Text(t));
            }
        } else {
            return err(&req.id, "bad_params", "patch.date must be a string or null", None);
        }
    }
    if let Some(v) = patch.get("categoryName") {
        if v.is_null() {
            set_parts.push("category_name = ?".into());
            bind_values.push(Value::Null);
        } else if let Some(s) = v.as_str() {
            let t = s.trim().to_string();
            set_parts.push("category_name = ?".into());
            if t.is_empty() {
                bind_values.push(Value::Null);
            } else {
                bind_values.push(Value::Text(t));
            }
        } else {
            return err(
                &req.id,
                "bad_params",
                "patch.categoryName must be a string or null",
                None,
            );
        }
    }
    if let Some(v) = patch.get("title") {
        let Some(s) = v.as_str() else {
            return err(&req.id, "bad_params", "patch.title must be a string", None);
        };
        let t = s.trim().to_string();
        if t.is_empty() {
            return err(&req.id, "bad_params", "title must not be empty", None);
        }
        set_parts.push("title = ?".into());
        bind_values.push(Value::Text(t));
    }
    if let Some(v) = patch.get("term") {
        if v.is_null() {
            set_parts.push("term = ?".into());
            bind_values.push(Value::Null);
        } else if let Some(n) = v.as_i64() {
            set_parts.push("term = ?".into());
            bind_values.push(Value::Integer(n));
        } else {
            return err(
                &req.id,
                "bad_params",
                "patch.term must be an integer or null",
                None,
            );
        }
    }
    if let Some(v) = patch.get("legacyType") {
        if v.is_null() {
            set_parts.push("legacy_type = ?".into());
            bind_values.push(Value::Null);
        } else if let Some(n) = v.as_i64() {
            set_parts.push("legacy_type = ?".into());
            bind_values.push(Value::Integer(n));
        } else {
            return err(
                &req.id,
                "bad_params",
                "patch.legacyType must be an integer or null",
                None,
            );
        }
    }
    if let Some(v) = patch.get("weight") {
        if v.is_null() {
            set_parts.push("weight = ?".into());
            bind_values.push(Value::Null);
        } else if let Some(n) = v.as_f64() {
            set_parts.push("weight = ?".into());
            bind_values.push(Value::Real(n));
        } else {
            return err(
                &req.id,
                "bad_params",
                "patch.weight must be a number or null",
                None,
            );
        }
    }
    if let Some(v) = patch.get("outOf") {
        if v.is_null() {
            set_parts.push("out_of = ?".into());
            bind_values.push(Value::Null);
        } else if let Some(n) = v.as_f64() {
            set_parts.push("out_of = ?".into());
            bind_values.push(Value::Real(n));
        } else {
            return err(
                &req.id,
                "bad_params",
                "patch.outOf must be a number or null",
                None,
            );
        }
    }

    if set_parts.is_empty() {
        return err(
            &req.id,
            "bad_params",
            "patch must include at least one field",
            None,
        );
    }

    let sql = format!(
        "UPDATE assessments SET {} WHERE id = ? AND mark_set_id = ?",
        set_parts.join(", ")
    );
    bind_values.push(Value::Text(assessment_id.clone()));
    bind_values.push(Value::Text(mark_set_id.clone()));

    let changed = match conn.execute(&sql, params_from_iter(bind_values)) {
        Ok(v) => v,
        Err(e) => {
            return err(
                &req.id,
                "db_update_failed",
                e.to_string(),
                Some(json!({ "table": "assessments" })),
            )
        }
    };
    if changed == 0 {
        return err(&req.id, "not_found", "assessment not found", None);
    }

    ok(&req.id, json!({ "ok": true }))
}

fn handle_assessments_delete(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };
    let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing markSetId", None),
    };
    let assessment_id = match req.params.get("assessmentId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing assessmentId", None),
    };

    match mark_set_exists(conn, &class_id, &mark_set_id) {
        Ok(true) => {}
        Ok(false) => return err(&req.id, "not_found", "mark set not found", None),
        Err(e) => return e.response(&req.id),
    }

    let idx: Option<i64> = match conn
        .query_row(
            "SELECT idx FROM assessments WHERE id = ? AND mark_set_id = ?",
            (&assessment_id, &mark_set_id),
            |r| r.get(0),
        )
        .optional()
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let Some(idx) = idx else {
        return err(&req.id, "not_found", "assessment not found", None);
    };

    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return err(&req.id, "db_tx_failed", e.to_string(), None),
    };

    if let Err(e) = tx.execute("DELETE FROM scores WHERE assessment_id = ?", [&assessment_id]) {
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "scores" })),
        );
    }

    let changed = match tx.execute(
        "DELETE FROM assessments WHERE id = ? AND mark_set_id = ?",
        (&assessment_id, &mark_set_id),
    ) {
        Ok(v) => v,
        Err(e) => {
            return err(
                &req.id,
                "db_delete_failed",
                e.to_string(),
                Some(json!({ "table": "assessments" })),
            );
        }
    };
    if changed == 0 {
        return err(&req.id, "not_found", "assessment not found", None);
    }

    // Shift down higher idx values (ascending).
    let mut stmt = match tx.prepare(
        "SELECT id, idx FROM assessments WHERE mark_set_id = ? AND idx > ? ORDER BY idx ASC",
    ) {
        Ok(s) => s,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let rows: Vec<(String, i64)> = match stmt
        .query_map((&mark_set_id, idx), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    drop(stmt);
    let mut up = match tx.prepare("UPDATE assessments SET idx = ? WHERE id = ?") {
        Ok(s) => s,
        Err(e) => {
            return err(
                &req.id,
                "db_update_failed",
                e.to_string(),
                Some(json!({ "table": "assessments" })),
            )
        }
    };
    for (aid, cur_idx) in rows {
        if let Err(e) = up.execute((cur_idx - 1, &aid)) {
            return err(
                &req.id,
                "db_update_failed",
                e.to_string(),
                Some(json!({ "table": "assessments" })),
            );
        }
    }
    drop(up);

    if let Err(e) = tx.commit() {
        return err(&req.id, "db_commit_failed", e.to_string(), None);
    }

    ok(&req.id, json!({ "ok": true }))
}

fn handle_assessments_reorder(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };
    let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing markSetId", None),
    };
    let Some(arr) = req
        .params
        .get("orderedAssessmentIds")
        .and_then(|v| v.as_array())
    else {
        return err(
            &req.id,
            "bad_params",
            "missing/invalid orderedAssessmentIds",
            None,
        );
    };
    let mut ordered: Vec<String> = Vec::with_capacity(arr.len());
    for v in arr {
        let Some(s) = v.as_str() else {
            return err(
                &req.id,
                "bad_params",
                "orderedAssessmentIds must be strings",
                None,
            );
        };
        ordered.push(s.to_string());
    }

    match mark_set_exists(conn, &class_id, &mark_set_id) {
        Ok(true) => {}
        Ok(false) => return err(&req.id, "not_found", "mark set not found", None),
        Err(e) => return e.response(&req.id),
    }

    let mut stmt = match conn.prepare("SELECT id FROM assessments WHERE mark_set_id = ? ORDER BY idx")
    {
        Ok(s) => s,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let current_ids: Vec<String> = match stmt
        .query_map([&mark_set_id], |row| row.get::<_, String>(0))
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    if ordered.len() != current_ids.len() {
        return err(
            &req.id,
            "bad_params",
            "orderedAssessmentIds must be a permutation of the mark set assessments",
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
                "orderedAssessmentIds contains duplicates",
                Some(json!({ "assessmentId": id })),
            );
        }
        if !current_set.contains(id) {
            return err(
                &req.id,
                "bad_params",
                "orderedAssessmentIds contains unknown assessmentId",
                Some(json!({ "assessmentId": id })),
            );
        }
    }

    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return err(&req.id, "db_tx_failed", e.to_string(), None),
    };

    // Avoid UNIQUE collisions by first moving all idx into a temporary range.
    if let Err(e) = tx.execute(
        "UPDATE assessments SET idx = idx + 1000000 WHERE mark_set_id = ?",
        [&mark_set_id],
    ) {
        return err(
            &req.id,
            "db_update_failed",
            e.to_string(),
            Some(json!({ "table": "assessments" })),
        );
    }

    let mut up = match tx.prepare("UPDATE assessments SET idx = ? WHERE id = ? AND mark_set_id = ?")
    {
        Ok(s) => s,
        Err(e) => {
            return err(
                &req.id,
                "db_update_failed",
                e.to_string(),
                Some(json!({ "table": "assessments" })),
            )
        }
    };
    for (i, aid) in ordered.iter().enumerate() {
        if let Err(e) = up.execute((i as i64, aid, &mark_set_id)) {
            return err(
                &req.id,
                "db_update_failed",
                e.to_string(),
                Some(json!({ "table": "assessments" })),
            );
        }
    }
    drop(up);

    if let Err(e) = tx.commit() {
        return err(&req.id, "db_commit_failed", e.to_string(), None);
    }

    ok(&req.id, json!({ "ok": true }))
}

fn handle_markset_settings_get(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };
    let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing markSetId", None),
    };

    let row: Option<(
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        i64,
        i64,
    )> = match conn
        .query_row(
            "SELECT id, code, description, full_code, room, day, period, weight_method, calc_method
             FROM mark_sets WHERE id = ? AND class_id = ?",
            (&mark_set_id, &class_id),
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                    r.get(8)?,
                ))
            },
        )
        .optional()
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    let Some((
        id,
        code,
        description,
        full_code,
        room,
        day,
        period,
        weight_method,
        calc_method,
    )) = row
    else {
        return err(&req.id, "not_found", "mark set not found", None);
    };

    ok(
        &req.id,
        json!({
            "markSet": {
                "id": id,
                "code": code,
                "description": description,
                "fullCode": full_code,
                "room": room,
                "day": day,
                "period": period,
                "weightMethod": weight_method,
                "calcMethod": calc_method
            }
        }),
    )
}

fn handle_markset_settings_update(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };
    let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing markSetId", None),
    };
    let Some(patch) = req.params.get("patch").and_then(|v| v.as_object()) else {
        return err(&req.id, "bad_params", "missing/invalid patch", None);
    };

    let mut set_parts: Vec<String> = Vec::new();
    let mut bind_values: Vec<Value> = Vec::new();

    if let Some(v) = patch.get("fullCode") {
        if v.is_null() {
            set_parts.push("full_code = ?".into());
            bind_values.push(Value::Null);
        } else if let Some(s) = v.as_str() {
            let t = s.trim().to_string();
            set_parts.push("full_code = ?".into());
            if t.is_empty() {
                bind_values.push(Value::Null);
            } else {
                bind_values.push(Value::Text(t));
            }
        } else {
            return err(&req.id, "bad_params", "patch.fullCode must be string or null", None);
        }
    }
    if let Some(v) = patch.get("room") {
        if v.is_null() {
            set_parts.push("room = ?".into());
            bind_values.push(Value::Null);
        } else if let Some(s) = v.as_str() {
            let t = s.trim().to_string();
            set_parts.push("room = ?".into());
            if t.is_empty() {
                bind_values.push(Value::Null);
            } else {
                bind_values.push(Value::Text(t));
            }
        } else {
            return err(&req.id, "bad_params", "patch.room must be string or null", None);
        }
    }
    if let Some(v) = patch.get("day") {
        if v.is_null() {
            set_parts.push("day = ?".into());
            bind_values.push(Value::Null);
        } else if let Some(s) = v.as_str() {
            let t = s.trim().to_string();
            set_parts.push("day = ?".into());
            if t.is_empty() {
                bind_values.push(Value::Null);
            } else {
                bind_values.push(Value::Text(t));
            }
        } else {
            return err(&req.id, "bad_params", "patch.day must be string or null", None);
        }
    }
    if let Some(v) = patch.get("period") {
        if v.is_null() {
            set_parts.push("period = ?".into());
            bind_values.push(Value::Null);
        } else if let Some(s) = v.as_str() {
            let t = s.trim().to_string();
            set_parts.push("period = ?".into());
            if t.is_empty() {
                bind_values.push(Value::Null);
            } else {
                bind_values.push(Value::Text(t));
            }
        } else {
            return err(&req.id, "bad_params", "patch.period must be string or null", None);
        }
    }
    if let Some(v) = patch.get("weightMethod") {
        let Some(n) = v.as_i64() else {
            return err(&req.id, "bad_params", "patch.weightMethod must be integer", None);
        };
        if !(0..=2).contains(&n) {
            return err(&req.id, "bad_params", "patch.weightMethod must be 0, 1, or 2", None);
        }
        set_parts.push("weight_method = ?".into());
        bind_values.push(Value::Integer(n));
    }
    if let Some(v) = patch.get("calcMethod") {
        let Some(n) = v.as_i64() else {
            return err(&req.id, "bad_params", "patch.calcMethod must be integer", None);
        };
        if !(0..=4).contains(&n) {
            return err(&req.id, "bad_params", "patch.calcMethod must be 0..4", None);
        }
        set_parts.push("calc_method = ?".into());
        bind_values.push(Value::Integer(n));
    }

    if set_parts.is_empty() {
        return err(
            &req.id,
            "bad_params",
            "patch must include at least one field",
            None,
        );
    }

    let sql = format!(
        "UPDATE mark_sets SET {} WHERE id = ? AND class_id = ?",
        set_parts.join(", ")
    );
    bind_values.push(Value::Text(mark_set_id.clone()));
    bind_values.push(Value::Text(class_id.clone()));

    let changed = match conn.execute(&sql, params_from_iter(bind_values)) {
        Ok(v) => v,
        Err(e) => {
            return err(
                &req.id,
                "db_update_failed",
                e.to_string(),
                Some(json!({ "table": "mark_sets" })),
            )
        }
    };
    if changed == 0 {
        return err(&req.id, "not_found", "mark set not found", None);
    }

    ok(&req.id, json!({ "ok": true }))
}

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "categories.list" => Some(handle_categories_list(state, req)),
        "categories.create" => Some(handle_categories_create(state, req)),
        "categories.update" => Some(handle_categories_update(state, req)),
        "categories.delete" => Some(handle_categories_delete(state, req)),
        "assessments.list" => Some(handle_assessments_list(state, req)),
        "assessments.create" => Some(handle_assessments_create(state, req)),
        "assessments.update" => Some(handle_assessments_update(state, req)),
        "assessments.delete" => Some(handle_assessments_delete(state, req)),
        "assessments.reorder" => Some(handle_assessments_reorder(state, req)),
        "markset.settings.get" => Some(handle_markset_settings_get(state, req)),
        "markset.settings.update" => Some(handle_markset_settings_update(state, req)),
        _ => None,
    }
}

