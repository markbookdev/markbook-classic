use crate::ipc::error::{err, ok};
use crate::ipc::types::{AppState, Request};
use rusqlite::{params_from_iter, Connection, OptionalExtension, ToSql};
use serde_json::json;
use std::collections::HashMap;
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

#[derive(Debug, Clone)]
struct BasicStudent {
    id: String,
    display_name: String,
    sort_order: i64,
    active: bool,
}

fn get_required_str(params: &serde_json::Value, key: &str) -> Result<String, HandlerErr> {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| HandlerErr {
            code: "bad_params",
            message: format!("missing {}", key),
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

fn list_students_for_class(conn: &Connection, class_id: &str) -> Result<Vec<BasicStudent>, HandlerErr> {
    let mut stmt = conn
        .prepare(
            "SELECT id, last_name, first_name, sort_order, active
             FROM students
             WHERE class_id = ?
             ORDER BY sort_order",
        )
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    stmt.query_map([class_id], |r| {
        let last: String = r.get(1)?;
        let first: String = r.get(2)?;
        Ok(BasicStudent {
            id: r.get(0)?,
            display_name: format!("{}, {}", last, first),
            sort_order: r.get(3)?,
            active: r.get::<_, i64>(4)? != 0,
        })
    })
    .and_then(|it| it.collect::<Result<Vec<_>, _>>())
    .map_err(|e| HandlerErr {
        code: "db_query_failed",
        message: e.to_string(),
        details: None,
    })
}

fn loaned_list(conn: &Connection, params: &serde_json::Value) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    if !class_exists(conn, &class_id)? {
        return Err(HandlerErr {
            code: "not_found",
            message: "class not found".to_string(),
            details: None,
        });
    }
    let mark_set_id = params
        .get("markSetId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let (sql, binds): (&str, Vec<&dyn ToSql>) = if let Some(ref msid) = mark_set_id {
        (
            "SELECT li.id, li.student_id, s.last_name, s.first_name, li.mark_set_id, li.item_name, li.quantity, li.notes, li.raw_line
             FROM loaned_items li
             JOIN students s ON s.id = li.student_id
             WHERE li.class_id = ? AND li.mark_set_id = ?
             ORDER BY s.sort_order, li.item_name",
            vec![&class_id as &dyn ToSql, msid as &dyn ToSql],
        )
    } else {
        (
            "SELECT li.id, li.student_id, s.last_name, s.first_name, li.mark_set_id, li.item_name, li.quantity, li.notes, li.raw_line
             FROM loaned_items li
             JOIN students s ON s.id = li.student_id
             WHERE li.class_id = ?
             ORDER BY s.sort_order, li.item_name",
            vec![&class_id as &dyn ToSql],
        )
    };

    let mut stmt = conn.prepare(sql).map_err(|e| HandlerErr {
        code: "db_query_failed",
        message: e.to_string(),
        details: None,
    })?;
    let rows = stmt
        .query_map(params_from_iter(binds), |r| {
            let id: String = r.get(0)?;
            let student_id: String = r.get(1)?;
            let last_name: String = r.get(2)?;
            let first_name: String = r.get(3)?;
            let mark_set_id: Option<String> = r.get(4)?;
            let item_name: String = r.get(5)?;
            let quantity: Option<f64> = r.get(6)?;
            let notes: Option<String> = r.get(7)?;
            let raw_line: String = r.get(8)?;
            Ok(json!({
                "id": id,
                "studentId": student_id,
                "displayName": format!("{}, {}", last_name, first_name),
                "markSetId": mark_set_id,
                "itemName": item_name,
                "quantity": quantity,
                "notes": notes,
                "rawLine": raw_line
            }))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;

    Ok(json!({ "items": rows }))
}

fn loaned_get(conn: &Connection, params: &serde_json::Value) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    let item_id = get_required_str(params, "itemId")?;
    let mut stmt = conn
        .prepare(
            "SELECT li.id, li.student_id, s.last_name, s.first_name, li.mark_set_id, li.item_name, li.quantity, li.notes, li.raw_line
             FROM loaned_items li
             JOIN students s ON s.id = li.student_id
             WHERE li.class_id = ? AND li.id = ?",
        )
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let row = stmt
        .query_row((&class_id, &item_id), |r| {
            let id: String = r.get(0)?;
            let student_id: String = r.get(1)?;
            let last_name: String = r.get(2)?;
            let first_name: String = r.get(3)?;
            let mark_set_id: Option<String> = r.get(4)?;
            let item_name: String = r.get(5)?;
            let quantity: Option<f64> = r.get(6)?;
            let notes: Option<String> = r.get(7)?;
            let raw_line: String = r.get(8)?;
            Ok(json!({
                "id": id,
                "studentId": student_id,
                "displayName": format!("{}, {}", last_name, first_name),
                "markSetId": mark_set_id,
                "itemName": item_name,
                "quantity": quantity,
                "notes": notes,
                "rawLine": raw_line
            }))
        })
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let Some(item) = row else {
        return Err(HandlerErr {
            code: "not_found",
            message: "loaned item not found".to_string(),
            details: None,
        });
    };
    Ok(json!({ "item": item }))
}

fn loaned_update(conn: &Connection, params: &serde_json::Value) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    let student_id = get_required_str(params, "studentId")?;
    let item_name = get_required_str(params, "itemName")?;
    let mark_set_id = params
        .get("markSetId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let quantity = params.get("quantity").and_then(|v| v.as_f64());
    let notes = params
        .get("notes")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let raw_line = params
        .get("rawLine")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let item_id = params
        .get("itemId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let student_exists = conn
        .query_row(
            "SELECT 1 FROM students WHERE class_id = ? AND id = ?",
            (&class_id, &student_id),
            |r| r.get::<_, i64>(0),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?
        .is_some();
    if !student_exists {
        return Err(HandlerErr {
            code: "not_found",
            message: "student not found".to_string(),
            details: None,
        });
    }

    conn.execute(
        "INSERT INTO loaned_items(id, class_id, student_id, mark_set_id, item_name, quantity, notes, raw_line)
         VALUES(?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
           student_id = excluded.student_id,
           mark_set_id = excluded.mark_set_id,
           item_name = excluded.item_name,
           quantity = excluded.quantity,
           notes = excluded.notes,
           raw_line = excluded.raw_line",
        (
            &item_id,
            &class_id,
            &student_id,
            mark_set_id.as_deref(),
            &item_name,
            quantity,
            notes.as_deref(),
            &raw_line,
        ),
    )
    .map_err(|e| HandlerErr {
        code: "db_update_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "loaned_items" })),
    })?;
    Ok(json!({ "ok": true, "itemId": item_id }))
}

fn devices_list(conn: &Connection, params: &serde_json::Value) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    if !class_exists(conn, &class_id)? {
        return Err(HandlerErr {
            code: "not_found",
            message: "class not found".to_string(),
            details: None,
        });
    }

    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.last_name, s.first_name, s.sort_order, s.active, dm.device_code, dm.raw_line
             FROM students s
             LEFT JOIN student_device_map dm
               ON dm.class_id = s.class_id AND dm.student_id = s.id
             WHERE s.class_id = ?
             ORDER BY s.sort_order",
        )
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let rows = stmt
        .query_map([&class_id], |r| {
            let student_id: String = r.get(0)?;
            let last_name: String = r.get(1)?;
            let first_name: String = r.get(2)?;
            let sort_order: i64 = r.get(3)?;
            let active: i64 = r.get(4)?;
            let device_code: Option<String> = r.get(5)?;
            let raw_line: Option<String> = r.get(6)?;
            Ok(json!({
                "studentId": student_id,
                "displayName": format!("{}, {}", last_name, first_name),
                "sortOrder": sort_order,
                "active": active != 0,
                "deviceCode": device_code.unwrap_or_default(),
                "rawLine": raw_line.unwrap_or_default()
            }))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;

    Ok(json!({ "devices": rows }))
}

fn devices_get(conn: &Connection, params: &serde_json::Value) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    let student_id = get_required_str(params, "studentId")?;
    let row = conn
        .query_row(
            "SELECT s.id, s.last_name, s.first_name, s.sort_order, s.active, dm.device_code, dm.raw_line
             FROM students s
             LEFT JOIN student_device_map dm
               ON dm.class_id = s.class_id AND dm.student_id = s.id
             WHERE s.class_id = ? AND s.id = ?",
            (&class_id, &student_id),
            |r| {
                let student_id: String = r.get(0)?;
                let last_name: String = r.get(1)?;
                let first_name: String = r.get(2)?;
                let sort_order: i64 = r.get(3)?;
                let active: i64 = r.get(4)?;
                let device_code: Option<String> = r.get(5)?;
                let raw_line: Option<String> = r.get(6)?;
                Ok(json!({
                    "studentId": student_id,
                    "displayName": format!("{}, {}", last_name, first_name),
                    "sortOrder": sort_order,
                    "active": active != 0,
                    "deviceCode": device_code.unwrap_or_default(),
                    "rawLine": raw_line.unwrap_or_default()
                }))
            },
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let Some(device) = row else {
        return Err(HandlerErr {
            code: "not_found",
            message: "student not found".to_string(),
            details: None,
        });
    };
    Ok(json!({ "device": device }))
}

fn devices_update(conn: &Connection, params: &serde_json::Value) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    let student_id = get_required_str(params, "studentId")?;
    let device_code = params
        .get("deviceCode")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    let raw_line = params
        .get("rawLine")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let student_exists = conn
        .query_row(
            "SELECT 1 FROM students WHERE class_id = ? AND id = ?",
            (&class_id, &student_id),
            |r| r.get::<_, i64>(0),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?
        .is_some();
    if !student_exists {
        return Err(HandlerErr {
            code: "not_found",
            message: "student not found".to_string(),
            details: None,
        });
    }

    if device_code.is_empty() && raw_line.is_empty() {
        conn.execute(
            "DELETE FROM student_device_map WHERE class_id = ? AND student_id = ?",
            (&class_id, &student_id),
        )
        .map_err(|e| HandlerErr {
            code: "db_delete_failed",
            message: e.to_string(),
            details: Some(json!({ "table": "student_device_map" })),
        })?;
        return Ok(json!({ "ok": true }));
    }

    conn.execute(
        "INSERT INTO student_device_map(id, class_id, student_id, device_code, raw_line)
         VALUES(?, ?, ?, ?, ?)
         ON CONFLICT(class_id, student_id) DO UPDATE SET
           device_code = excluded.device_code,
           raw_line = excluded.raw_line",
        (
            Uuid::new_v4().to_string(),
            &class_id,
            &student_id,
            &device_code,
            &raw_line,
        ),
    )
    .map_err(|e| HandlerErr {
        code: "db_update_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "student_device_map" })),
    })?;

    Ok(json!({ "ok": true }))
}

fn learning_skills_open(conn: &Connection, params: &serde_json::Value) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    if !class_exists(conn, &class_id)? {
        return Err(HandlerErr {
            code: "not_found",
            message: "class not found".to_string(),
            details: None,
        });
    }
    let term = params
        .get("term")
        .and_then(|v| v.as_i64())
        .unwrap_or(1)
        .clamp(1, 3);
    let skill_codes: Vec<&'static str> = vec!["R", "O", "I", "C"];
    let students = list_students_for_class(conn, &class_id)?;

    let mut stmt = conn
        .prepare(
            "SELECT student_id, skill_code, value
             FROM learning_skills_cells
             WHERE class_id = ? AND term = ?",
        )
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let rows = stmt
        .query_map((&class_id, term), |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let mut by_key: HashMap<(String, String), String> = HashMap::new();
    for (student_id, skill_code, value) in rows {
        by_key.insert((student_id, skill_code.to_ascii_uppercase()), value);
    }

    let students_json: Vec<serde_json::Value> = students
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "displayName": s.display_name,
                "sortOrder": s.sort_order,
                "active": s.active
            })
        })
        .collect();
    let rows_json: Vec<serde_json::Value> = students
        .iter()
        .map(|s| {
            let mut values = serde_json::Map::new();
            for code in &skill_codes {
                let v = by_key
                    .get(&(s.id.clone(), code.to_string()))
                    .cloned()
                    .unwrap_or_default();
                values.insert((*code).to_string(), json!(v));
            }
            json!({
                "studentId": s.id,
                "values": values
            })
        })
        .collect();

    Ok(json!({
        "classId": class_id,
        "term": term,
        "skillCodes": skill_codes,
        "students": students_json,
        "rows": rows_json
    }))
}

fn learning_skills_update_cell(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    let student_id = get_required_str(params, "studentId")?;
    let skill_code = get_required_str(params, "skillCode")?.to_ascii_uppercase();
    if skill_code.is_empty() || skill_code.len() > 8 {
        return Err(HandlerErr {
            code: "bad_params",
            message: "skillCode must be 1..8 chars".to_string(),
            details: None,
        });
    }
    let term = params
        .get("term")
        .and_then(|v| v.as_i64())
        .unwrap_or(1)
        .clamp(1, 3);
    let value = match params.get("value") {
        None => String::new(),
        Some(v) if v.is_null() => String::new(),
        Some(v) => v.as_str().unwrap_or("").trim().to_string(),
    };

    let student_exists = conn
        .query_row(
            "SELECT 1 FROM students WHERE class_id = ? AND id = ?",
            (&class_id, &student_id),
            |r| r.get::<_, i64>(0),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?
        .is_some();
    if !student_exists {
        return Err(HandlerErr {
            code: "not_found",
            message: "student not found".to_string(),
            details: None,
        });
    }

    if value.is_empty() {
        conn.execute(
            "DELETE FROM learning_skills_cells
             WHERE class_id = ? AND student_id = ? AND term = ? AND skill_code = ?",
            (&class_id, &student_id, term, &skill_code),
        )
        .map_err(|e| HandlerErr {
            code: "db_delete_failed",
            message: e.to_string(),
            details: Some(json!({ "table": "learning_skills_cells" })),
        })?;
        return Ok(json!({ "ok": true }));
    }

    conn.execute(
        "INSERT INTO learning_skills_cells(class_id, student_id, term, skill_code, value, updated_at)
         VALUES(?, ?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%SZ','now'))
         ON CONFLICT(class_id, student_id, term, skill_code) DO UPDATE SET
           value = excluded.value,
           updated_at = excluded.updated_at",
        (&class_id, &student_id, term, &skill_code, &value),
    )
    .map_err(|e| HandlerErr {
        code: "db_update_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "learning_skills_cells" })),
    })?;
    Ok(json!({ "ok": true }))
}

fn learning_skills_report_model(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let mut open = learning_skills_open(conn, params)?;
    let class_id = get_required_str(params, "classId")?;
    let class_name: Option<String> = conn
        .query_row("SELECT name FROM classes WHERE id = ?", [&class_id], |r| {
            r.get(0)
        })
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let Some(class_name) = class_name else {
        return Err(HandlerErr {
            code: "not_found",
            message: "class not found".to_string(),
            details: None,
        });
    };
    let obj = open.as_object_mut().ok_or_else(|| HandlerErr {
        code: "server_error",
        message: "invalid learning skills model".to_string(),
        details: None,
    })?;
    obj.insert(
        "class".to_string(),
        json!({ "id": class_id, "name": class_name }),
    );
    Ok(open)
}

fn handle_loaned_list(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match loaned_list(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_loaned_get(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match loaned_get(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_loaned_update(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match loaned_update(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_devices_list(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match devices_list(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_devices_get(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match devices_get(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_devices_update(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match devices_update(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_learning_skills_open(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match learning_skills_open(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_learning_skills_update_cell(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match learning_skills_update_cell(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_learning_skills_report_model(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match learning_skills_report_model(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "loaned.list" => Some(handle_loaned_list(state, req)),
        "loaned.get" => Some(handle_loaned_get(state, req)),
        "loaned.update" => Some(handle_loaned_update(state, req)),
        "devices.list" => Some(handle_devices_list(state, req)),
        "devices.get" => Some(handle_devices_get(state, req)),
        "devices.update" => Some(handle_devices_update(state, req)),
        "learningSkills.open" => Some(handle_learning_skills_open(state, req)),
        "learningSkills.updateCell" => Some(handle_learning_skills_update_cell(state, req)),
        "learningSkills.reportModel" => Some(handle_learning_skills_report_model(state, req)),
        _ => None,
    }
}

