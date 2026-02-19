use crate::ipc::error::{err, ok};
use crate::ipc::types::{AppState, Request};
use rusqlite::types::Value;
use rusqlite::{params_from_iter, Connection, OptionalExtension};
use serde_json::json;
use std::collections::{HashMap, HashSet};
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

fn mark_set_exists(
    conn: &Connection,
    class_id: &str,
    mark_set_id: &str,
) -> Result<bool, HandlerErr> {
    conn.query_row(
        "SELECT 1 FROM mark_sets WHERE id = ? AND class_id = ? AND deleted_at IS NULL",
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

fn class_exists(conn: &Connection, class_id: &str) -> Result<bool, HandlerErr> {
    conn.query_row("SELECT 1 FROM classes WHERE id = ?", [class_id], |r| {
        r.get::<_, i64>(0)
    })
    .optional()
    .map(|v| v.is_some())
    .map_err(|e| HandlerErr {
        code: "db_query_failed",
        message: e.to_string(),
        details: None,
    })
}

fn normalized_opt_str(
    value: Option<&serde_json::Value>,
    field: &'static str,
) -> Result<Option<String>, HandlerErr> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let Some(s) = value.as_str() else {
        return Err(HandlerErr {
            code: "bad_params",
            message: format!("{field} must be string or null"),
            details: None,
        });
    };
    let t = s.trim().to_string();
    if t.is_empty() { Ok(None) } else { Ok(Some(t)) }
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
            return err(
                &req.id,
                "bad_params",
                "patch.weight must be a number or null",
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
            return err(
                &req.id,
                "bad_params",
                "patch.date must be a string or null",
                None,
            );
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

    if let Err(e) = tx.execute(
        "DELETE FROM scores WHERE assessment_id = ?",
        [&assessment_id],
    ) {
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

    let mut stmt =
        match conn.prepare("SELECT id FROM assessments WHERE mark_set_id = ? ORDER BY idx") {
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
        i64,
        Option<String>,
        Option<String>,
    )> = match conn
        .query_row(
            "SELECT
                id,
                code,
                description,
                full_code,
                room,
                day,
                period,
                weight_method,
                calc_method,
                is_default,
                deleted_at,
                block_title
             FROM mark_sets
             WHERE id = ? AND class_id = ? AND deleted_at IS NULL",
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
                    r.get(9)?,
                    r.get(10)?,
                    r.get(11)?,
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
        is_default,
        deleted_at,
        block_title,
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
                "calcMethod": calc_method,
                "isDefault": is_default != 0,
                "deletedAt": deleted_at,
                "blockTitle": block_title
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
            return err(
                &req.id,
                "bad_params",
                "patch.fullCode must be string or null",
                None,
            );
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
            return err(
                &req.id,
                "bad_params",
                "patch.room must be string or null",
                None,
            );
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
            return err(
                &req.id,
                "bad_params",
                "patch.day must be string or null",
                None,
            );
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
            return err(
                &req.id,
                "bad_params",
                "patch.period must be string or null",
                None,
            );
        }
    }
    if let Some(v) = patch.get("blockTitle") {
        if v.is_null() {
            set_parts.push("block_title = ?".into());
            bind_values.push(Value::Null);
        } else if let Some(s) = v.as_str() {
            let t = s.trim().to_string();
            set_parts.push("block_title = ?".into());
            if t.is_empty() {
                bind_values.push(Value::Null);
            } else {
                bind_values.push(Value::Text(t));
            }
        } else {
            return err(
                &req.id,
                "bad_params",
                "patch.blockTitle must be string or null",
                None,
            );
        }
    }
    if let Some(v) = patch.get("weightMethod") {
        let Some(n) = v.as_i64() else {
            return err(
                &req.id,
                "bad_params",
                "patch.weightMethod must be integer",
                None,
            );
        };
        if !(0..=2).contains(&n) {
            return err(
                &req.id,
                "bad_params",
                "patch.weightMethod must be 0, 1, or 2",
                None,
            );
        }
        set_parts.push("weight_method = ?".into());
        bind_values.push(Value::Integer(n));
    }
    if let Some(v) = patch.get("calcMethod") {
        let Some(n) = v.as_i64() else {
            return err(
                &req.id,
                "bad_params",
                "patch.calcMethod must be integer",
                None,
            );
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

fn ensure_mark_set_code_unique(
    conn: &Connection,
    class_id: &str,
    code: &str,
    exclude_mark_set_id: Option<&str>,
) -> Result<(), HandlerErr> {
    let existing: Option<String> = if let Some(exclude_id) = exclude_mark_set_id {
        conn.query_row(
            "SELECT id FROM mark_sets
             WHERE class_id = ? AND UPPER(code) = UPPER(?) AND id <> ?",
            (class_id, code, exclude_id),
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?
    } else {
        conn.query_row(
            "SELECT id FROM mark_sets
             WHERE class_id = ? AND UPPER(code) = UPPER(?)",
            (class_id, code),
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?
    };

    if existing.is_some() {
        return Err(HandlerErr {
            code: "bad_params",
            message: "mark set code already exists in class".into(),
            details: Some(json!({ "field": "code" })),
        });
    }
    Ok(())
}

fn handle_marksets_create(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };
    match class_exists(conn, &class_id) {
        Ok(true) => {}
        Ok(false) => return err(&req.id, "not_found", "class not found", None),
        Err(e) => return e.response(&req.id),
    }

    let code = match req.params.get("code").and_then(|v| v.as_str()) {
        Some(v) => v.trim().to_string(),
        None => return err(&req.id, "bad_params", "missing code", None),
    };
    if code.is_empty() {
        return err(&req.id, "bad_params", "code must not be empty", None);
    }
    if code.len() > 15 {
        return err(&req.id, "bad_params", "code must be 15 chars or fewer", None);
    }
    let description = match req.params.get("description").and_then(|v| v.as_str()) {
        Some(v) => v.trim().to_string(),
        None => return err(&req.id, "bad_params", "missing description", None),
    };
    if description.is_empty() {
        return err(&req.id, "bad_params", "description must not be empty", None);
    }

    if let Err(e) = ensure_mark_set_code_unique(conn, &class_id, &code, None) {
        return e.response(&req.id);
    }

    let file_prefix = req
        .params
        .get("filePrefix")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| code.clone());

    let weight = req.params.get("weight").and_then(|v| v.as_f64());
    let full_code = match normalized_opt_str(req.params.get("fullCode"), "fullCode") {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let room = match normalized_opt_str(req.params.get("room"), "room") {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let day = match normalized_opt_str(req.params.get("day"), "day") {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let period = match normalized_opt_str(req.params.get("period"), "period") {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let block_title = match normalized_opt_str(req.params.get("blockTitle"), "blockTitle") {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };

    let weight_method = req
        .params
        .get("weightMethod")
        .and_then(|v| v.as_i64())
        .unwrap_or(1);
    if !(0..=2).contains(&weight_method) {
        return err(&req.id, "bad_params", "weightMethod must be 0, 1, or 2", None);
    }
    let calc_method = req
        .params
        .get("calcMethod")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if !(0..=4).contains(&calc_method) {
        return err(&req.id, "bad_params", "calcMethod must be 0..4", None);
    }
    let make_default = req
        .params
        .get("makeDefault")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let active_mark_set_count: i64 = match conn.query_row(
        "SELECT COUNT(*) FROM mark_sets WHERE class_id = ? AND deleted_at IS NULL",
        [&class_id],
        |r| r.get(0),
    ) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let make_default = make_default || active_mark_set_count == 0;

    let starter_categories = req
        .params
        .get("starterCategories")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let sort_order: i64 = match conn.query_row(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM mark_sets WHERE class_id = ?",
        [&class_id],
        |r| r.get(0),
    ) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    let mark_set_id = Uuid::new_v4().to_string();
    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return err(&req.id, "db_tx_failed", e.to_string(), None),
    };

    if make_default {
        if let Err(e) = tx.execute(
            "UPDATE mark_sets SET is_default = 0 WHERE class_id = ?",
            [&class_id],
        ) {
            let _ = tx.rollback();
            return err(
                &req.id,
                "db_update_failed",
                e.to_string(),
                Some(json!({ "table": "mark_sets" })),
            );
        }
    }

    if let Err(e) = tx.execute(
        "INSERT INTO mark_sets(
            id,
            class_id,
            code,
            file_prefix,
            description,
            weight,
            source_filename,
            sort_order,
            full_code,
            room,
            day,
            period,
            weight_method,
            calc_method,
            is_default,
            deleted_at,
            block_title
        ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?)",
        (
            &mark_set_id,
            &class_id,
            &code,
            &file_prefix,
            &description,
            weight,
            Option::<&str>::None,
            sort_order,
            full_code.as_deref(),
            room.as_deref(),
            day.as_deref(),
            period.as_deref(),
            weight_method,
            calc_method,
            if make_default { 1 } else { 0 },
            block_title.as_deref(),
        ),
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_insert_failed",
            e.to_string(),
            Some(json!({ "table": "mark_sets" })),
        );
    }

    for (idx, item) in starter_categories.iter().enumerate() {
        let Some(obj) = item.as_object() else {
            continue;
        };
        let Some(name) = obj.get("name").and_then(|v| v.as_str()) else {
            continue;
        };
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        let weight = obj.get("weight").and_then(|v| v.as_f64());
        let category_id = Uuid::new_v4().to_string();
        if let Err(e) = tx.execute(
            "INSERT INTO categories(id, mark_set_id, name, weight, sort_order) VALUES(?, ?, ?, ?, ?)",
            (&category_id, &mark_set_id, name, weight, idx as i64),
        ) {
            let _ = tx.rollback();
            return err(
                &req.id,
                "db_insert_failed",
                e.to_string(),
                Some(json!({ "table": "categories" })),
            );
        }
    }

    if let Err(e) = tx.commit() {
        return err(&req.id, "db_commit_failed", e.to_string(), None);
    }

    ok(&req.id, json!({ "markSetId": mark_set_id }))
}

fn handle_marksets_delete(state: &mut AppState, req: &Request) -> serde_json::Value {
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

    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return err(&req.id, "db_tx_failed", e.to_string(), None),
    };
    let changed = match tx.execute(
        "UPDATE mark_sets
         SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ','now'),
             is_default = 0
         WHERE id = ? AND class_id = ?",
        (&mark_set_id, &class_id),
    ) {
        Ok(v) => v,
        Err(e) => {
            let _ = tx.rollback();
            return err(
                &req.id,
                "db_update_failed",
                e.to_string(),
                Some(json!({ "table": "mark_sets" })),
            );
        }
    };
    if changed == 0 {
        let _ = tx.rollback();
        return err(&req.id, "not_found", "mark set not found", None);
    }

    let next_default: Option<String> = match tx
        .query_row(
            "SELECT id FROM mark_sets
             WHERE class_id = ? AND deleted_at IS NULL
             ORDER BY sort_order LIMIT 1",
            [&class_id],
            |r| r.get(0),
        )
        .optional()
    {
        Ok(v) => v,
        Err(e) => {
            let _ = tx.rollback();
            return err(&req.id, "db_query_failed", e.to_string(), None);
        }
    };
    if let Some(next_id) = next_default {
        if let Err(e) = tx.execute("UPDATE mark_sets SET is_default = 1 WHERE id = ?", [&next_id]) {
            let _ = tx.rollback();
            return err(
                &req.id,
                "db_update_failed",
                e.to_string(),
                Some(json!({ "table": "mark_sets" })),
            );
        }
    }

    if let Err(e) = tx.commit() {
        return err(&req.id, "db_commit_failed", e.to_string(), None);
    }

    ok(&req.id, json!({ "ok": true }))
}

fn handle_marksets_undelete(state: &mut AppState, req: &Request) -> serde_json::Value {
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

    let changed = match conn.execute(
        "UPDATE mark_sets SET deleted_at = NULL WHERE id = ? AND class_id = ?",
        (&mark_set_id, &class_id),
    ) {
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

fn handle_marksets_set_default(state: &mut AppState, req: &Request) -> serde_json::Value {
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

    let exists_active: Option<i64> = match conn
        .query_row(
            "SELECT 1 FROM mark_sets WHERE id = ? AND class_id = ? AND deleted_at IS NULL",
            (&mark_set_id, &class_id),
            |r| r.get(0),
        )
        .optional()
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    if exists_active.is_none() {
        return err(&req.id, "not_found", "mark set not found", None);
    }

    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return err(&req.id, "db_tx_failed", e.to_string(), None),
    };
    if let Err(e) = tx.execute("UPDATE mark_sets SET is_default = 0 WHERE class_id = ?", [&class_id]) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_update_failed",
            e.to_string(),
            Some(json!({ "table": "mark_sets" })),
        );
    }
    if let Err(e) = tx.execute("UPDATE mark_sets SET is_default = 1 WHERE id = ?", [&mark_set_id]) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_update_failed",
            e.to_string(),
            Some(json!({ "table": "mark_sets" })),
        );
    }
    if let Err(e) = tx.commit() {
        return err(&req.id, "db_commit_failed", e.to_string(), None);
    }
    ok(&req.id, json!({ "ok": true }))
}

fn handle_marksets_clone(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };
    let source_mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing markSetId", None),
    };

    let source_row: Option<(
        String,
        String,
        Option<f64>,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        i64,
        i64,
        Option<String>,
    )> = match conn
        .query_row(
            "SELECT
                description,
                file_prefix,
                weight,
                full_code,
                room,
                day,
                period,
                weight_method,
                calc_method,
                block_title
             FROM mark_sets
             WHERE id = ? AND class_id = ?",
            (&source_mark_set_id, &class_id),
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
                    r.get(9)?,
                ))
            },
        )
        .optional()
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let Some((
        source_description,
        source_file_prefix,
        source_weight,
        source_full_code,
        source_room,
        source_day,
        source_period,
        source_weight_method,
        source_calc_method,
        source_block_title,
    )) = source_row
    else {
        return err(&req.id, "not_found", "mark set not found", None);
    };

    let code = req
        .params
        .get("code")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{}C", source_file_prefix));
    if code.len() > 15 {
        return err(&req.id, "bad_params", "code must be 15 chars or fewer", None);
    }
    if let Err(e) = ensure_mark_set_code_unique(conn, &class_id, &code, None) {
        return e.response(&req.id);
    }
    let description = req
        .params
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("{source_description} (Copy)"));
    let clone_assessments = req
        .params
        .get("cloneAssessments")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let clone_scores = req
        .params
        .get("cloneScores")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let sort_order: i64 = match conn.query_row(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM mark_sets WHERE class_id = ?",
        [&class_id],
        |r| r.get(0),
    ) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    let categories: Vec<(String, Option<f64>, i64)> = match conn.prepare(
        "SELECT name, weight, sort_order
         FROM categories
         WHERE mark_set_id = ?
         ORDER BY sort_order",
    ) {
        Ok(mut stmt) => match stmt
            .query_map([&source_mark_set_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        {
            Ok(v) => v,
            Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
        },
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    let assessments: Vec<(
        String,
        i64,
        Option<String>,
        Option<String>,
        String,
        Option<i64>,
        Option<i64>,
        Option<f64>,
        Option<f64>,
    )> = if clone_assessments {
        match conn.prepare(
            "SELECT id, idx, date, category_name, title, term, legacy_type, weight, out_of
             FROM assessments
             WHERE mark_set_id = ?
             ORDER BY idx",
        ) {
            Ok(mut stmt) => match stmt
                .query_map([&source_mark_set_id], |r| {
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
                })
                .and_then(|it| it.collect::<Result<Vec<_>, _>>())
            {
                Ok(v) => v,
                Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
            },
            Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
        }
    } else {
        Vec::new()
    };

    let mut scores_by_assessment: HashMap<String, Vec<(String, Option<f64>, String, Option<String>)>> =
        HashMap::new();
    if clone_assessments && clone_scores {
        for (source_assessment_id, ..) in &assessments {
            let score_rows: Vec<(String, Option<f64>, String, Option<String>)> = match conn
                .prepare(
                    "SELECT student_id, raw_value, status, remark
                     FROM scores
                     WHERE assessment_id = ?",
                ) {
                Ok(mut stmt) => match stmt
                    .query_map([source_assessment_id], |r| {
                        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
                    })
                    .and_then(|it| it.collect::<Result<Vec<_>, _>>())
                {
                    Ok(v) => v,
                    Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
                },
                Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
            };
            scores_by_assessment.insert(source_assessment_id.clone(), score_rows);
        }
    }

    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return err(&req.id, "db_tx_failed", e.to_string(), None),
    };
    let new_mark_set_id = Uuid::new_v4().to_string();
    if let Err(e) = tx.execute(
        "INSERT INTO mark_sets(
            id,
            class_id,
            code,
            file_prefix,
            description,
            weight,
            source_filename,
            sort_order,
            full_code,
            room,
            day,
            period,
            weight_method,
            calc_method,
            is_default,
            deleted_at,
            block_title
        ) VALUES(?, ?, ?, ?, ?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, 0, NULL, ?)",
        (
            &new_mark_set_id,
            &class_id,
            &code,
            &code,
            &description,
            source_weight,
            sort_order,
            source_full_code.as_deref(),
            source_room.as_deref(),
            source_day.as_deref(),
            source_period.as_deref(),
            source_weight_method,
            source_calc_method,
            source_block_title.as_deref(),
        ),
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_insert_failed",
            e.to_string(),
            Some(json!({ "table": "mark_sets" })),
        );
    }

    for (name, weight, sort_order) in categories {
        let category_id = Uuid::new_v4().to_string();
        if let Err(e) = tx.execute(
            "INSERT INTO categories(id, mark_set_id, name, weight, sort_order) VALUES(?, ?, ?, ?, ?)",
            (&category_id, &new_mark_set_id, &name, weight, sort_order),
        ) {
            let _ = tx.rollback();
            return err(
                &req.id,
                "db_insert_failed",
                e.to_string(),
                Some(json!({ "table": "categories" })),
            );
        }
    }

    if clone_assessments {
        let mut assessment_id_map: Vec<(String, String)> = Vec::with_capacity(assessments.len());
        for (
            source_assessment_id,
            idx,
            date,
            category_name,
            title,
            term,
            legacy_type,
            weight,
            out_of,
        ) in assessments
        {
            let new_assessment_id = Uuid::new_v4().to_string();
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
                    &new_assessment_id,
                    &new_mark_set_id,
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
                let _ = tx.rollback();
                return err(
                    &req.id,
                    "db_insert_failed",
                    e.to_string(),
                    Some(json!({ "table": "assessments" })),
                );
            }
            assessment_id_map.push((source_assessment_id, new_assessment_id));
        }

        if clone_scores {
            for (source_assessment_id, new_assessment_id) in assessment_id_map {
                let score_rows = scores_by_assessment
                    .get(&source_assessment_id)
                    .cloned()
                    .unwrap_or_default();
                for (student_id, raw_value, status, remark) in score_rows {
                    let score_id = Uuid::new_v4().to_string();
                    if let Err(e) = tx.execute(
                        "INSERT INTO scores(id, assessment_id, student_id, raw_value, status, remark)
                         VALUES(?, ?, ?, ?, ?, ?)",
                        (
                            &score_id,
                            &new_assessment_id,
                            &student_id,
                            raw_value,
                            &status,
                            remark.as_deref(),
                        ),
                    ) {
                        let _ = tx.rollback();
                        return err(
                            &req.id,
                            "db_insert_failed",
                            e.to_string(),
                            Some(json!({ "table": "scores" })),
                        );
                    }
                }
            }
        }
    }

    if let Err(e) = tx.commit() {
        return err(&req.id, "db_commit_failed", e.to_string(), None);
    }

    ok(&req.id, json!({ "markSetId": new_mark_set_id }))
}

fn handle_assessments_bulk_create(state: &mut AppState, req: &Request) -> serde_json::Value {
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
    let Some(entries) = req.params.get("entries").and_then(|v| v.as_array()) else {
        return err(&req.id, "bad_params", "missing entries", None);
    };
    if entries.is_empty() {
        return err(&req.id, "bad_params", "entries must not be empty", None);
    }

    match mark_set_exists(conn, &class_id, &mark_set_id) {
        Ok(true) => {}
        Ok(false) => return err(&req.id, "not_found", "mark set not found", None),
        Err(e) => return e.response(&req.id),
    }

    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return err(&req.id, "db_tx_failed", e.to_string(), None),
    };
    let mut next_idx: i64 = match tx.query_row(
        "SELECT COALESCE(MAX(idx), -1) + 1 FROM assessments WHERE mark_set_id = ?",
        [&mark_set_id],
        |r| r.get(0),
    ) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    let mut created_ids: Vec<String> = Vec::new();
    for (row_idx, raw) in entries.iter().enumerate() {
        let Some(obj) = raw.as_object() else {
            let _ = tx.rollback();
            return err(
                &req.id,
                "bad_params",
                "entries must be objects",
                Some(json!({ "row": row_idx })),
            );
        };
        let Some(title) = obj.get("title").and_then(|v| v.as_str()) else {
            let _ = tx.rollback();
            return err(
                &req.id,
                "bad_params",
                "entry.title is required",
                Some(json!({ "row": row_idx })),
            );
        };
        let title = title.trim().to_string();
        if title.is_empty() {
            let _ = tx.rollback();
            return err(
                &req.id,
                "bad_params",
                "entry.title must not be empty",
                Some(json!({ "row": row_idx })),
            );
        }
        let date = obj
            .get("date")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .and_then(|s| if s.is_empty() { None } else { Some(s) });
        let category_name = obj
            .get("categoryName")
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .and_then(|s| if s.is_empty() { None } else { Some(s) });
        let term = obj.get("term").and_then(|v| v.as_i64());
        let legacy_type = obj.get("legacyType").and_then(|v| v.as_i64());
        let weight = obj.get("weight").and_then(|v| v.as_f64());
        let out_of = obj.get("outOf").and_then(|v| v.as_f64());

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
                next_idx,
                date.as_deref(),
                category_name.as_deref(),
                &title,
                term,
                legacy_type,
                weight,
                out_of,
            ),
        ) {
            let _ = tx.rollback();
            return err(
                &req.id,
                "db_insert_failed",
                e.to_string(),
                Some(json!({ "table": "assessments", "row": row_idx })),
            );
        }
        next_idx += 1;
        created_ids.push(assessment_id);
    }

    if let Err(e) = tx.commit() {
        return err(&req.id, "db_commit_failed", e.to_string(), None);
    }

    ok(
        &req.id,
        json!({
            "ok": true,
            "created": created_ids.len(),
            "assessmentIds": created_ids
        }),
    )
}

fn handle_assessments_bulk_update(state: &mut AppState, req: &Request) -> serde_json::Value {
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
    let Some(updates) = req.params.get("updates").and_then(|v| v.as_array()) else {
        return err(&req.id, "bad_params", "missing updates", None);
    };
    if updates.is_empty() {
        return err(&req.id, "bad_params", "updates must not be empty", None);
    }

    match mark_set_exists(conn, &class_id, &mark_set_id) {
        Ok(true) => {}
        Ok(false) => return err(&req.id, "not_found", "mark set not found", None),
        Err(e) => return e.response(&req.id),
    }

    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return err(&req.id, "db_tx_failed", e.to_string(), None),
    };
    let mut updated = 0usize;
    let mut rejected: Vec<serde_json::Value> = Vec::new();

    for (row_idx, update) in updates.iter().enumerate() {
        let Some(obj) = update.as_object() else {
            rejected.push(json!({
                "index": row_idx,
                "code": "bad_params",
                "message": "update row must be object"
            }));
            continue;
        };
        let Some(assessment_id) = obj.get("assessmentId").and_then(|v| v.as_str()) else {
            rejected.push(json!({
                "index": row_idx,
                "code": "bad_params",
                "message": "assessmentId is required"
            }));
            continue;
        };
        let Some(patch) = obj.get("patch").and_then(|v| v.as_object()) else {
            rejected.push(json!({
                "index": row_idx,
                "assessmentId": assessment_id,
                "code": "bad_params",
                "message": "patch is required"
            }));
            continue;
        };

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
                rejected.push(json!({
                    "index": row_idx,
                    "assessmentId": assessment_id,
                    "code": "bad_params",
                    "message": "patch.date must be string or null"
                }));
                continue;
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
                rejected.push(json!({
                    "index": row_idx,
                    "assessmentId": assessment_id,
                    "code": "bad_params",
                    "message": "patch.categoryName must be string or null"
                }));
                continue;
            }
        }
        if let Some(v) = patch.get("title") {
            let Some(s) = v.as_str() else {
                rejected.push(json!({
                    "index": row_idx,
                    "assessmentId": assessment_id,
                    "code": "bad_params",
                    "message": "patch.title must be string"
                }));
                continue;
            };
            let t = s.trim().to_string();
            if t.is_empty() {
                rejected.push(json!({
                    "index": row_idx,
                    "assessmentId": assessment_id,
                    "code": "bad_params",
                    "message": "patch.title must not be empty"
                }));
                continue;
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
                rejected.push(json!({
                    "index": row_idx,
                    "assessmentId": assessment_id,
                    "code": "bad_params",
                    "message": "patch.term must be integer or null"
                }));
                continue;
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
                rejected.push(json!({
                    "index": row_idx,
                    "assessmentId": assessment_id,
                    "code": "bad_params",
                    "message": "patch.legacyType must be integer or null"
                }));
                continue;
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
                rejected.push(json!({
                    "index": row_idx,
                    "assessmentId": assessment_id,
                    "code": "bad_params",
                    "message": "patch.weight must be number or null"
                }));
                continue;
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
                rejected.push(json!({
                    "index": row_idx,
                    "assessmentId": assessment_id,
                    "code": "bad_params",
                    "message": "patch.outOf must be number or null"
                }));
                continue;
            }
        }

        if set_parts.is_empty() {
            rejected.push(json!({
                "index": row_idx,
                "assessmentId": assessment_id,
                "code": "bad_params",
                "message": "patch must include at least one field"
            }));
            continue;
        }

        let sql = format!(
            "UPDATE assessments SET {} WHERE id = ? AND mark_set_id = ?",
            set_parts.join(", ")
        );
        bind_values.push(Value::Text(assessment_id.to_string()));
        bind_values.push(Value::Text(mark_set_id.clone()));
        match tx.execute(&sql, params_from_iter(bind_values)) {
            Ok(0) => rejected.push(json!({
                "index": row_idx,
                "assessmentId": assessment_id,
                "code": "not_found",
                "message": "assessment not found"
            })),
            Ok(_) => updated += 1,
            Err(e) => {
                rejected.push(json!({
                    "index": row_idx,
                    "assessmentId": assessment_id,
                    "code": "db_update_failed",
                    "message": e.to_string()
                }));
            }
        }
    }

    if let Err(e) = tx.commit() {
        return err(&req.id, "db_commit_failed", e.to_string(), None);
    }

    ok(
        &req.id,
        json!({
            "ok": true,
            "updated": updated,
            "rejected": rejected.len(),
            "errors": rejected
        }),
    )
}

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "marksets.create" => Some(handle_marksets_create(state, req)),
        "marksets.delete" => Some(handle_marksets_delete(state, req)),
        "marksets.undelete" => Some(handle_marksets_undelete(state, req)),
        "marksets.setDefault" => Some(handle_marksets_set_default(state, req)),
        "marksets.clone" => Some(handle_marksets_clone(state, req)),
        "categories.list" => Some(handle_categories_list(state, req)),
        "categories.create" => Some(handle_categories_create(state, req)),
        "categories.update" => Some(handle_categories_update(state, req)),
        "categories.delete" => Some(handle_categories_delete(state, req)),
        "assessments.list" => Some(handle_assessments_list(state, req)),
        "assessments.create" => Some(handle_assessments_create(state, req)),
        "assessments.bulkCreate" => Some(handle_assessments_bulk_create(state, req)),
        "assessments.update" => Some(handle_assessments_update(state, req)),
        "assessments.bulkUpdate" => Some(handle_assessments_bulk_update(state, req)),
        "assessments.delete" => Some(handle_assessments_delete(state, req)),
        "assessments.reorder" => Some(handle_assessments_reorder(state, req)),
        "markset.settings.get" => Some(handle_markset_settings_get(state, req)),
        "markset.settings.update" => Some(handle_markset_settings_update(state, req)),
        _ => None,
    }
}
