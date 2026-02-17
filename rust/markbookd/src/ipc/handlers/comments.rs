use crate::ipc::error::{err, ok};
use crate::ipc::types::{AppState, Request};
use crate::legacy;
use rusqlite::types::Value;
use rusqlite::{params_from_iter, Connection, OptionalExtension};
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
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

fn comments_sets_list(conn: &Connection, params: &serde_json::Value) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    let mark_set_id = get_required_str(params, "markSetId")?;
    if !mark_set_exists(conn, &class_id, &mark_set_id)? {
        return Err(HandlerErr {
            code: "not_found",
            message: "mark set not found".to_string(),
            details: None,
        });
    }
    let mut stmt = conn
        .prepare(
            "SELECT set_number, title, fit_mode, fit_font_size, fit_width, fit_lines, fit_subj, max_chars, is_default, bank_short
             FROM comment_set_indexes
             WHERE class_id = ? AND mark_set_id = ?
             ORDER BY set_number",
        )
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let sets = stmt
        .query_map((&class_id, &mark_set_id), |r| {
            Ok(json!({
                "setNumber": r.get::<_, i64>(0)?,
                "title": r.get::<_, String>(1)?,
                "fitMode": r.get::<_, i64>(2)?,
                "fitFontSize": r.get::<_, i64>(3)?,
                "fitWidth": r.get::<_, i64>(4)?,
                "fitLines": r.get::<_, i64>(5)?,
                "fitSubj": r.get::<_, String>(6)?,
                "maxChars": r.get::<_, i64>(7)?,
                "isDefault": r.get::<_, i64>(8)? != 0,
                "bankShort": r.get::<_, Option<String>>(9)?
            }))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    Ok(json!({ "sets": sets }))
}

fn comments_sets_open(conn: &Connection, params: &serde_json::Value) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    let mark_set_id = get_required_str(params, "markSetId")?;
    let set_number = params
        .get("setNumber")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| HandlerErr {
            code: "bad_params",
            message: "missing setNumber".to_string(),
            details: None,
        })?;
    let set_row: Option<(String, i64, String, i64, i64, i64, i64, String, i64, i64, Option<String>)> = conn
        .query_row(
            "SELECT id, set_number, title, fit_mode, fit_font_size, fit_width, fit_lines, fit_subj, max_chars, is_default, bank_short
             FROM comment_set_indexes
             WHERE class_id = ? AND mark_set_id = ? AND set_number = ?",
            (&class_id, &mark_set_id, set_number),
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
                ))
            },
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let Some((
        set_id,
        set_number,
        title,
        fit_mode,
        fit_font_size,
        fit_width,
        fit_lines,
        fit_subj,
        max_chars,
        is_default,
        bank_short,
    )) = set_row
    else {
        return Err(HandlerErr {
            code: "not_found",
            message: "comment set not found".to_string(),
            details: None,
        });
    };

    let students = list_students_for_class(conn, &class_id)?;
    let mut remark_by_student: HashMap<String, String> = HashMap::new();
    let mut stmt = conn
        .prepare("SELECT student_id, remark FROM comment_set_remarks WHERE comment_set_index_id = ?")
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let rows = stmt
        .query_map([&set_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    for (sid, remark) in rows {
        remark_by_student.insert(sid, remark);
    }
    let remarks_by_student: Vec<serde_json::Value> = students
        .iter()
        .map(|s| {
            json!({
                "studentId": s.id,
                "displayName": s.display_name,
                "sortOrder": s.sort_order,
                "active": s.active,
                "remark": remark_by_student.get(&s.id).cloned().unwrap_or_default(),
            })
        })
        .collect();

    Ok(json!({
        "set": {
            "id": set_id,
            "setNumber": set_number,
            "title": title,
            "fitMode": fit_mode,
            "fitFontSize": fit_font_size,
            "fitWidth": fit_width,
            "fitLines": fit_lines,
            "fitSubj": fit_subj,
            "maxChars": max_chars,
            "isDefault": is_default != 0,
            "bankShort": bank_short
        },
        "remarksByStudent": remarks_by_student
    }))
}

fn parse_remarks_by_student(raw: Option<&serde_json::Value>) -> Result<Vec<(String, String)>, HandlerErr> {
    let Some(raw) = raw else {
        return Ok(Vec::new());
    };
    if let Some(arr) = raw.as_array() {
        let mut out = Vec::new();
        for item in arr {
            let Some(student_id) = item.get("studentId").and_then(|v| v.as_str()) else {
                continue;
            };
            let remark = item
                .get("remark")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            out.push((student_id.to_string(), remark));
        }
        return Ok(out);
    }
    if let Some(map) = raw.as_object() {
        let mut out = Vec::new();
        for (student_id, value) in map {
            let remark = value.as_str().unwrap_or("").to_string();
            out.push((student_id.clone(), remark));
        }
        return Ok(out);
    }
    Err(HandlerErr {
        code: "bad_params",
        message: "remarksByStudent must be array or object".to_string(),
        details: None,
    })
}

fn comments_sets_upsert(conn: &Connection, params: &serde_json::Value) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    let mark_set_id = get_required_str(params, "markSetId")?;
    if !mark_set_exists(conn, &class_id, &mark_set_id)? {
        return Err(HandlerErr {
            code: "not_found",
            message: "mark set not found".to_string(),
            details: None,
        });
    }
    let title = params
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Comment Set")
        .trim()
        .to_string();
    let fit_mode = params.get("fitMode").and_then(|v| v.as_i64()).unwrap_or(0);
    let fit_font_size = params.get("fitFontSize").and_then(|v| v.as_i64()).unwrap_or(9);
    let fit_width = params.get("fitWidth").and_then(|v| v.as_i64()).unwrap_or(83);
    let fit_lines = params.get("fitLines").and_then(|v| v.as_i64()).unwrap_or(12);
    let fit_subj = params
        .get("fitSubj")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let max_chars = params
        .get("maxChars")
        .and_then(|v| v.as_i64())
        .unwrap_or(100)
        .max(100);
    let is_default = params.get("isDefault").and_then(|v| v.as_bool()).unwrap_or(false);
    let bank_short = params
        .get("bankShort")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .and_then(|s| if s.is_empty() { None } else { Some(s) });
    let requested_set_number = params.get("setNumber").and_then(|v| v.as_i64());
    let remarks_by_student = parse_remarks_by_student(params.get("remarksByStudent"))?;

    let tx = conn.unchecked_transaction().map_err(|e| HandlerErr {
        code: "db_tx_failed",
        message: e.to_string(),
        details: None,
    })?;

    let set_number = if let Some(v) = requested_set_number {
        v.max(1)
    } else {
        tx.query_row(
            "SELECT COALESCE(MAX(set_number), 0) + 1 FROM comment_set_indexes WHERE mark_set_id = ?",
            [&mark_set_id],
            |r| r.get::<_, i64>(0),
        )
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?
    };

    if is_default {
        tx.execute(
            "UPDATE comment_set_indexes SET is_default = 0 WHERE mark_set_id = ?",
            [&mark_set_id],
        )
        .map_err(|e| HandlerErr {
            code: "db_update_failed",
            message: e.to_string(),
            details: Some(json!({ "table": "comment_set_indexes" })),
        })?;
    }

    let existing_id: Option<String> = tx
        .query_row(
            "SELECT id FROM comment_set_indexes WHERE mark_set_id = ? AND set_number = ?",
            (&mark_set_id, set_number),
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let set_id = existing_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    tx.execute(
        "INSERT INTO comment_set_indexes(
           id, class_id, mark_set_id, set_number, title, fit_mode, fit_font_size, fit_width, fit_lines, fit_subj, max_chars, is_default, bank_short
         ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(mark_set_id, set_number) DO UPDATE SET
           title = excluded.title,
           fit_mode = excluded.fit_mode,
           fit_font_size = excluded.fit_font_size,
           fit_width = excluded.fit_width,
           fit_lines = excluded.fit_lines,
           fit_subj = excluded.fit_subj,
           max_chars = excluded.max_chars,
           is_default = excluded.is_default,
           bank_short = excluded.bank_short",
        (
            &set_id,
            &class_id,
            &mark_set_id,
            set_number,
            &title,
            fit_mode,
            fit_font_size,
            fit_width,
            fit_lines,
            &fit_subj,
            max_chars,
            if is_default { 1 } else { 0 },
            bank_short.as_deref(),
        ),
    )
    .map_err(|e| HandlerErr {
        code: "db_insert_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "comment_set_indexes" })),
    })?;

    for (student_id, remark) in remarks_by_student {
        let trimmed = remark.trim().to_string();
        if trimmed.is_empty() {
            tx.execute(
                "DELETE FROM comment_set_remarks WHERE comment_set_index_id = ? AND student_id = ?",
                (&set_id, &student_id),
            )
            .map_err(|e| HandlerErr {
                code: "db_delete_failed",
                message: e.to_string(),
                details: Some(json!({ "table": "comment_set_remarks" })),
            })?;
            continue;
        }
        let remark_id = Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO comment_set_remarks(id, comment_set_index_id, student_id, remark)
             VALUES(?, ?, ?, ?)
             ON CONFLICT(comment_set_index_id, student_id) DO UPDATE SET
               remark = excluded.remark",
            (&remark_id, &set_id, &student_id, &trimmed),
        )
        .map_err(|e| HandlerErr {
            code: "db_insert_failed",
            message: e.to_string(),
            details: Some(json!({ "table": "comment_set_remarks" })),
        })?;
    }

    tx.commit().map_err(|e| HandlerErr {
        code: "db_commit_failed",
        message: e.to_string(),
        details: None,
    })?;
    Ok(json!({ "setNumber": set_number }))
}

fn comments_sets_delete(conn: &Connection, params: &serde_json::Value) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    let mark_set_id = get_required_str(params, "markSetId")?;
    let set_number = params
        .get("setNumber")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| HandlerErr {
            code: "bad_params",
            message: "missing setNumber".to_string(),
            details: None,
        })?;
    let tx = conn.unchecked_transaction().map_err(|e| HandlerErr {
        code: "db_tx_failed",
        message: e.to_string(),
        details: None,
    })?;
    let set_id: Option<String> = tx
        .query_row(
            "SELECT id FROM comment_set_indexes WHERE class_id = ? AND mark_set_id = ? AND set_number = ?",
            (&class_id, &mark_set_id, set_number),
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let Some(set_id) = set_id else {
        return Err(HandlerErr {
            code: "not_found",
            message: "comment set not found".to_string(),
            details: None,
        });
    };
    tx.execute(
        "DELETE FROM comment_set_remarks WHERE comment_set_index_id = ?",
        [&set_id],
    )
    .map_err(|e| HandlerErr {
        code: "db_delete_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "comment_set_remarks" })),
    })?;
    tx.execute("DELETE FROM comment_set_indexes WHERE id = ?", [&set_id])
        .map_err(|e| HandlerErr {
            code: "db_delete_failed",
            message: e.to_string(),
            details: Some(json!({ "table": "comment_set_indexes" })),
        })?;
    tx.commit().map_err(|e| HandlerErr {
        code: "db_commit_failed",
        message: e.to_string(),
        details: None,
    })?;
    Ok(json!({ "ok": true }))
}

fn comments_banks_list(conn: &Connection) -> Result<serde_json::Value, HandlerErr> {
    let mut stmt = conn
        .prepare(
            "SELECT
               b.id,
               b.short_name,
               b.is_default,
               b.fit_profile,
               b.source_path,
               (SELECT COUNT(*) FROM comment_bank_entries e WHERE e.bank_id = b.id) AS entry_count
             FROM comment_banks b
             ORDER BY b.short_name",
        )
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let banks = stmt
        .query_map([], |r| {
            Ok(json!({
                "id": r.get::<_, String>(0)?,
                "shortName": r.get::<_, String>(1)?,
                "isDefault": r.get::<_, i64>(2)? != 0,
                "fitProfile": r.get::<_, Option<String>>(3)?,
                "sourcePath": r.get::<_, Option<String>>(4)?,
                "entryCount": r.get::<_, i64>(5)?
            }))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    Ok(json!({ "banks": banks }))
}

fn comments_banks_open(conn: &Connection, params: &serde_json::Value) -> Result<serde_json::Value, HandlerErr> {
    let bank_id = get_required_str(params, "bankId")?;
    let bank: Option<serde_json::Value> = conn
        .query_row(
            "SELECT id, short_name, is_default, fit_profile, source_path FROM comment_banks WHERE id = ?",
            [&bank_id],
            |r| {
                Ok(json!({
                    "id": r.get::<_, String>(0)?,
                    "shortName": r.get::<_, String>(1)?,
                    "isDefault": r.get::<_, i64>(2)? != 0,
                    "fitProfile": r.get::<_, Option<String>>(3)?,
                    "sourcePath": r.get::<_, Option<String>>(4)?
                }))
            },
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let Some(bank) = bank else {
        return Err(HandlerErr {
            code: "not_found",
            message: "bank not found".to_string(),
            details: None,
        });
    };
    let mut stmt = conn
        .prepare(
            "SELECT id, sort_order, type_code, level_code, text
             FROM comment_bank_entries
             WHERE bank_id = ?
             ORDER BY sort_order",
        )
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let entries = stmt
        .query_map([&bank_id], |r| {
            Ok(json!({
                "id": r.get::<_, String>(0)?,
                "sortOrder": r.get::<_, i64>(1)?,
                "typeCode": r.get::<_, String>(2)?,
                "levelCode": r.get::<_, String>(3)?,
                "text": r.get::<_, String>(4)?,
            }))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    Ok(json!({ "bank": bank, "entries": entries }))
}

fn comments_banks_create(conn: &Connection, params: &serde_json::Value) -> Result<serde_json::Value, HandlerErr> {
    let short_name = get_required_str(params, "shortName")?.trim().to_string();
    if short_name.is_empty() {
        return Err(HandlerErr {
            code: "bad_params",
            message: "shortName must not be empty".to_string(),
            details: None,
        });
    }
    let bank_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO comment_banks(id, short_name, is_default, fit_profile, source_path)
         VALUES(?, ?, 0, NULL, NULL)",
        (&bank_id, &short_name),
    )
    .map_err(|e| HandlerErr {
        code: "db_insert_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "comment_banks" })),
    })?;
    Ok(json!({ "bankId": bank_id }))
}

fn comments_banks_update_meta(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let bank_id = get_required_str(params, "bankId")?;
    let Some(patch) = params.get("patch").and_then(|v| v.as_object()) else {
        return Err(HandlerErr {
            code: "bad_params",
            message: "missing patch".to_string(),
            details: None,
        });
    };
    let tx = conn.unchecked_transaction().map_err(|e| HandlerErr {
        code: "db_tx_failed",
        message: e.to_string(),
        details: None,
    })?;
    if patch
        .get("isDefault")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        tx.execute("UPDATE comment_banks SET is_default = 0", [])
            .map_err(|e| HandlerErr {
                code: "db_update_failed",
                message: e.to_string(),
                details: Some(json!({ "table": "comment_banks" })),
            })?;
    }

    let mut set_parts: Vec<String> = Vec::new();
    let mut binds: Vec<Value> = Vec::new();
    if let Some(v) = patch.get("shortName") {
        let Some(s) = v.as_str() else {
            return Err(HandlerErr {
                code: "bad_params",
                message: "patch.shortName must be string".to_string(),
                details: None,
            });
        };
        set_parts.push("short_name = ?".into());
        binds.push(Value::Text(s.trim().to_string()));
    }
    if let Some(v) = patch.get("isDefault") {
        let Some(b) = v.as_bool() else {
            return Err(HandlerErr {
                code: "bad_params",
                message: "patch.isDefault must be boolean".to_string(),
                details: None,
            });
        };
        set_parts.push("is_default = ?".into());
        binds.push(Value::Integer(if b { 1 } else { 0 }));
    }
    if let Some(v) = patch.get("fitProfile") {
        set_parts.push("fit_profile = ?".into());
        if v.is_null() {
            binds.push(Value::Null);
        } else if let Some(s) = v.as_str() {
            binds.push(Value::Text(s.to_string()));
        } else {
            return Err(HandlerErr {
                code: "bad_params",
                message: "patch.fitProfile must be string|null".to_string(),
                details: None,
            });
        }
    }
    if let Some(v) = patch.get("sourcePath") {
        set_parts.push("source_path = ?".into());
        if v.is_null() {
            binds.push(Value::Null);
        } else if let Some(s) = v.as_str() {
            binds.push(Value::Text(s.to_string()));
        } else {
            return Err(HandlerErr {
                code: "bad_params",
                message: "patch.sourcePath must be string|null".to_string(),
                details: None,
            });
        }
    }

    if !set_parts.is_empty() {
        let sql = format!("UPDATE comment_banks SET {} WHERE id = ?", set_parts.join(", "));
        binds.push(Value::Text(bank_id.clone()));
        tx.execute(&sql, params_from_iter(binds))
            .map_err(|e| HandlerErr {
                code: "db_update_failed",
                message: e.to_string(),
                details: Some(json!({ "table": "comment_banks" })),
            })?;
    }
    tx.commit().map_err(|e| HandlerErr {
        code: "db_commit_failed",
        message: e.to_string(),
        details: None,
    })?;
    Ok(json!({ "ok": true }))
}

fn comments_banks_entry_upsert(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let bank_id = get_required_str(params, "bankId")?;
    let type_code = get_required_str(params, "typeCode")?;
    let level_code = get_required_str(params, "levelCode")?;
    let text = get_required_str(params, "text")?;
    let requested_sort = params.get("sortOrder").and_then(|v| v.as_i64());
    let entry_id = params
        .get("entryId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let tx = conn.unchecked_transaction().map_err(|e| HandlerErr {
        code: "db_tx_failed",
        message: e.to_string(),
        details: None,
    })?;

    let resolved_entry_id = entry_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let existing_sort: Option<i64> = tx
        .query_row(
            "SELECT sort_order FROM comment_bank_entries WHERE id = ? AND bank_id = ?",
            (&resolved_entry_id, &bank_id),
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;

    let mut target_sort = requested_sort.unwrap_or_else(|| {
        tx.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM comment_bank_entries WHERE bank_id = ?",
            [&bank_id],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
    });
    if target_sort < 0 {
        target_sort = 0;
    }

    if let Some(cur_sort) = existing_sort {
        if target_sort != cur_sort {
            if target_sort > cur_sort {
                tx.execute(
                    "UPDATE comment_bank_entries
                     SET sort_order = sort_order - 1
                     WHERE bank_id = ? AND sort_order > ? AND sort_order <= ?",
                    (&bank_id, cur_sort, target_sort),
                )
                .map_err(|e| HandlerErr {
                    code: "db_update_failed",
                    message: e.to_string(),
                    details: Some(json!({ "table": "comment_bank_entries" })),
                })?;
            } else {
                tx.execute(
                    "UPDATE comment_bank_entries
                     SET sort_order = sort_order + 1
                     WHERE bank_id = ? AND sort_order >= ? AND sort_order < ?",
                    (&bank_id, target_sort, cur_sort),
                )
                .map_err(|e| HandlerErr {
                    code: "db_update_failed",
                    message: e.to_string(),
                    details: Some(json!({ "table": "comment_bank_entries" })),
                })?;
            }
        }
    } else {
        tx.execute(
            "UPDATE comment_bank_entries
             SET sort_order = sort_order + 1
             WHERE bank_id = ? AND sort_order >= ?",
            (&bank_id, target_sort),
        )
        .map_err(|e| HandlerErr {
            code: "db_update_failed",
            message: e.to_string(),
            details: Some(json!({ "table": "comment_bank_entries" })),
        })?;
    }

    tx.execute(
        "INSERT INTO comment_bank_entries(id, bank_id, sort_order, type_code, level_code, text)
         VALUES(?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
           sort_order = excluded.sort_order,
           type_code = excluded.type_code,
           level_code = excluded.level_code,
           text = excluded.text",
        (
            &resolved_entry_id,
            &bank_id,
            target_sort,
            &type_code,
            &level_code,
            &text,
        ),
    )
    .map_err(|e| HandlerErr {
        code: "db_insert_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "comment_bank_entries" })),
    })?;

    tx.commit().map_err(|e| HandlerErr {
        code: "db_commit_failed",
        message: e.to_string(),
        details: None,
    })?;
    Ok(json!({ "entryId": resolved_entry_id }))
}

fn comments_banks_entry_delete(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let bank_id = get_required_str(params, "bankId")?;
    let entry_id = get_required_str(params, "entryId")?;
    let tx = conn.unchecked_transaction().map_err(|e| HandlerErr {
        code: "db_tx_failed",
        message: e.to_string(),
        details: None,
    })?;
    let sort_order: Option<i64> = tx
        .query_row(
            "SELECT sort_order FROM comment_bank_entries WHERE id = ? AND bank_id = ?",
            (&entry_id, &bank_id),
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let Some(sort_order) = sort_order else {
        return Err(HandlerErr {
            code: "not_found",
            message: "entry not found".to_string(),
            details: None,
        });
    };
    tx.execute(
        "DELETE FROM comment_bank_entries WHERE id = ? AND bank_id = ?",
        (&entry_id, &bank_id),
    )
    .map_err(|e| HandlerErr {
        code: "db_delete_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "comment_bank_entries" })),
    })?;
    tx.execute(
        "UPDATE comment_bank_entries
         SET sort_order = sort_order - 1
         WHERE bank_id = ? AND sort_order > ?",
        (&bank_id, sort_order),
    )
    .map_err(|e| HandlerErr {
        code: "db_update_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "comment_bank_entries" })),
    })?;
    tx.commit().map_err(|e| HandlerErr {
        code: "db_commit_failed",
        message: e.to_string(),
        details: None,
    })?;
    Ok(json!({ "ok": true }))
}

fn comments_banks_import_bnk(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let path = get_required_str(params, "path")?;
    let file_path = PathBuf::from(&path);
    let short_name = file_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| HandlerErr {
            code: "bad_params",
            message: "invalid path".to_string(),
            details: None,
        })?
        .to_string();
    let parsed = legacy::parse_bnk_file(&file_path).map_err(|e| HandlerErr {
        code: "legacy_parse_failed",
        message: e.to_string(),
        details: Some(json!({ "path": path })),
    })?;

    let tx = conn.unchecked_transaction().map_err(|e| HandlerErr {
        code: "db_tx_failed",
        message: e.to_string(),
        details: None,
    })?;
    let new_id = Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO comment_banks(id, short_name, is_default, fit_profile, source_path)
         VALUES(?, ?, 0, ?, ?)
         ON CONFLICT(short_name) DO UPDATE SET
           fit_profile = excluded.fit_profile,
           source_path = excluded.source_path",
        (&new_id, &short_name, parsed.fit_profile.as_deref(), &path),
    )
    .map_err(|e| HandlerErr {
        code: "db_insert_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "comment_banks" })),
    })?;
    let bank_id: String = tx
        .query_row(
            "SELECT id FROM comment_banks WHERE short_name = ?",
            [&short_name],
            |r| r.get(0),
        )
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    tx.execute("DELETE FROM comment_bank_entries WHERE bank_id = ?", [&bank_id])
        .map_err(|e| HandlerErr {
            code: "db_delete_failed",
            message: e.to_string(),
            details: Some(json!({ "table": "comment_bank_entries" })),
        })?;
    for (sort_order, entry) in parsed.entries.iter().enumerate() {
        let eid = Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO comment_bank_entries(id, bank_id, sort_order, type_code, level_code, text)
             VALUES(?, ?, ?, ?, ?, ?)",
            (
                &eid,
                &bank_id,
                sort_order as i64,
                &entry.type_code,
                &entry.level_code,
                &entry.text,
            ),
        )
        .map_err(|e| HandlerErr {
            code: "db_insert_failed",
            message: e.to_string(),
            details: Some(json!({ "table": "comment_bank_entries" })),
        })?;
    }
    tx.commit().map_err(|e| HandlerErr {
        code: "db_commit_failed",
        message: e.to_string(),
        details: None,
    })?;
    Ok(json!({ "bankId": bank_id }))
}

fn comments_banks_export_bnk(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let bank_id = get_required_str(params, "bankId")?;
    let out_path = get_required_str(params, "path")?;
    let bank_meta: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT short_name, fit_profile FROM comment_banks WHERE id = ?",
            [&bank_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let Some((_short_name, fit_profile)) = bank_meta else {
        return Err(HandlerErr {
            code: "not_found",
            message: "bank not found".to_string(),
            details: None,
        });
    };
    let mut stmt = conn
        .prepare(
            "SELECT sort_order, type_code, level_code, text
             FROM comment_bank_entries
             WHERE bank_id = ?
             ORDER BY sort_order",
        )
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let entries = stmt
        .query_map([&bank_id], |r| {
            Ok(legacy::ParsedBnkEntry {
                sort_order: r.get::<_, i64>(0)? as usize,
                type_code: r.get(1)?,
                level_code: r.get(2)?,
                text: r.get(3)?,
            })
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let text = legacy::serialize_bnk_file(&legacy::ParsedBnkFile { fit_profile, entries });
    let out = PathBuf::from(&out_path);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| HandlerErr {
            code: "io_failed",
            message: e.to_string(),
            details: Some(json!({ "path": out_path })),
        })?;
    }
    std::fs::write(&out, text).map_err(|e| HandlerErr {
        code: "io_failed",
        message: e.to_string(),
        details: Some(json!({ "path": out_path })),
    })?;
    Ok(json!({ "ok": true }))
}

fn handle_comments_sets_list(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_sets_list(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_comments_sets_open(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_sets_open(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_comments_sets_upsert(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_sets_upsert(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_comments_sets_delete(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_sets_delete(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_comments_banks_list(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_banks_list(conn) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_comments_banks_open(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_banks_open(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_comments_banks_create(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_banks_create(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_comments_banks_update_meta(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_banks_update_meta(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_comments_banks_entry_upsert(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_banks_entry_upsert(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_comments_banks_entry_delete(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_banks_entry_delete(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_comments_banks_import_bnk(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_banks_import_bnk(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_comments_banks_export_bnk(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_banks_export_bnk(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "comments.sets.list" => Some(handle_comments_sets_list(state, req)),
        "comments.sets.open" => Some(handle_comments_sets_open(state, req)),
        "comments.sets.upsert" => Some(handle_comments_sets_upsert(state, req)),
        "comments.sets.delete" => Some(handle_comments_sets_delete(state, req)),
        "comments.banks.list" => Some(handle_comments_banks_list(state, req)),
        "comments.banks.open" => Some(handle_comments_banks_open(state, req)),
        "comments.banks.create" => Some(handle_comments_banks_create(state, req)),
        "comments.banks.updateMeta" => Some(handle_comments_banks_update_meta(state, req)),
        "comments.banks.entryUpsert" => Some(handle_comments_banks_entry_upsert(state, req)),
        "comments.banks.entryDelete" => Some(handle_comments_banks_entry_delete(state, req)),
        "comments.banks.importBnk" => Some(handle_comments_banks_import_bnk(state, req)),
        "comments.banks.exportBnk" => Some(handle_comments_banks_export_bnk(state, req)),
        _ => None,
    }
}

