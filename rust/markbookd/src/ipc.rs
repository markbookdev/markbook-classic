use crate::{calc, db, legacy};
use rusqlite::types::Value;
use rusqlite::{params_from_iter, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct Request {
    pub id: String,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct OkResp {
    id: String,
    ok: bool,
    result: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct ErrObj {
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct ErrResp {
    id: String,
    ok: bool,
    error: ErrObj,
}

pub struct AppState {
    pub workspace: Option<PathBuf>,
    pub db: Option<Connection>,
}

pub fn handle_request(state: &mut AppState, req: Request) -> serde_json::Value {
    match req.method.as_str() {
        "health" => {
            let result = json!({
                "version": env!("CARGO_PKG_VERSION"),
                "workspacePath": state.workspace.as_ref().map(|p| p.to_string_lossy().to_string())
            });
            json!(OkResp {
                id: req.id,
                ok: true,
                result
            })
        }
        "workspace.select" => {
            let p = req
                .params
                .get("path")
                .and_then(|v| v.as_str())
                .map(PathBuf::from);
            let Some(path) = p else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "bad_params".into(),
                        message: "missing params.path".into(),
                        details: None
                    }
                });
            };

            match db::open_db(&path) {
                Ok(conn) => {
                    state.workspace = Some(path.clone());
                    state.db = Some(conn);
                    json!(OkResp {
                        id: req.id,
                        ok: true,
                        result: json!({ "workspacePath": path.to_string_lossy() })
                    })
                }
                Err(e) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_open_failed".into(),
                        message: format!("{e:?}"),
                        details: None
                    }
                }),
            }
        }
        "classes.list" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(OkResp {
                    id: req.id,
                    ok: true,
                    result: json!({ "classes": [] })
                });
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
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
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
                Ok(classes) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result: json!({ "classes": classes })
                }),
                Err(e) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_query_failed".into(),
                        message: e.to_string(),
                        details: None
                    }
                }),
            }
        }
        "classes.create" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let name = match req.params.get("name").and_then(|v| v.as_str()) {
                Some(v) => v.trim().to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing name".into(),
                            details: None
                        }
                    })
                }
            };
            if name.is_empty() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "bad_params".into(),
                        message: "name must not be empty".into(),
                        details: None
                    }
                });
            }

            let class_id = Uuid::new_v4().to_string();
            if let Err(e) = conn.execute(
                "INSERT INTO classes(id, name) VALUES(?, ?)",
                (&class_id, &name),
            ) {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_insert_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "classes" }))
                    }
                });
            }

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({ "classId": class_id, "name": name })
            })
        }
        "classes.delete" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };

            let exists: Option<i64> = match conn
                .query_row("SELECT 1 FROM classes WHERE id = ?", [&class_id], |r| {
                    r.get(0)
                })
                .optional()
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };

            if exists.is_none() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "class not found".into(),
                        details: None
                    }
                });
            }

            let tx = match conn.unchecked_transaction() {
                Ok(t) => t,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_tx_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
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
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "scores" }))
                    }
                });
            }

            if let Err(e) = tx.execute(
                "DELETE FROM assessments
                 WHERE mark_set_id IN (SELECT id FROM mark_sets WHERE class_id = ?)",
                [&class_id],
            ) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "assessments" }))
                    }
                });
            }

            if let Err(e) = tx.execute(
                "DELETE FROM categories
                 WHERE mark_set_id IN (SELECT id FROM mark_sets WHERE class_id = ?)",
                [&class_id],
            ) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "categories" }))
                    }
                });
            }

            if let Err(e) = tx.execute("DELETE FROM mark_sets WHERE class_id = ?", [&class_id]) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "mark_sets" }))
                    }
                });
            }

            if let Err(e) = tx.execute("DELETE FROM student_notes WHERE class_id = ?", [&class_id])
            {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "student_notes" }))
                    }
                });
            }

            if let Err(e) = tx.execute("DELETE FROM students WHERE class_id = ?", [&class_id]) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "students" }))
                    }
                });
            }

            if let Err(e) = tx.execute("DELETE FROM classes WHERE id = ?", [&class_id]) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "classes" }))
                    }
                });
            }

            if let Err(e) = tx.commit() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_commit_failed".into(),
                        message: e.to_string(),
                        details: None
                    }
                });
            }

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({ "ok": true })
            })
        }
        "students.list" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };

            let mut stmt = match conn.prepare(
                "SELECT id, last_name, first_name, student_no, birth_date, active, sort_order
                 FROM students
                 WHERE class_id = ?
                 ORDER BY sort_order",
            ) {
                Ok(s) => s,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
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
                Ok(students) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result: json!({ "students": students })
                }),
                Err(e) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_query_failed".into(),
                        message: e.to_string(),
                        details: None
                    }
                }),
            }
        }
        "students.create" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };

            let last_name = match req.params.get("lastName").and_then(|v| v.as_str()) {
                Some(v) => v.trim().to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing lastName".into(),
                            details: None
                        }
                    })
                }
            };
            let first_name = match req.params.get("firstName").and_then(|v| v.as_str()) {
                Some(v) => v.trim().to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing firstName".into(),
                            details: None
                        }
                    })
                }
            };
            if last_name.is_empty() || first_name.is_empty() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "bad_params".into(),
                        message: "firstName/lastName must not be empty".into(),
                        details: None
                    }
                });
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
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            if class_exists.is_none() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "class not found".into(),
                        details: None
                    }
                });
            }

            let sort_order: i64 = match conn.query_row(
                "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM students WHERE class_id = ?",
                [&class_id],
                |r| r.get(0),
            ) {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
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
                   updated_at
                 ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%SZ','now'))",
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
                ),
            ) {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_insert_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "students" }))
                    }
                });
            }

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({ "studentId": student_id })
            })
        }
        "students.update" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };
            let student_id = match req.params.get("studentId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing studentId".into(),
                            details: None
                        }
                    })
                }
            };
            let Some(patch) = req.params.get("patch").and_then(|v| v.as_object()) else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "bad_params".into(),
                        message: "missing/invalid patch".into(),
                        details: None
                    }
                });
            };

            let mut set_parts: Vec<String> = Vec::new();
            let mut bind_values: Vec<Value> = Vec::new();

            if let Some(v) = patch.get("lastName") {
                let Some(s) = v.as_str() else {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "patch.lastName must be a string".into(),
                            details: None
                        }
                    });
                };
                let s = s.trim().to_string();
                if s.is_empty() {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "lastName must not be empty".into(),
                            details: None
                        }
                    });
                }
                set_parts.push("last_name = ?".into());
                bind_values.push(Value::Text(s));
            }
            if let Some(v) = patch.get("firstName") {
                let Some(s) = v.as_str() else {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "patch.firstName must be a string".into(),
                            details: None
                        }
                    });
                };
                let s = s.trim().to_string();
                if s.is_empty() {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "firstName must not be empty".into(),
                            details: None
                        }
                    });
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
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "patch.studentNo must be a string or null".into(),
                            details: None
                        }
                    });
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
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "patch.birthDate must be a string or null".into(),
                            details: None
                        }
                    });
                }
            }
            if let Some(v) = patch.get("active") {
                let Some(b) = v.as_bool() else {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "patch.active must be a boolean".into(),
                            details: None
                        }
                    });
                };
                set_parts.push("active = ?".into());
                bind_values.push(Value::Integer(if b { 1 } else { 0 }));
            }

            if set_parts.is_empty() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "bad_params".into(),
                        message: "patch must include at least one field".into(),
                        details: None
                    }
                });
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
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_update_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "students" }))
                        }
                    })
                }
            };
            if changed == 0 {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "student not found".into(),
                        details: None
                    }
                });
            }

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({ "ok": true })
            })
        }
        "students.reorder" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };

            let Some(arr) = req
                .params
                .get("orderedStudentIds")
                .and_then(|v| v.as_array())
            else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "bad_params".into(),
                        message: "missing/invalid orderedStudentIds".into(),
                        details: None
                    }
                });
            };
            let mut ordered: Vec<String> = Vec::with_capacity(arr.len());
            for v in arr {
                let Some(s) = v.as_str() else {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "orderedStudentIds must be strings".into(),
                            details: None
                        }
                    });
                };
                ordered.push(s.to_string());
            }

            let mut stmt = match conn
                .prepare("SELECT id FROM students WHERE class_id = ? ORDER BY sort_order")
            {
                Ok(s) => s,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            let current_ids: Vec<String> = match stmt
                .query_map([&class_id], |row| row.get::<_, String>(0))
                .and_then(|it| it.collect::<Result<Vec<_>, _>>())
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };

            if ordered.len() != current_ids.len() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "bad_params".into(),
                        message: "orderedStudentIds must be a permutation of the class students"
                            .into(),
                        details: Some(
                            json!({ "expected": current_ids.len(), "got": ordered.len() })
                        )
                    }
                });
            }

            let current_set: HashSet<String> = current_ids.into_iter().collect();
            let mut seen: HashSet<String> = HashSet::new();
            for id in &ordered {
                if !seen.insert(id.clone()) {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "orderedStudentIds contains duplicates".into(),
                            details: Some(json!({ "studentId": id }))
                        }
                    });
                }
                if !current_set.contains(id) {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "orderedStudentIds contains unknown studentId".into(),
                            details: Some(json!({ "studentId": id }))
                        }
                    });
                }
            }
            if seen.len() != current_set.len() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "bad_params".into(),
                        message: "orderedStudentIds must include every student exactly once".into(),
                        details: None
                    }
                });
            }

            let tx = match conn.unchecked_transaction() {
                Ok(t) => t,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_tx_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };

            for (i, sid) in ordered.iter().enumerate() {
                if let Err(e) = tx.execute(
                    "UPDATE students
                     SET sort_order = ?, updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
                     WHERE id = ? AND class_id = ?",
                    (i as i64, sid, &class_id),
                ) {
                    let _ = tx.rollback();
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_update_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "students" }))
                        }
                    });
                }
            }

            if let Err(e) = tx.commit() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_commit_failed".into(),
                        message: e.to_string(),
                        details: None
                    }
                });
            }

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({ "ok": true })
            })
        }
        "students.delete" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };
            let student_id = match req.params.get("studentId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing studentId".into(),
                            details: None
                        }
                    })
                }
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
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            let Some(sort_order) = sort_order else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "student not found".into(),
                        details: None
                    }
                });
            };

            let tx = match conn.unchecked_transaction() {
                Ok(t) => t,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_tx_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };

            if let Err(e) = tx.execute("DELETE FROM scores WHERE student_id = ?", [&student_id]) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "scores" }))
                    }
                });
            }

            if let Err(e) = tx.execute(
                "DELETE FROM student_notes WHERE class_id = ? AND student_id = ?",
                (&class_id, &student_id),
            ) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "student_notes" }))
                    }
                });
            }

            let changed = match tx.execute(
                "DELETE FROM students WHERE id = ? AND class_id = ?",
                (&student_id, &class_id),
            ) {
                Ok(v) => v,
                Err(e) => {
                    let _ = tx.rollback();
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_delete_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "students" }))
                        }
                    });
                }
            };
            if changed == 0 {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "student not found".into(),
                        details: None
                    }
                });
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
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_update_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "students" }))
                    }
                });
            }

            if let Err(e) = tx.commit() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_commit_failed".into(),
                        message: e.to_string(),
                        details: None
                    }
                });
            }

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({ "ok": true })
            })
        }
        "notes.get" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };

            let class_exists: Option<i64> = match conn
                .query_row("SELECT 1 FROM classes WHERE id = ?", [&class_id], |r| {
                    r.get(0)
                })
                .optional()
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            if class_exists.is_none() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "class not found".into(),
                        details: None
                    }
                });
            }

            let mut stmt = match conn
                .prepare("SELECT student_id, note FROM student_notes WHERE class_id = ?")
            {
                Ok(s) => s,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };

            let rows = stmt
                .query_map([&class_id], |row| {
                    let student_id: String = row.get(0)?;
                    let note: String = row.get(1)?;
                    Ok(json!({ "studentId": student_id, "note": note }))
                })
                .and_then(|it| it.collect::<Result<Vec<_>, _>>());

            match rows {
                Ok(notes) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result: json!({ "notes": notes })
                }),
                Err(e) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_query_failed".into(),
                        message: e.to_string(),
                        details: None
                    }
                }),
            }
        }
        "notes.update" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };
            let student_id = match req.params.get("studentId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing studentId".into(),
                            details: None
                        }
                    })
                }
            };
            let note = match req.params.get("note").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing note".into(),
                            details: None
                        }
                    })
                }
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
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            if student_exists.is_none() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "student not found".into(),
                        details: None
                    }
                });
            }

            let trimmed = note.trim().to_string();
            if trimmed.is_empty() {
                if let Err(e) = conn.execute(
                    "DELETE FROM student_notes WHERE class_id = ? AND student_id = ?",
                    (&class_id, &student_id),
                ) {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_delete_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "student_notes" }))
                        }
                    });
                }
                return json!(OkResp {
                    id: req.id,
                    ok: true,
                    result: json!({ "ok": true })
                });
            }

            let note_id = Uuid::new_v4().to_string();
            if let Err(e) = conn.execute(
                "INSERT INTO student_notes(id, class_id, student_id, note)
                 VALUES(?, ?, ?, ?)
                 ON CONFLICT(class_id, student_id) DO UPDATE SET
                   note = excluded.note",
                (&note_id, &class_id, &student_id, &note),
            ) {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_insert_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "student_notes" }))
                    }
                });
            }

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({ "ok": true })
            })
        }
        "class.importLegacy" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let legacy_folder = req
                .params
                .get("legacyClassFolderPath")
                .and_then(|v| v.as_str())
                .map(PathBuf::from);

            let Some(legacy_folder) = legacy_folder else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "bad_params".into(),
                        message: "missing legacyClassFolderPath".into(),
                        details: None
                    }
                });
            };

            let cl_file = match legacy::find_cl_file(&legacy_folder) {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "legacy_no_cl".into(),
                            message: e.to_string(),
                            details: Some(json!({ "folder": legacy_folder.to_string_lossy() }))
                        }
                    })
                }
            };

            let parsed = match legacy::parse_legacy_cl(&cl_file) {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "legacy_parse_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "clFile": cl_file.to_string_lossy() }))
                        }
                    })
                }
            };

            let class_id = Uuid::new_v4().to_string();
            let class_name = parsed.class_name;

            let tx = match conn.unchecked_transaction() {
                Ok(t) => t,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_tx_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };

            if let Err(e) = tx.execute(
                "INSERT INTO classes(id, name) VALUES(?, ?)",
                [&class_id, &class_name],
            ) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_insert_failed".into(),
                        message: e.to_string(),
                        details: None
                    }
                });
            }

            let mut imported = 0usize;
            let mut student_ids_by_sort: Vec<String> = Vec::new();
            for (sort_order, s) in parsed.students.into_iter().enumerate() {
                let sid = Uuid::new_v4().to_string();
                let active_i = if s.active { 1 } else { 0 };
                let student_no = s.student_no.unwrap_or_default();
                let birth_date = s.birth_date.unwrap_or_default();
                let res = tx.execute(
                    "INSERT INTO students(id, class_id, last_name, first_name, student_no, birth_date, active, sort_order, raw_line)
                     VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    (
                        &sid,
                        &class_id,
                        &s.last_name,
                        &s.first_name,
                        &student_no,
                        &birth_date,
                        active_i,
                        sort_order as i64,
                        &s.raw_line,
                    ),
                );
                if res.is_ok() {
                    imported += 1;
                    student_ids_by_sort.push(sid);
                }
            }

            // Best-effort import class-level student notes (*NOTE.TXT).
            if let Some(note_file) = match legacy::find_note_file(&legacy_folder) {
                Ok(v) => v,
                Err(e) => {
                    let _ = tx.rollback();
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "legacy_read_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "folder": legacy_folder.to_string_lossy() }))
                        }
                    });
                }
            } {
                let notes = match legacy::parse_legacy_note_file(&note_file) {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = tx.rollback();
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "legacy_parse_failed".into(),
                                message: e.to_string(),
                                details: Some(json!({ "noteFile": note_file.to_string_lossy() }))
                            }
                        });
                    }
                };

                let mut ins = match tx.prepare(
                    "INSERT INTO student_notes(id, class_id, student_id, note)
                     VALUES(?, ?, ?, ?)
                     ON CONFLICT(class_id, student_id) DO UPDATE SET
                       note = excluded.note",
                ) {
                    Ok(s) => s,
                    Err(e) => {
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "db_insert_failed".into(),
                                message: e.to_string(),
                                details: Some(json!({ "table": "student_notes" }))
                            }
                        });
                    }
                };

                let max = std::cmp::min(notes.len(), student_ids_by_sort.len());
                for s_idx in 0..max {
                    let note = notes[s_idx].trim().to_string();
                    if note.is_empty() {
                        continue;
                    }
                    let nid = Uuid::new_v4().to_string();
                    let student_id = &student_ids_by_sort[s_idx];
                    if let Err(e) = ins.execute((&nid, &class_id, student_id, &note)) {
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "db_insert_failed".into(),
                                message: e.to_string(),
                                details: Some(json!({ "table": "student_notes" }))
                            }
                        });
                    }
                }
            }

            let mut mark_sets_imported = 0usize;
            let mut assessments_imported = 0usize;
            let mut scores_imported = 0usize;
            let mut imported_mark_files: Vec<String> = Vec::new();
            let mut missing_mark_files: Vec<serde_json::Value> = Vec::new();

            for def in &parsed.mark_sets {
                let mark_file = match legacy::find_mark_file(&legacy_folder, &def.file_prefix) {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = tx.rollback();
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "legacy_read_failed".into(),
                                message: e.to_string(),
                                details: Some(
                                    json!({ "folder": legacy_folder.to_string_lossy(), "filePrefix": def.file_prefix })
                                )
                            }
                        });
                    }
                };

                let Some(mark_file) = mark_file else {
                    missing_mark_files
                        .push(json!({ "code": def.code, "filePrefix": def.file_prefix }));
                    continue;
                };

                let parsed_mark = match legacy::parse_legacy_mark_file(&mark_file) {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = tx.rollback();
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "legacy_parse_failed".into(),
                                message: e.to_string(),
                                details: Some(json!({ "markFile": mark_file.to_string_lossy() }))
                            }
                        });
                    }
                };

                let mark_filename = mark_file
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();

                let mark_set_id = Uuid::new_v4().to_string();
                let misc = parsed_mark.misc.as_ref();
                let full_code = misc.map(|m| m.full_code.trim()).filter(|s| !s.is_empty());
                let room = misc.map(|m| m.room.trim()).filter(|s| !s.is_empty());
                let day = misc.map(|m| m.day.trim()).filter(|s| !s.is_empty());
                let period = misc.map(|m| m.period.trim()).filter(|s| !s.is_empty());
                let weight_method: i64 = misc.map(|m| m.weight_method as i64).unwrap_or(1);
                let calc_method: i64 = misc.map(|m| m.calc_method as i64).unwrap_or(0);
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
                       calc_method
                     ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    (
                        &mark_set_id,
                        &class_id,
                        &def.code,
                        &def.file_prefix,
                        &def.description,
                        &def.weight,
                        &mark_filename,
                        def.sort_order as i64,
                        full_code,
                        room,
                        day,
                        period,
                        weight_method,
                        calc_method,
                    ),
                ) {
                    let _ = tx.rollback();
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_insert_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "mark_sets" }))
                        }
                    });
                }

                for (i, cat) in parsed_mark.categories.iter().enumerate() {
                    let cid = Uuid::new_v4().to_string();
                    if let Err(e) = tx.execute(
                        "INSERT INTO categories(id, mark_set_id, name, weight, sort_order) VALUES(?, ?, ?, ?, ?)",
                        (&cid, &mark_set_id, &cat.name, cat.weight, i as i64),
                    ) {
                        let _ = tx.rollback();
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "db_insert_failed".into(),
                                message: e.to_string(),
                                details: Some(json!({ "table": "categories" }))
                            }
                        });
                    }
                }

                let mut assessment_ids_by_idx: Vec<String> = Vec::new();
                for a in &parsed_mark.assessments {
                    let aid = Uuid::new_v4().to_string();
                    if let Err(e) = tx.execute(
                        "INSERT INTO assessments(id, mark_set_id, idx, date, category_name, title, term, legacy_kind, weight, out_of, avg_percent, avg_raw)
                         VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                        (
                            &aid,
                            &mark_set_id,
                            a.idx as i64,
                            &a.date,
                            &a.category_name,
                            &a.title,
                            a.term,
                            a.legacy_kind,
                            a.weight,
                            a.out_of,
                            a.avg_percent,
                            a.avg_raw,
                        ),
                    ) {
                        let _ = tx.rollback();
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "db_insert_failed".into(),
                                message: e.to_string(),
                                details: Some(json!({ "table": "assessments" }))
                            }
                        });
                    }
                    assessment_ids_by_idx.push(aid);
                }

                // Insert scores with legacy mark-state parity:
                // - raw == 0  => no_mark (excluded, displays blank)
                // - raw < 0   => zero (counts as 0, displays 0)
                // - raw > 0   => scored
                for (a_idx, a) in parsed_mark.assessments.iter().enumerate() {
                    let assessment_id = &assessment_ids_by_idx[a_idx];
                    let max_students =
                        std::cmp::min(student_ids_by_sort.len(), parsed_mark.last_student);
                    for s_idx in 0..max_students {
                        let student_id = &student_ids_by_sort[s_idx];
                        let (raw_value, status) = match a.raw_scores[s_idx] {
                            legacy::LegacyScore::NoMark => (Some(0.0), "no_mark"),
                            legacy::LegacyScore::Zero => (None, "zero"),
                            legacy::LegacyScore::Scored(v) => (Some(v), "scored"),
                        };
                        let sid = Uuid::new_v4().to_string();
                        if let Err(e) = tx.execute(
                            "INSERT INTO scores(id, assessment_id, student_id, raw_value, status) VALUES(?, ?, ?, ?, ?)",
                            (&sid, assessment_id, student_id, raw_value, status),
                        ) {
                            let _ = tx.rollback();
                            return json!(ErrResp {
                                id: req.id,
                                ok: false,
                                error: ErrObj {
                                    code: "db_insert_failed".into(),
                                    message: e.to_string(),
                                    details: Some(json!({ "table": "scores" }))
                                }
                            });
                        }
                        scores_imported += 1;
                    }
                }

                // Best-effort import companions: .TYP (assessment types) and .RMK (remarks).
                // These aren't required for the grid to function, but they matter for parity.
                let typ_file = mark_file.with_extension("TYP");
                if typ_file.is_file() {
                    let types = match legacy::parse_legacy_typ_file(&typ_file) {
                        Ok(v) => v,
                        Err(e) => {
                            return json!(ErrResp {
                                id: req.id,
                                ok: false,
                                error: ErrObj {
                                    code: "legacy_parse_failed".into(),
                                    message: e.to_string(),
                                    details: Some(json!({ "typFile": typ_file.to_string_lossy() }))
                                }
                            });
                        }
                    };
                    let max = std::cmp::min(types.len(), assessment_ids_by_idx.len());
                    let mut up =
                        match tx.prepare("UPDATE assessments SET legacy_type = ? WHERE id = ?") {
                            Ok(s) => s,
                            Err(e) => {
                                return json!(ErrResp {
                                    id: req.id,
                                    ok: false,
                                    error: ErrObj {
                                        code: "db_update_failed".into(),
                                        message: e.to_string(),
                                        details: Some(json!({ "table": "assessments" }))
                                    }
                                });
                            }
                        };
                    for i in 0..max {
                        if let Err(e) = up.execute((types[i] as i64, &assessment_ids_by_idx[i])) {
                            return json!(ErrResp {
                                id: req.id,
                                ok: false,
                                error: ErrObj {
                                    code: "db_update_failed".into(),
                                    message: e.to_string(),
                                    details: Some(json!({ "table": "assessments" }))
                                }
                            });
                        }
                    }
                }

                let rmk_file = mark_file.with_extension("RMK");
                if rmk_file.is_file() {
                    let rmk = match legacy::parse_legacy_rmk_file(&rmk_file) {
                        Ok(v) => v,
                        Err(e) => {
                            return json!(ErrResp {
                                id: req.id,
                                ok: false,
                                error: ErrObj {
                                    code: "legacy_parse_failed".into(),
                                    message: e.to_string(),
                                    details: Some(json!({ "rmkFile": rmk_file.to_string_lossy() }))
                                }
                            });
                        }
                    };

                    let max_entries =
                        std::cmp::min(rmk.remarks_by_entry.len(), assessment_ids_by_idx.len());
                    let max_students = std::cmp::min(student_ids_by_sort.len(), rmk.last_student);

                    let mut up = match tx.prepare(
                        "UPDATE scores SET remark = ? WHERE assessment_id = ? AND student_id = ?",
                    ) {
                        Ok(s) => s,
                        Err(e) => {
                            return json!(ErrResp {
                                id: req.id,
                                ok: false,
                                error: ErrObj {
                                    code: "db_update_failed".into(),
                                    message: e.to_string(),
                                    details: Some(json!({ "table": "scores" }))
                                }
                            });
                        }
                    };

                    for a_idx in 0..max_entries {
                        let assessment_id = &assessment_ids_by_idx[a_idx];
                        let remarks = &rmk.remarks_by_entry[a_idx];
                        for s_idx in 0..max_students {
                            let remark = remarks.get(s_idx).cloned().unwrap_or_default();
                            let remark = remark.trim().to_string();
                            if remark.is_empty() {
                                continue;
                            }
                            let student_id = &student_ids_by_sort[s_idx];
                            if let Err(e) = up.execute((&remark, assessment_id, student_id)) {
                                return json!(ErrResp {
                                    id: req.id,
                                    ok: false,
                                    error: ErrObj {
                                        code: "db_update_failed".into(),
                                        message: e.to_string(),
                                        details: Some(json!({ "table": "scores" }))
                                    }
                                });
                            }
                        }
                    }
                }

                mark_sets_imported += 1;
                assessments_imported += parsed_mark.assessments.len();
                imported_mark_files.push(mark_filename);
            }

            if let Err(e) = tx.commit() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_commit_failed".into(),
                        message: e.to_string(),
                        details: None
                    }
                });
            }

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({
                    "classId": class_id,
                    "name": class_name,
                    "studentsImported": imported,
                    "markSetsImported": mark_sets_imported,
                    "assessmentsImported": assessments_imported,
                    "scoresImported": scores_imported,
                    "sourceClFile": cl_file.to_string_lossy(),
                    "importedMarkFiles": imported_mark_files,
                    "missingMarkFiles": missing_mark_files,
                })
            })
        }
        "marksets.list" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };
            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };

            let mut stmt = match conn.prepare(
                "SELECT id, code, description, sort_order FROM mark_sets WHERE class_id = ? ORDER BY sort_order",
            ) {
                Ok(s) => s,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };

            let rows = stmt
                .query_map([&class_id], |row| {
                    let id: String = row.get(0)?;
                    let code: String = row.get(1)?;
                    let description: String = row.get(2)?;
                    let sort_order: i64 = row.get(3)?;
                    Ok(json!({ "id": id, "code": code, "description": description, "sortOrder": sort_order }))
                })
                .and_then(|it| it.collect::<Result<Vec<_>, _>>());

            match rows {
                Ok(mark_sets) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result: json!({ "markSets": mark_sets })
                }),
                Err(e) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_query_failed".into(),
                        message: e.to_string(),
                        details: None
                    }
                }),
            }
        }
        "markset.open" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };
            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };
            let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing markSetId".into(),
                            details: None
                        }
                    })
                }
            };

            let ms_row: Option<(String, String, String)> = match conn
                .query_row(
                    "SELECT id, code, description FROM mark_sets WHERE id = ? AND class_id = ?",
                    (&mark_set_id, &class_id),
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            let Some((ms_id, ms_code, ms_desc)) = ms_row else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "mark set not found".into(),
                        details: None
                    }
                });
            };

            let mut stud_stmt = match conn.prepare(
                "SELECT id, last_name, first_name, sort_order, active FROM students WHERE class_id = ? ORDER BY sort_order",
            ) {
                Ok(s) => s,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            let students_json: Vec<serde_json::Value> = match stud_stmt
                .query_map([&class_id], |row| {
                    let id: String = row.get(0)?;
                    let last: String = row.get(1)?;
                    let first: String = row.get(2)?;
                    let sort_order: i64 = row.get(3)?;
                    let active: i64 = row.get(4)?;
                    let display_name = format!("{}, {}", last, first);
                    Ok(json!({
                        "id": id,
                        "displayName": display_name,
                        "sortOrder": sort_order,
                        "active": active != 0
                    }))
                })
                .and_then(|it| it.collect::<Result<Vec<_>, _>>())
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };

            let mut assess_stmt = match conn.prepare(
                "SELECT id, idx, date, category_name, title, weight, out_of FROM assessments WHERE mark_set_id = ? ORDER BY idx",
            ) {
                Ok(s) => s,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            let assessments_json: Vec<serde_json::Value> = match assess_stmt
                .query_map([&ms_id], |row| {
                    let id: String = row.get(0)?;
                    let idx: i64 = row.get(1)?;
                    let date: Option<String> = row.get(2)?;
                    let category_name: Option<String> = row.get(3)?;
                    let title: String = row.get(4)?;
                    let weight: Option<f64> = row.get(5)?;
                    let out_of: Option<f64> = row.get(6)?;
                    Ok(json!({
                        "id": id,
                        "idx": idx,
                        "date": date,
                        "categoryName": category_name,
                        "title": title,
                        "weight": weight,
                        "outOf": out_of
                    }))
                })
                .and_then(|it| it.collect::<Result<Vec<_>, _>>())
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({
                    "markSet": { "id": ms_id, "code": ms_code, "description": ms_desc },
                    "students": students_json,
                    "assessments": assessments_json,
                    "rowCount": students_json.len(),
                    "colCount": assessments_json.len()
                })
            })
        }
        "categories.list" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };
            let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing markSetId".into(),
                            details: None
                        }
                    })
                }
            };

            let ms_exists: Option<i64> = match conn
                .query_row(
                    "SELECT 1 FROM mark_sets WHERE id = ? AND class_id = ?",
                    (&mark_set_id, &class_id),
                    |r| r.get(0),
                )
                .optional()
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            if ms_exists.is_none() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "mark set not found".into(),
                        details: None
                    }
                });
            }

            let mut stmt = match conn.prepare(
                "SELECT id, name, weight, sort_order FROM categories WHERE mark_set_id = ? ORDER BY sort_order",
            ) {
                Ok(s) => s,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
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
                Ok(categories) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result: json!({ "categories": categories })
                }),
                Err(e) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_query_failed".into(),
                        message: e.to_string(),
                        details: None
                    }
                }),
            }
        }
        "categories.create" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };
            let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing markSetId".into(),
                            details: None
                        }
                    })
                }
            };
            let name = match req.params.get("name").and_then(|v| v.as_str()) {
                Some(v) => v.trim().to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing name".into(),
                            details: None
                        }
                    })
                }
            };
            if name.is_empty() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "bad_params".into(),
                        message: "name must not be empty".into(),
                        details: None
                    }
                });
            }
            let weight = req.params.get("weight").and_then(|v| v.as_f64());

            let ms_exists: Option<i64> = match conn
                .query_row(
                    "SELECT 1 FROM mark_sets WHERE id = ? AND class_id = ?",
                    (&mark_set_id, &class_id),
                    |r| r.get(0),
                )
                .optional()
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            if ms_exists.is_none() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "mark set not found".into(),
                        details: None
                    }
                });
            }

            let sort_order: i64 = match conn.query_row(
                "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM categories WHERE mark_set_id = ?",
                [&mark_set_id],
                |r| r.get(0),
            ) {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };

            let category_id = Uuid::new_v4().to_string();
            if let Err(e) = conn.execute(
                "INSERT INTO categories(id, mark_set_id, name, weight, sort_order) VALUES(?, ?, ?, ?, ?)",
                (&category_id, &mark_set_id, &name, weight, sort_order),
            ) {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_insert_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "categories" }))
                    }
                });
            }

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({ "categoryId": category_id })
            })
        }
        "categories.update" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };
            let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing markSetId".into(),
                            details: None
                        }
                    })
                }
            };
            let category_id = match req.params.get("categoryId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing categoryId".into(),
                            details: None
                        }
                    })
                }
            };
            let Some(patch) = req.params.get("patch").and_then(|v| v.as_object()) else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "bad_params".into(),
                        message: "missing/invalid patch".into(),
                        details: None
                    }
                });
            };

            let ms_exists: Option<i64> = match conn
                .query_row(
                    "SELECT 1 FROM mark_sets WHERE id = ? AND class_id = ?",
                    (&mark_set_id, &class_id),
                    |r| r.get(0),
                )
                .optional()
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            if ms_exists.is_none() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "mark set not found".into(),
                        details: None
                    }
                });
            }

            let mut set_parts: Vec<String> = Vec::new();
            let mut bind_values: Vec<Value> = Vec::new();

            if let Some(v) = patch.get("name") {
                let Some(s) = v.as_str() else {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "patch.name must be a string".into(),
                            details: None
                        }
                    });
                };
                let s = s.trim().to_string();
                if s.is_empty() {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "name must not be empty".into(),
                            details: None
                        }
                    });
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
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "patch.weight must be a number or null".into(),
                            details: None
                        }
                    });
                }
            }

            if set_parts.is_empty() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "bad_params".into(),
                        message: "patch must include at least one field".into(),
                        details: None
                    }
                });
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
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_update_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "categories" }))
                        }
                    })
                }
            };
            if changed == 0 {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "category not found".into(),
                        details: None
                    }
                });
            }

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({ "ok": true })
            })
        }
        "categories.delete" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };
            let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing markSetId".into(),
                            details: None
                        }
                    })
                }
            };
            let category_id = match req.params.get("categoryId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing categoryId".into(),
                            details: None
                        }
                    })
                }
            };

            let ms_exists: Option<i64> = match conn
                .query_row(
                    "SELECT 1 FROM mark_sets WHERE id = ? AND class_id = ?",
                    (&mark_set_id, &class_id),
                    |r| r.get(0),
                )
                .optional()
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            if ms_exists.is_none() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "mark set not found".into(),
                        details: None
                    }
                });
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
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            let Some(sort_order) = sort_order else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "category not found".into(),
                        details: None
                    }
                });
            };

            let tx = match conn.unchecked_transaction() {
                Ok(t) => t,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_tx_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };

            let changed = match tx.execute(
                "DELETE FROM categories WHERE id = ? AND mark_set_id = ?",
                (&category_id, &mark_set_id),
            ) {
                Ok(v) => v,
                Err(e) => {
                    let _ = tx.rollback();
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_delete_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "categories" }))
                        }
                    });
                }
            };
            if changed == 0 {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "category not found".into(),
                        details: None
                    }
                });
            }

            // Keep sort_order contiguous.
            if let Err(e) = tx.execute(
                "UPDATE categories
                 SET sort_order = sort_order - 1
                 WHERE mark_set_id = ? AND sort_order > ?",
                (&mark_set_id, sort_order),
            ) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_update_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "categories" }))
                    }
                });
            }

            if let Err(e) = tx.commit() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_commit_failed".into(),
                        message: e.to_string(),
                        details: None
                    }
                });
            }

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({ "ok": true })
            })
        }
        "assessments.list" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };
            let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing markSetId".into(),
                            details: None
                        }
                    })
                }
            };

            let ms_exists: Option<i64> = match conn
                .query_row(
                    "SELECT 1 FROM mark_sets WHERE id = ? AND class_id = ?",
                    (&mark_set_id, &class_id),
                    |r| r.get(0),
                )
                .optional()
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            if ms_exists.is_none() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "mark set not found".into(),
                        details: None
                    }
                });
            }

            let mut stmt = match conn.prepare(
                "SELECT id, idx, date, category_name, title, term, legacy_type, weight, out_of
                 FROM assessments
                 WHERE mark_set_id = ?
                 ORDER BY idx",
            ) {
                Ok(s) => s,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
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
                Ok(assessments) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result: json!({ "assessments": assessments })
                }),
                Err(e) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_query_failed".into(),
                        message: e.to_string(),
                        details: None
                    }
                }),
            }
        }
        "assessments.create" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };
            let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing markSetId".into(),
                            details: None
                        }
                    })
                }
            };
            let title = match req.params.get("title").and_then(|v| v.as_str()) {
                Some(v) => v.trim().to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing title".into(),
                            details: None
                        }
                    })
                }
            };
            if title.is_empty() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "bad_params".into(),
                        message: "title must not be empty".into(),
                        details: None
                    }
                });
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

            let ms_exists: Option<i64> = match conn
                .query_row(
                    "SELECT 1 FROM mark_sets WHERE id = ? AND class_id = ?",
                    (&mark_set_id, &class_id),
                    |r| r.get(0),
                )
                .optional()
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            if ms_exists.is_none() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "mark set not found".into(),
                        details: None
                    }
                });
            }

            let append_idx: i64 = match conn.query_row(
                "SELECT COALESCE(MAX(idx), -1) + 1 FROM assessments WHERE mark_set_id = ?",
                [&mark_set_id],
                |r| r.get(0),
            ) {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            let idx = match idx_req {
                Some(v) if v >= 0 && v <= append_idx => v,
                Some(_) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "idx out of range".into(),
                            details: Some(json!({ "max": append_idx }))
                        }
                    })
                }
                None => append_idx,
            };

            let tx = match conn.unchecked_transaction() {
                Ok(t) => t,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_tx_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };

            // If inserting into the middle, shift existing idx values up by 1 (descending).
            if idx < append_idx {
                let mut stmt = match tx.prepare(
                    "SELECT id, idx FROM assessments WHERE mark_set_id = ? AND idx >= ? ORDER BY idx DESC",
                ) {
                    Ok(s) => s,
                    Err(e) => {
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "db_query_failed".into(),
                                message: e.to_string(),
                                details: None
                            }
                        });
                    }
                };
                let rows: Vec<(String, i64)> = match stmt
                    .query_map((&mark_set_id, idx), |row| {
                        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                    })
                    .and_then(|it| it.collect::<Result<Vec<_>, _>>())
                {
                    Ok(v) => v,
                    Err(e) => {
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "db_query_failed".into(),
                                message: e.to_string(),
                                details: None
                            }
                        });
                    }
                };
                let mut up = match tx.prepare("UPDATE assessments SET idx = ? WHERE id = ?") {
                    Ok(s) => s,
                    Err(e) => {
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "db_update_failed".into(),
                                message: e.to_string(),
                                details: Some(json!({ "table": "assessments" }))
                            }
                        });
                    }
                };
                for (aid, cur_idx) in rows {
                    if let Err(e) = up.execute((cur_idx + 1, &aid)) {
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "db_update_failed".into(),
                                message: e.to_string(),
                                details: Some(json!({ "table": "assessments" }))
                            }
                        });
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
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_insert_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "assessments" }))
                    }
                });
            }

            if let Err(e) = tx.commit() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_commit_failed".into(),
                        message: e.to_string(),
                        details: None
                    }
                });
            }

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({ "assessmentId": assessment_id })
            })
        }
        "assessments.update" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };
            let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing markSetId".into(),
                            details: None
                        }
                    })
                }
            };
            let assessment_id = match req.params.get("assessmentId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing assessmentId".into(),
                            details: None
                        }
                    })
                }
            };
            let Some(patch) = req.params.get("patch").and_then(|v| v.as_object()) else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "bad_params".into(),
                        message: "missing/invalid patch".into(),
                        details: None
                    }
                });
            };

            let ms_exists: Option<i64> = match conn
                .query_row(
                    "SELECT 1 FROM mark_sets WHERE id = ? AND class_id = ?",
                    (&mark_set_id, &class_id),
                    |r| r.get(0),
                )
                .optional()
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            if ms_exists.is_none() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "mark set not found".into(),
                        details: None
                    }
                });
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
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "patch.date must be a string or null".into(),
                            details: None
                        }
                    });
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
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "patch.categoryName must be a string or null".into(),
                            details: None
                        }
                    });
                }
            }
            if let Some(v) = patch.get("title") {
                let Some(s) = v.as_str() else {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "patch.title must be a string".into(),
                            details: None
                        }
                    });
                };
                let t = s.trim().to_string();
                if t.is_empty() {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "title must not be empty".into(),
                            details: None
                        }
                    });
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
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "patch.term must be an integer or null".into(),
                            details: None
                        }
                    });
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
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "patch.legacyType must be an integer or null".into(),
                            details: None
                        }
                    });
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
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "patch.weight must be a number or null".into(),
                            details: None
                        }
                    });
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
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "patch.outOf must be a number or null".into(),
                            details: None
                        }
                    });
                }
            }

            if set_parts.is_empty() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "bad_params".into(),
                        message: "patch must include at least one field".into(),
                        details: None
                    }
                });
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
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_update_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "assessments" }))
                        }
                    })
                }
            };
            if changed == 0 {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "assessment not found".into(),
                        details: None
                    }
                });
            }

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({ "ok": true })
            })
        }
        "assessments.delete" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };
            let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing markSetId".into(),
                            details: None
                        }
                    })
                }
            };
            let assessment_id = match req.params.get("assessmentId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing assessmentId".into(),
                            details: None
                        }
                    })
                }
            };

            let ms_exists: Option<i64> = match conn
                .query_row(
                    "SELECT 1 FROM mark_sets WHERE id = ? AND class_id = ?",
                    (&mark_set_id, &class_id),
                    |r| r.get(0),
                )
                .optional()
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            if ms_exists.is_none() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "mark set not found".into(),
                        details: None
                    }
                });
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
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            let Some(idx) = idx else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "assessment not found".into(),
                        details: None
                    }
                });
            };

            let tx = match conn.unchecked_transaction() {
                Ok(t) => t,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_tx_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };

            if let Err(e) = tx.execute(
                "DELETE FROM scores WHERE assessment_id = ?",
                [&assessment_id],
            ) {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "scores" }))
                    }
                });
            }

            let changed = match tx.execute(
                "DELETE FROM assessments WHERE id = ? AND mark_set_id = ?",
                (&assessment_id, &mark_set_id),
            ) {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_delete_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "assessments" }))
                        }
                    });
                }
            };
            if changed == 0 {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "assessment not found".into(),
                        details: None
                    }
                });
            }

            // Shift down higher idx values (ascending).
            let mut stmt = match tx.prepare(
                "SELECT id, idx FROM assessments WHERE mark_set_id = ? AND idx > ? ORDER BY idx ASC",
            ) {
                Ok(s) => s,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    });
                }
            };
            let rows: Vec<(String, i64)> = match stmt
                .query_map((&mark_set_id, idx), |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
                })
                .and_then(|it| it.collect::<Result<Vec<_>, _>>())
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    });
                }
            };
            drop(stmt);
            let mut up = match tx.prepare("UPDATE assessments SET idx = ? WHERE id = ?") {
                Ok(s) => s,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_update_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "assessments" }))
                        }
                    });
                }
            };
            for (aid, cur_idx) in rows {
                if let Err(e) = up.execute((cur_idx - 1, &aid)) {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_update_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "assessments" }))
                        }
                    });
                }
            }
            drop(up);

            if let Err(e) = tx.commit() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_commit_failed".into(),
                        message: e.to_string(),
                        details: None
                    }
                });
            }

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({ "ok": true })
            })
        }
        "assessments.reorder" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };
            let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing markSetId".into(),
                            details: None
                        }
                    })
                }
            };
            let Some(arr) = req
                .params
                .get("orderedAssessmentIds")
                .and_then(|v| v.as_array())
            else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "bad_params".into(),
                        message: "missing/invalid orderedAssessmentIds".into(),
                        details: None
                    }
                });
            };
            let mut ordered: Vec<String> = Vec::with_capacity(arr.len());
            for v in arr {
                let Some(s) = v.as_str() else {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "orderedAssessmentIds must be strings".into(),
                            details: None
                        }
                    });
                };
                ordered.push(s.to_string());
            }

            let ms_exists: Option<i64> = match conn
                .query_row(
                    "SELECT 1 FROM mark_sets WHERE id = ? AND class_id = ?",
                    (&mark_set_id, &class_id),
                    |r| r.get(0),
                )
                .optional()
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            if ms_exists.is_none() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "mark set not found".into(),
                        details: None
                    }
                });
            }

            let mut stmt = match conn
                .prepare("SELECT id FROM assessments WHERE mark_set_id = ? ORDER BY idx")
            {
                Ok(s) => s,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            let current_ids: Vec<String> = match stmt
                .query_map([&mark_set_id], |row| row.get::<_, String>(0))
                .and_then(|it| it.collect::<Result<Vec<_>, _>>())
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            if ordered.len() != current_ids.len() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "bad_params".into(),
                        message:
                            "orderedAssessmentIds must be a permutation of the mark set assessments"
                                .into(),
                        details: Some(
                            json!({ "expected": current_ids.len(), "got": ordered.len() })
                        )
                    }
                });
            }

            let current_set: HashSet<String> = current_ids.into_iter().collect();
            let mut seen: HashSet<String> = HashSet::new();
            for id in &ordered {
                if !seen.insert(id.clone()) {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "orderedAssessmentIds contains duplicates".into(),
                            details: Some(json!({ "assessmentId": id }))
                        }
                    });
                }
                if !current_set.contains(id) {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "orderedAssessmentIds contains unknown assessmentId".into(),
                            details: Some(json!({ "assessmentId": id }))
                        }
                    });
                }
            }

            let tx = match conn.unchecked_transaction() {
                Ok(t) => t,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_tx_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };

            // Avoid UNIQUE collisions by first moving all idx into a temporary range.
            if let Err(e) = tx.execute(
                "UPDATE assessments SET idx = idx + 1000000 WHERE mark_set_id = ?",
                [&mark_set_id],
            ) {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_update_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "assessments" }))
                    }
                });
            }

            let mut up = match tx
                .prepare("UPDATE assessments SET idx = ? WHERE id = ? AND mark_set_id = ?")
            {
                Ok(s) => s,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_update_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "assessments" }))
                        }
                    });
                }
            };
            for (i, aid) in ordered.iter().enumerate() {
                if let Err(e) = up.execute((i as i64, aid, &mark_set_id)) {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_update_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "assessments" }))
                        }
                    });
                }
            }
            drop(up);

            if let Err(e) = tx.commit() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_commit_failed".into(),
                        message: e.to_string(),
                        details: None
                    }
                });
            }

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({ "ok": true })
            })
        }
        "grid.get" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };
            let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing markSetId".into(),
                            details: None
                        }
                    })
                }
            };

            let row_start = req
                .params
                .get("rowStart")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let row_count_req = req
                .params
                .get("rowCount")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let col_start = req
                .params
                .get("colStart")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            let col_count_req = req
                .params
                .get("colCount")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);

            let mut student_stmt = match conn.prepare(
                "SELECT id FROM students WHERE class_id = ? ORDER BY sort_order LIMIT ? OFFSET ?",
            ) {
                Ok(s) => s,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            let student_ids = match student_stmt
                .query_map((&class_id, row_count_req, row_start), |row| {
                    row.get::<_, String>(0)
                })
                .and_then(|it| it.collect::<Result<Vec<_>, _>>())
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };

            let mut assess_stmt = match conn.prepare(
                "SELECT id FROM assessments WHERE mark_set_id = ? ORDER BY idx LIMIT ? OFFSET ?",
            ) {
                Ok(s) => s,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            let assessment_ids = match assess_stmt
                .query_map((&mark_set_id, col_count_req, col_start), |row| {
                    row.get::<_, String>(0)
                })
                .and_then(|it| it.collect::<Result<Vec<_>, _>>())
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };

            let row_count = student_ids.len();
            let col_count = assessment_ids.len();

            let mut cells: Vec<Vec<Option<f64>>> = vec![vec![None; col_count]; row_count];

            if row_count > 0 && col_count > 0 {
                let assess_placeholders = std::iter::repeat("?")
                    .take(col_count)
                    .collect::<Vec<_>>()
                    .join(",");
                let stud_placeholders = std::iter::repeat("?")
                    .take(row_count)
                    .collect::<Vec<_>>()
                    .join(",");
                let sql = format!(
                    "SELECT assessment_id, student_id, raw_value, status FROM scores
                     WHERE assessment_id IN ({}) AND student_id IN ({})",
                    assess_placeholders, stud_placeholders
                );

                let mut bind_values: Vec<Value> = Vec::with_capacity(col_count + row_count);
                for id in &assessment_ids {
                    bind_values.push(Value::Text(id.clone()));
                }
                for id in &student_ids {
                    bind_values.push(Value::Text(id.clone()));
                }

                let mut score_stmt = match conn.prepare(&sql) {
                    Ok(s) => s,
                    Err(e) => {
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "db_query_failed".into(),
                                message: e.to_string(),
                                details: None
                            }
                        })
                    }
                };

                let student_index: HashMap<&str, usize> = student_ids
                    .iter()
                    .enumerate()
                    .map(|(i, id)| (id.as_str(), i))
                    .collect();
                let assessment_index: HashMap<&str, usize> = assessment_ids
                    .iter()
                    .enumerate()
                    .map(|(i, id)| (id.as_str(), i))
                    .collect();

                let score_rows = score_stmt.query_map(params_from_iter(bind_values), |row| {
                    let assessment_id: String = row.get(0)?;
                    let student_id: String = row.get(1)?;
                    let raw_value: Option<f64> = row.get(2)?;
                    let status: String = row.get(3)?;
                    Ok((assessment_id, student_id, raw_value, status))
                });

                match score_rows {
                    Ok(it) => {
                        for r in it.flatten() {
                            let Some(&r_i) = student_index.get(r.1.as_str()) else {
                                continue;
                            };
                            let Some(&c_i) = assessment_index.get(r.0.as_str()) else {
                                continue;
                            };

                            let display_value = match r.3.as_str() {
                                "no_mark" => None,
                                "zero" => Some(0.0),
                                "scored" => r.2,
                                _ => r.2,
                            };
                            cells[r_i][c_i] = display_value;
                        }
                    }
                    Err(e) => {
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "db_query_failed".into(),
                                message: e.to_string(),
                                details: None
                            }
                        })
                    }
                }
            }

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({
                    "rowStart": row_start,
                    "rowCount": row_count,
                    "colStart": col_start,
                    "colCount": col_count,
                    "cells": cells
                })
            })
        }
        "grid.updateCell" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };
            let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing markSetId".into(),
                            details: None
                        }
                    })
                }
            };
            let row = match req.params.get("row").and_then(|v| v.as_i64()) {
                Some(v) if v >= 0 => v,
                _ => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing/invalid row".into(),
                            details: None
                        }
                    })
                }
            };
            let col = match req.params.get("col").and_then(|v| v.as_i64()) {
                Some(v) if v >= 0 => v,
                _ => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing/invalid col".into(),
                            details: None
                        }
                    })
                }
            };

            let value = req.params.get("value").and_then(|v| v.as_f64());
            let (raw_value, status) = match value {
                Some(v) if v < 0.0 => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "negative marks are not allowed".into(),
                            details: Some(json!({ "value": v }))
                        }
                    })
                }
                Some(v) if v > 0.0 => (Some(v), "scored"),
                // Legacy parity: blank or 0 means "No Mark" (excluded, displays blank).
                _ => (Some(0.0), "no_mark"),
            };

            let student_id: Option<String> = match conn
                .query_row(
                    "SELECT id FROM students WHERE class_id = ? AND sort_order = ?",
                    (&class_id, row),
                    |r| r.get(0),
                )
                .optional()
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            let Some(student_id) = student_id else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "student not found".into(),
                        details: None
                    }
                });
            };

            let assessment_id: Option<String> = match conn
                .query_row(
                    "SELECT id FROM assessments WHERE mark_set_id = ? AND idx = ?",
                    (&mark_set_id, col),
                    |r| r.get(0),
                )
                .optional()
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            let Some(assessment_id) = assessment_id else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "assessment not found".into(),
                        details: None
                    }
                });
            };

            let score_id = Uuid::new_v4().to_string();
            if let Err(e) = conn.execute(
                "INSERT INTO scores(id, assessment_id, student_id, raw_value, status)
                 VALUES(?, ?, ?, ?, ?)
                 ON CONFLICT(assessment_id, student_id) DO UPDATE SET
                   raw_value = excluded.raw_value,
                   status = excluded.status",
                (&score_id, &assessment_id, &student_id, raw_value, status),
            ) {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_insert_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "scores" }))
                    }
                });
            }

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({ "ok": true })
            })
        }
        "markset.settings.get" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };
            let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing markSetId".into(),
                            details: None
                        }
                    })
                }
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
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
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
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "mark set not found".into(),
                        details: None
                    }
                });
            };

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({
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
                })
            })
        }
        "markset.settings.update" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };
            let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing markSetId".into(),
                            details: None
                        }
                    })
                }
            };
            let Some(patch) = req.params.get("patch").and_then(|v| v.as_object()) else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "bad_params".into(),
                        message: "missing/invalid patch".into(),
                        details: None
                    }
                });
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
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "patch.fullCode must be string or null".into(),
                            details: None
                        }
                    });
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
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "patch.room must be string or null".into(),
                            details: None
                        }
                    });
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
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "patch.day must be string or null".into(),
                            details: None
                        }
                    });
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
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "patch.period must be string or null".into(),
                            details: None
                        }
                    });
                }
            }
            if let Some(v) = patch.get("weightMethod") {
                let Some(n) = v.as_i64() else {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "patch.weightMethod must be integer".into(),
                            details: None
                        }
                    });
                };
                if !(0..=2).contains(&n) {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "patch.weightMethod must be 0, 1, or 2".into(),
                            details: None
                        }
                    });
                }
                set_parts.push("weight_method = ?".into());
                bind_values.push(Value::Integer(n));
            }
            if let Some(v) = patch.get("calcMethod") {
                let Some(n) = v.as_i64() else {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "patch.calcMethod must be integer".into(),
                            details: None
                        }
                    });
                };
                if !(0..=4).contains(&n) {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "patch.calcMethod must be 0..4".into(),
                            details: None
                        }
                    });
                }
                set_parts.push("calc_method = ?".into());
                bind_values.push(Value::Integer(n));
            }

            if set_parts.is_empty() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "bad_params".into(),
                        message: "patch must include at least one field".into(),
                        details: None
                    }
                });
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
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_update_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "mark_sets" }))
                        }
                    })
                }
            };
            if changed == 0 {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "mark set not found".into(),
                        details: None
                    }
                });
            }

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({ "ok": true })
            })
        }
        "calc.assessmentStats" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };
            let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing markSetId".into(),
                            details: None
                        }
                    })
                }
            };

            let filters = match parse_summary_filters(req.params.get("filters")) {
                Ok(v) => v,
                Err(err) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: err
                    })
                }
            };

            let summary = match compute_markset_summary(conn, &class_id, &mark_set_id, &filters) {
                Ok(v) => v,
                Err(err) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: err
                    })
                }
            };

            let assessments = summary
                .get("perAssessment")
                .cloned()
                .unwrap_or_else(|| json!([]));
            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({
                    "assessments": assessments
                })
            })
        }
        "calc.markSetSummary" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };
            let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing markSetId".into(),
                            details: None
                        }
                    })
                }
            };
            let filters = match parse_summary_filters(req.params.get("filters")) {
                Ok(v) => v,
                Err(err) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: err
                    })
                }
            };

            match compute_markset_summary(conn, &class_id, &mark_set_id, &filters) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(err) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: err
                }),
            }
        }
        "reports.markSetSummaryModel" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };
            let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing markSetId".into(),
                            details: None
                        }
                    })
                }
            };

            let filters = SummaryFilters::default();
            match compute_markset_summary(conn, &class_id, &mark_set_id, &filters) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(err) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: err
                }),
            }
        }
        "reports.markSetGridModel" => {
            let Some(conn) = state.db.as_ref() else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "no_workspace".into(),
                        message: "select a workspace first".into(),
                        details: None
                    }
                });
            };

            let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing classId".into(),
                            details: None
                        }
                    })
                }
            };
            let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing markSetId".into(),
                            details: None
                        }
                    })
                }
            };

            let class_name: Option<String> = match conn
                .query_row("SELECT name FROM classes WHERE id = ?", [&class_id], |r| {
                    r.get(0)
                })
                .optional()
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            let Some(class_name) = class_name else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "class not found".into(),
                        details: None
                    }
                });
            };

            let ms_row: Option<(String, String, String)> = match conn
                .query_row(
                    "SELECT id, code, description FROM mark_sets WHERE id = ? AND class_id = ?",
                    (&mark_set_id, &class_id),
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            let Some((ms_id, ms_code, ms_desc)) = ms_row else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "mark set not found".into(),
                        details: None
                    }
                });
            };

            let mut stud_stmt = match conn.prepare(
                "SELECT id, last_name, first_name, sort_order, active FROM students WHERE class_id = ? ORDER BY sort_order",
            ) {
                Ok(s) => s,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            let student_rows: Vec<(String, serde_json::Value)> = match stud_stmt
                .query_map([&class_id], |row| {
                    let id: String = row.get(0)?;
                    let id2 = id.clone();
                    let last: String = row.get(1)?;
                    let first: String = row.get(2)?;
                    let sort_order: i64 = row.get(3)?;
                    let active: i64 = row.get(4)?;
                    let display_name = format!("{}, {}", last, first);
                    let j = json!({
                        "id": id,
                        "displayName": display_name,
                        "sortOrder": sort_order,
                        "active": active != 0
                    });
                    Ok((id2, j))
                })
                .and_then(|it| it.collect::<Result<Vec<_>, _>>())
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };

            let mut student_ids: Vec<String> = Vec::with_capacity(student_rows.len());
            let mut students_json: Vec<serde_json::Value> = Vec::with_capacity(student_rows.len());
            for (id, j) in student_rows {
                student_ids.push(id);
                students_json.push(j);
            }

            let mut assess_stmt = match conn.prepare(
                "SELECT id, idx, date, category_name, title, weight, out_of FROM assessments WHERE mark_set_id = ? ORDER BY idx",
            ) {
                Ok(s) => s,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };
            let assessment_rows: Vec<(String, serde_json::Value)> = match assess_stmt
                .query_map([&ms_id], |row| {
                    let id: String = row.get(0)?;
                    let id2 = id.clone();
                    let idx: i64 = row.get(1)?;
                    let date: Option<String> = row.get(2)?;
                    let category_name: Option<String> = row.get(3)?;
                    let title: String = row.get(4)?;
                    let weight: Option<f64> = row.get(5)?;
                    let out_of: Option<f64> = row.get(6)?;
                    let j = json!({
                        "id": id,
                        "idx": idx,
                        "date": date,
                        "categoryName": category_name,
                        "title": title,
                        "weight": weight,
                        "outOf": out_of
                    });
                    Ok((id2, j))
                })
                .and_then(|it| it.collect::<Result<Vec<_>, _>>())
            {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            };

            let mut assessment_ids: Vec<String> = Vec::with_capacity(assessment_rows.len());
            let mut assessments_json: Vec<serde_json::Value> =
                Vec::with_capacity(assessment_rows.len());
            for (id, j) in assessment_rows {
                assessment_ids.push(id);
                assessments_json.push(j);
            }

            let row_count = student_ids.len();
            let col_count = assessment_ids.len();

            let mut cells: Vec<Vec<Option<f64>>> = vec![vec![None; col_count]; row_count];

            if row_count > 0 && col_count > 0 {
                let assess_placeholders = std::iter::repeat("?")
                    .take(col_count)
                    .collect::<Vec<_>>()
                    .join(",");
                let stud_placeholders = std::iter::repeat("?")
                    .take(row_count)
                    .collect::<Vec<_>>()
                    .join(",");
                let sql = format!(
                    "SELECT assessment_id, student_id, raw_value, status FROM scores
                     WHERE assessment_id IN ({}) AND student_id IN ({})",
                    assess_placeholders, stud_placeholders
                );

                let mut bind_values: Vec<Value> = Vec::with_capacity(col_count + row_count);
                for id in &assessment_ids {
                    bind_values.push(Value::Text(id.clone()));
                }
                for id in &student_ids {
                    bind_values.push(Value::Text(id.clone()));
                }

                let mut score_stmt = match conn.prepare(&sql) {
                    Ok(s) => s,
                    Err(e) => {
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "db_query_failed".into(),
                                message: e.to_string(),
                                details: None
                            }
                        })
                    }
                };

                let student_index: HashMap<&str, usize> = student_ids
                    .iter()
                    .enumerate()
                    .map(|(i, id)| (id.as_str(), i))
                    .collect();
                let assessment_index: HashMap<&str, usize> = assessment_ids
                    .iter()
                    .enumerate()
                    .map(|(i, id)| (id.as_str(), i))
                    .collect();

                let score_rows = score_stmt.query_map(params_from_iter(bind_values), |row| {
                    let assessment_id: String = row.get(0)?;
                    let student_id: String = row.get(1)?;
                    let raw_value: Option<f64> = row.get(2)?;
                    let status: String = row.get(3)?;
                    Ok((assessment_id, student_id, raw_value, status))
                });

                match score_rows {
                    Ok(it) => {
                        for r in it.flatten() {
                            let Some(&r_i) = student_index.get(r.1.as_str()) else {
                                continue;
                            };
                            let Some(&c_i) = assessment_index.get(r.0.as_str()) else {
                                continue;
                            };

                            let display_value = match r.3.as_str() {
                                "no_mark" => None,
                                "zero" => Some(0.0),
                                "scored" => r.2,
                                _ => r.2,
                            };
                            cells[r_i][c_i] = display_value;
                        }
                    }
                    Err(e) => {
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "db_query_failed".into(),
                                message: e.to_string(),
                                details: None
                            }
                        })
                    }
                }
            }

            let student_active: Vec<bool> = students_json
                .iter()
                .map(|j| j.get("active").and_then(|v| v.as_bool()).unwrap_or(true))
                .collect();
            let out_of_by_col: Vec<f64> = assessments_json
                .iter()
                .map(|j| j.get("outOf").and_then(|v| v.as_f64()).unwrap_or(0.0))
                .collect();

            let mut assessment_averages: Vec<serde_json::Value> = Vec::with_capacity(col_count);
            for c_i in 0..col_count {
                let out_of = *out_of_by_col.get(c_i).unwrap_or(&0.0);
                let assessment_id = assessments_json
                    .get(c_i)
                    .and_then(|j| j.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let assessment_idx = assessments_json
                    .get(c_i)
                    .and_then(|j| j.get("idx"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(c_i as i64);
                let avg = calc::assessment_average(
                    (0..row_count).filter_map(|r_i| {
                        if !*student_active.get(r_i).unwrap_or(&true) {
                            return None;
                        }
                        match cells[r_i][c_i] {
                            None => Some(calc::ScoreState::NoMark),
                            Some(v) if v == 0.0 => Some(calc::ScoreState::Zero),
                            Some(v) => Some(calc::ScoreState::Scored(v)),
                        }
                    }),
                    out_of,
                );
                assessment_averages.push(json!({
                    "assessmentId": assessment_id,
                    "idx": assessment_idx,
                    "avgRaw": avg.avg_raw,
                    "avgPercent": avg.avg_percent,
                    "scoredCount": avg.scored_count,
                    "zeroCount": avg.zero_count,
                    "noMarkCount": avg.no_mark_count
                }));
            }

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({
                    "class": { "id": class_id, "name": class_name },
                    "markSet": { "id": ms_id, "code": ms_code, "description": ms_desc },
                    "students": students_json,
                    "assessments": assessments_json,
                    "rowCount": row_count,
                    "colCount": col_count,
                    "assessmentAverages": assessment_averages,
                    "cells": cells
                })
            })
        }
        _ => json!(ErrResp {
            id: req.id,
            ok: false,
            error: ErrObj {
                code: "not_implemented".into(),
                message: format!("unknown method: {}", req.method),
                details: None
            }
        }),
    }
}

#[derive(Debug, Clone, Default)]
struct SummaryFilters {
    term: Option<i64>,
    category_name: Option<String>,
    types_mask: Option<i64>,
}

#[derive(Debug, Clone)]
struct SummaryStudent {
    id: String,
    display_name: String,
    sort_order: i64,
    active: bool,
}

#[derive(Debug, Clone)]
struct SummaryCategory {
    name: String,
    weight: f64,
    sort_order: i64,
}

#[derive(Debug, Clone)]
struct SummaryAssessment {
    id: String,
    idx: i64,
    date: Option<String>,
    category_name: Option<String>,
    title: String,
    term: Option<i64>,
    legacy_type: Option<i64>,
    weight: f64,
    out_of: f64,
}

fn parse_summary_filters(raw: Option<&serde_json::Value>) -> Result<SummaryFilters, ErrObj> {
    let Some(raw) = raw else {
        return Ok(SummaryFilters::default());
    };
    let Some(obj) = raw.as_object() else {
        return Err(ErrObj {
            code: "bad_params".into(),
            message: "filters must be an object".into(),
            details: None,
        });
    };

    let term = match obj.get("term") {
        None => None,
        Some(v) if v.is_null() => None,
        Some(v)
            if v.as_str()
                .map(|s| s.eq_ignore_ascii_case("ALL"))
                .unwrap_or(false) =>
        {
            None
        }
        Some(v) => {
            let Some(n) = v.as_i64() else {
                return Err(ErrObj {
                    code: "bad_params".into(),
                    message: "filters.term must be integer or 'ALL'".into(),
                    details: None,
                });
            };
            Some(n)
        }
    };

    let category_name = match obj.get("categoryName") {
        None => None,
        Some(v) if v.is_null() => None,
        Some(v) => {
            let Some(s) = v.as_str() else {
                return Err(ErrObj {
                    code: "bad_params".into(),
                    message: "filters.categoryName must be string or null".into(),
                    details: None,
                });
            };
            let t = s.trim();
            if t.is_empty() || t.eq_ignore_ascii_case("ALL") {
                None
            } else {
                Some(t.to_ascii_lowercase())
            }
        }
    };

    let types_mask = match obj.get("typesMask") {
        None => None,
        Some(v) if v.is_null() => None,
        Some(v) => {
            let Some(n) = v.as_i64() else {
                return Err(ErrObj {
                    code: "bad_params".into(),
                    message: "filters.typesMask must be an integer bitmask".into(),
                    details: None,
                });
            };
            Some(n)
        }
    };

    Ok(SummaryFilters {
        term,
        category_name,
        types_mask,
    })
}

fn matches_types_mask(mask: Option<i64>, legacy_type: Option<i64>) -> bool {
    let Some(mask) = mask else {
        return true;
    };
    let t = legacy_type.unwrap_or(0);
    if t < 0 {
        return false;
    }
    let shift = t as u32;
    if shift >= 63 {
        return false;
    }
    (mask & (1_i64 << shift)) != 0
}

fn compute_median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let n = sorted.len();
    if n % 2 == 1 {
        sorted[n / 2]
    } else {
        (sorted[(n / 2) - 1] + sorted[n / 2]) / 2.0
    }
}

fn compute_markset_summary(
    conn: &Connection,
    class_id: &str,
    mark_set_id: &str,
    filters: &SummaryFilters,
) -> Result<serde_json::Value, ErrObj> {
    let class_name: Option<String> = conn
        .query_row("SELECT name FROM classes WHERE id = ?", [class_id], |r| {
            r.get(0)
        })
        .optional()
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    let Some(class_name) = class_name else {
        return Err(ErrObj {
            code: "not_found".into(),
            message: "class not found".into(),
            details: None,
        });
    };

    let mark_set_row: Option<(
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        i64,
        i64,
    )> = conn
        .query_row(
            "SELECT code, description, full_code, room, day, period, weight_method, calc_method
             FROM mark_sets
             WHERE id = ? AND class_id = ?",
            (mark_set_id, class_id),
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
                ))
            },
        )
        .optional()
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    let Some((ms_code, ms_desc, full_code, room, day, period, weight_method, calc_method)) =
        mark_set_row
    else {
        return Err(ErrObj {
            code: "not_found".into(),
            message: "mark set not found".into(),
            details: None,
        });
    };

    let mut students_stmt = conn
        .prepare(
            "SELECT id, last_name, first_name, sort_order, active
             FROM students
             WHERE class_id = ?
             ORDER BY sort_order",
        )
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    let students: Vec<SummaryStudent> = students_stmt
        .query_map([class_id], |r| {
            let last: String = r.get(1)?;
            let first: String = r.get(2)?;
            Ok(SummaryStudent {
                id: r.get(0)?,
                display_name: format!("{}, {}", last, first),
                sort_order: r.get(3)?,
                active: r.get::<_, i64>(4)? != 0,
            })
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;

    let mut categories_stmt = conn
        .prepare(
            "SELECT name, COALESCE(weight, 0), sort_order
             FROM categories
             WHERE mark_set_id = ?
             ORDER BY sort_order",
        )
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    let categories: Vec<SummaryCategory> = categories_stmt
        .query_map([mark_set_id], |r| {
            Ok(SummaryCategory {
                name: r.get(0)?,
                weight: r.get::<_, f64>(1)?,
                sort_order: r.get(2)?,
            })
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;

    let mut assessments_stmt = conn
        .prepare(
            "SELECT id, idx, date, category_name, title, term, legacy_type, weight, out_of
             FROM assessments
             WHERE mark_set_id = ?
             ORDER BY idx",
        )
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    let all_assessments: Vec<SummaryAssessment> = assessments_stmt
        .query_map([mark_set_id], |r| {
            Ok(SummaryAssessment {
                id: r.get(0)?,
                idx: r.get(1)?,
                date: r.get(2)?,
                category_name: r.get(3)?,
                title: r.get(4)?,
                term: r.get(5)?,
                legacy_type: r.get(6)?,
                weight: r.get::<_, Option<f64>>(7)?.unwrap_or(1.0),
                out_of: r.get::<_, Option<f64>>(8)?.unwrap_or(0.0),
            })
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;

    let selected_assessments: Vec<SummaryAssessment> = all_assessments
        .iter()
        .filter(|a| {
            let term_ok = filters.term.map(|t| a.term == Some(t)).unwrap_or(true);
            let cat_ok = filters
                .category_name
                .as_ref()
                .map(|cat| {
                    a.category_name
                        .as_deref()
                        .map(|v| v.to_ascii_lowercase() == *cat)
                        .unwrap_or(false)
                })
                .unwrap_or(true);
            let type_ok = matches_types_mask(filters.types_mask, a.legacy_type);
            term_ok && cat_ok && type_ok
        })
        .cloned()
        .collect();

    let mut score_by_pair: HashMap<(String, String), calc::ScoreState> = HashMap::new();
    if !students.is_empty() && !all_assessments.is_empty() {
        let assessment_ids: Vec<String> = all_assessments.iter().map(|a| a.id.clone()).collect();
        let student_ids: Vec<String> = students.iter().map(|s| s.id.clone()).collect();

        let assess_placeholders = std::iter::repeat("?")
            .take(assessment_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let stud_placeholders = std::iter::repeat("?")
            .take(student_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT assessment_id, student_id, raw_value, status
             FROM scores
             WHERE assessment_id IN ({}) AND student_id IN ({})",
            assess_placeholders, stud_placeholders
        );
        let mut bind_values: Vec<Value> =
            Vec::with_capacity(assessment_ids.len() + student_ids.len());
        for id in &assessment_ids {
            bind_values.push(Value::Text(id.clone()));
        }
        for id in &student_ids {
            bind_values.push(Value::Text(id.clone()));
        }

        let mut stmt = conn.prepare(&sql).map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
        let rows = stmt
            .query_map(params_from_iter(bind_values), |r| {
                let assessment_id: String = r.get(0)?;
                let student_id: String = r.get(1)?;
                let raw_value: Option<f64> = r.get(2)?;
                let status: String = r.get(3)?;
                Ok((assessment_id, student_id, raw_value, status))
            })
            .map_err(|e| ErrObj {
                code: "db_query_failed".into(),
                message: e.to_string(),
                details: None,
            })?;
        for row in rows {
            let (assessment_id, student_id, raw_value, status) = row.map_err(|e| ErrObj {
                code: "db_query_failed".into(),
                message: e.to_string(),
                details: None,
            })?;
            let state = match status.as_str() {
                "no_mark" => calc::ScoreState::NoMark,
                "zero" => calc::ScoreState::Zero,
                "scored" => calc::ScoreState::Scored(raw_value.unwrap_or(0.0)),
                _ => raw_value
                    .map(calc::ScoreState::Scored)
                    .unwrap_or(calc::ScoreState::NoMark),
            };
            score_by_pair.insert((assessment_id, student_id), state);
        }
    }

    let mut per_assessment_json: Vec<serde_json::Value> = Vec::new();
    for a in &selected_assessments {
        let mut score_states: Vec<calc::ScoreState> = Vec::new();
        let mut median_values: Vec<f64> = Vec::new();
        for s in &students {
            if !s.active {
                continue;
            }
            let state = score_by_pair
                .get(&(a.id.clone(), s.id.clone()))
                .copied()
                .unwrap_or(calc::ScoreState::NoMark);
            match state {
                calc::ScoreState::NoMark => {}
                calc::ScoreState::Zero => median_values.push(0.0),
                calc::ScoreState::Scored(v) => {
                    if a.out_of > 0.0 {
                        median_values.push(100.0 * v / a.out_of);
                    } else {
                        median_values.push(0.0);
                    }
                }
            }
            score_states.push(state);
        }

        let stats = calc::assessment_average(score_states, a.out_of);
        per_assessment_json.push(json!({
            "assessmentId": a.id,
            "idx": a.idx,
            "date": a.date,
            "categoryName": a.category_name,
            "title": a.title,
            "outOf": a.out_of,
            "avgRaw": calc::round_off_1_decimal(stats.avg_raw),
            "avgPercent": calc::round_off_1_decimal(stats.avg_percent),
            "medianPercent": calc::round_off_1_decimal(compute_median(&median_values)),
            "scoredCount": stats.scored_count,
            "zeroCount": stats.zero_count,
            "noMarkCount": stats.no_mark_count
        }));
    }

    let mut category_weight_map: HashMap<String, f64> = HashMap::new();
    for c in &categories {
        category_weight_map.insert(c.name.to_ascii_lowercase(), c.weight);
    }

    let mut per_student_json: Vec<serde_json::Value> = Vec::new();
    let mut per_category_totals: HashMap<String, (f64, usize, i64, f64)> = HashMap::new();
    let mut per_category_assessment_counts: HashMap<String, i64> = HashMap::new();

    for a in &selected_assessments {
        let key = a
            .category_name
            .clone()
            .unwrap_or_else(|| "Uncategorized".to_string());
        *per_category_assessment_counts.entry(key).or_insert(0) += 1;
    }

    for s in &students {
        let mut no_mark_count = 0_i64;
        let mut zero_count = 0_i64;
        let mut scored_count = 0_i64;

        // Entry/equal methods.
        let mut weighted_sum = 0.0_f64;
        let mut weighted_denom = 0.0_f64;

        // Category method.
        let mut cat_inner: HashMap<String, (f64, f64)> = HashMap::new(); // sum, denom

        for a in &selected_assessments {
            let state = score_by_pair
                .get(&(a.id.clone(), s.id.clone()))
                .copied()
                .unwrap_or(calc::ScoreState::NoMark);
            let assessment_weight = if a.weight > 0.0 { a.weight } else { 1.0 };
            let percent_opt = match state {
                calc::ScoreState::NoMark => {
                    no_mark_count += 1;
                    None
                }
                calc::ScoreState::Zero => {
                    zero_count += 1;
                    Some(0.0)
                }
                calc::ScoreState::Scored(v) => {
                    scored_count += 1;
                    if a.out_of > 0.0 {
                        Some(100.0 * v / a.out_of)
                    } else {
                        Some(0.0)
                    }
                }
            };

            let Some(percent) = percent_opt else {
                continue;
            };

            let method = weight_method.clamp(0, 2);
            let use_weight = if method == 0 { assessment_weight } else { 1.0 };
            weighted_sum += percent * use_weight;
            weighted_denom += use_weight;

            let category = a
                .category_name
                .clone()
                .unwrap_or_else(|| "Uncategorized".to_string());
            let entry = cat_inner.entry(category).or_insert((0.0, 0.0));
            entry.0 += percent * assessment_weight;
            entry.1 += assessment_weight;
        }

        let final_mark_raw = {
            let method = weight_method.clamp(0, 2);
            if method == 1 {
                let mut sum = 0.0_f64;
                let mut denom = 0.0_f64;
                let mut sum_equal = 0.0_f64;
                let mut denom_equal = 0.0_f64;

                for (cat_name, (cat_sum, cat_denom)) in &cat_inner {
                    if *cat_denom <= 0.0 {
                        continue;
                    }
                    let cat_avg = *cat_sum / *cat_denom;
                    let cat_weight = category_weight_map
                        .get(&cat_name.to_ascii_lowercase())
                        .copied()
                        .unwrap_or(0.0);
                    if cat_weight > 0.0 {
                        sum += cat_avg * cat_weight;
                        denom += cat_weight;
                    }
                    sum_equal += cat_avg;
                    denom_equal += 1.0;
                }

                if denom > 0.0 {
                    Some(sum / denom)
                } else if denom_equal > 0.0 {
                    Some(sum_equal / denom_equal)
                } else {
                    None
                }
            } else if weighted_denom > 0.0 {
                Some(weighted_sum / weighted_denom)
            } else {
                None
            }
        };

        let final_mark = final_mark_raw.map(calc::round_off_1_decimal);
        per_student_json.push(json!({
            "studentId": s.id,
            "displayName": s.display_name,
            "sortOrder": s.sort_order,
            "active": s.active,
            "finalMark": final_mark,
            "noMarkCount": no_mark_count,
            "zeroCount": zero_count,
            "scoredCount": scored_count
        }));

        // Build class-level per-category averages across active students.
        if s.active {
            for (cat_name, (cat_sum, cat_denom)) in &cat_inner {
                if *cat_denom <= 0.0 {
                    continue;
                }
                let cat_avg = *cat_sum / *cat_denom;
                let weight = category_weight_map
                    .get(&cat_name.to_ascii_lowercase())
                    .copied()
                    .unwrap_or(0.0);
                let entry = per_category_totals.entry(cat_name.clone()).or_insert((
                    0.0,
                    0,
                    i64::MAX,
                    weight,
                ));
                entry.0 += cat_avg;
                entry.1 += 1;
                let cat_sort = categories
                    .iter()
                    .find(|c| c.name.eq_ignore_ascii_case(cat_name))
                    .map(|c| c.sort_order)
                    .unwrap_or(i64::MAX);
                entry.2 = entry.2.min(cat_sort);
            }
        }
    }

    let mut per_category_json: Vec<serde_json::Value> = per_category_totals
        .into_iter()
        .map(|(name, (sum, count, sort_order, weight))| {
            let class_avg = if count > 0 {
                calc::round_off_1_decimal(sum / (count as f64))
            } else {
                0.0
            };
            let assessment_count = per_category_assessment_counts
                .get(&name)
                .copied()
                .unwrap_or(0);
            let sort_order_json = if sort_order == i64::MAX {
                serde_json::Value::Null
            } else {
                json!(sort_order)
            };
            json!({
                "name": name,
                "weight": weight,
                "sortOrder": sort_order_json,
                "classAvg": class_avg,
                "studentCount": count,
                "assessmentCount": assessment_count
            })
        })
        .collect();
    per_category_json.sort_by(|a, b| {
        let a_sort = a
            .get("sortOrder")
            .and_then(|v| v.as_i64())
            .unwrap_or(i64::MAX);
        let b_sort = b
            .get("sortOrder")
            .and_then(|v| v.as_i64())
            .unwrap_or(i64::MAX);
        a_sort.cmp(&b_sort)
    });

    let categories_json: Vec<serde_json::Value> = categories
        .iter()
        .map(|c| {
            json!({
                "name": c.name,
                "weight": c.weight,
                "sortOrder": c.sort_order
            })
        })
        .collect();

    let assessments_json: Vec<serde_json::Value> = selected_assessments
        .iter()
        .map(|a| {
            json!({
                "assessmentId": a.id,
                "idx": a.idx,
                "date": a.date,
                "categoryName": a.category_name,
                "title": a.title,
                "term": a.term,
                "legacyType": a.legacy_type,
                "weight": a.weight,
                "outOf": a.out_of
            })
        })
        .collect();

    Ok(json!({
        "class": {
            "id": class_id,
            "name": class_name
        },
        "markSet": {
            "id": mark_set_id,
            "code": ms_code,
            "description": ms_desc
        },
        "settings": {
            "fullCode": full_code,
            "room": room,
            "day": day,
            "period": period,
            "weightMethod": weight_method,
            "calcMethod": calc_method
        },
        "filters": {
            "term": filters.term,
            "categoryName": filters.category_name,
            "typesMask": filters.types_mask
        },
        "categories": categories_json,
        "assessments": assessments_json,
        "perAssessment": per_assessment_json,
        "perCategory": per_category_json,
        "perStudent": per_student_json
    }))
}
