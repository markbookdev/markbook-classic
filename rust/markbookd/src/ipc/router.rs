use super::handlers;
use super::types::{AppState, Request};
use crate::{backup, calc, db, legacy};
use rusqlite::types::Value;
use rusqlite::{params_from_iter, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use uuid::Uuid;

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

pub fn handle_request(state: &mut AppState, req: Request) -> serde_json::Value {
    if let Some(resp) = handlers::core::try_handle(state, &req) {
        return resp;
    }
    if let Some(resp) = handlers::classes::try_handle(state, &req) {
        return resp;
    }
    if let Some(resp) = handlers::import_legacy::try_handle(state, &req) {
        return resp;
    }
    if let Some(resp) = handlers::grid::try_handle(state, &req) {
        return resp;
    }
    if let Some(resp) = handlers::students::try_handle(state, &req) {
        return resp;
    }
    if let Some(resp) = handlers::markset_setup::try_handle(state, &req) {
        return resp;
    }
    if let Some(resp) = handlers::attendance::try_handle(state, &req) {
        return resp;
    }
    if let Some(resp) = handlers::seating::try_handle(state, &req) {
        return resp;
    }
    if let Some(resp) = handlers::comments::try_handle(state, &req) {
        return resp;
    }
    if let Some(resp) = handlers::reports::try_handle(state, &req) {
        return resp;
    }
    if let Some(resp) = handlers::backup_exchange::try_handle(state, &req) {
        return resp;
    }
    if let Some(resp) = handlers::assets::try_handle(state, &req) {
        return resp;
    }

    handle_request_legacy(state, req)
}

pub(crate) fn handle_request_legacy(state: &mut AppState, req: Request) -> serde_json::Value {
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
                "DELETE FROM comment_set_remarks
                 WHERE comment_set_index_id IN (
                   SELECT csi.id
                   FROM comment_set_indexes csi
                   WHERE csi.class_id = ?
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
                        details: Some(json!({ "table": "comment_set_remarks" }))
                    }
                });
            }

            if let Err(e) = tx.execute(
                "DELETE FROM comment_set_indexes WHERE class_id = ?",
                [&class_id],
            ) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "comment_set_indexes" }))
                    }
                });
            }

            if let Err(e) = tx.execute(
                "DELETE FROM attendance_student_months WHERE class_id = ?",
                [&class_id],
            ) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "attendance_student_months" }))
                    }
                });
            }

            if let Err(e) = tx.execute(
                "DELETE FROM attendance_months WHERE class_id = ?",
                [&class_id],
            ) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "attendance_months" }))
                    }
                });
            }

            if let Err(e) = tx.execute(
                "DELETE FROM attendance_settings WHERE class_id = ?",
                [&class_id],
            ) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "attendance_settings" }))
                    }
                });
            }

            if let Err(e) = tx.execute(
                "DELETE FROM seating_assignments WHERE class_id = ?",
                [&class_id],
            ) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "seating_assignments" }))
                    }
                });
            }

            if let Err(e) = tx.execute("DELETE FROM seating_plans WHERE class_id = ?", [&class_id])
            {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "seating_plans" }))
                    }
                });
            }

            if let Err(e) = tx.execute("DELETE FROM loaned_items WHERE class_id = ?", [&class_id]) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "loaned_items" }))
                    }
                });
            }

            if let Err(e) = tx.execute(
                "DELETE FROM student_device_map WHERE class_id = ?",
                [&class_id],
            ) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "student_device_map" }))
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

            if let Err(e) = tx.execute(
                "DELETE FROM learning_skills_cells WHERE class_id = ?",
                [&class_id],
            ) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "learning_skills_cells" }))
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
        "backup.exportWorkspaceBundle" => {
            let out_path = match req.params.get("outPath").and_then(|v| v.as_str()) {
                Some(v) if !v.trim().is_empty() => v.trim().to_string(),
                _ => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing outPath".into(),
                            details: None
                        }
                    })
                }
            };
            let workspace_path = req
                .params
                .get("workspacePath")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .or_else(|| state.workspace.clone());
            let Some(workspace_path) = workspace_path else {
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

            if let Some(conn) = state.db.as_ref() {
                let _ = conn.execute_batch("PRAGMA wal_checkpoint(FULL)");
            }

            let out = PathBuf::from(&out_path);
            let export = match backup::export_workspace_bundle(&workspace_path, &out) {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "io_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "path": out_path }))
                        }
                    })
                }
            };

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({
                    "ok": true,
                    "path": out_path,
                    "bundleFormat": export.bundle_format,
                    "entryCount": export.entry_count
                })
            })
        }
        "backup.importWorkspaceBundle" => {
            let in_path = match req.params.get("inPath").and_then(|v| v.as_str()) {
                Some(v) if !v.trim().is_empty() => v.trim().to_string(),
                _ => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing inPath".into(),
                            details: None
                        }
                    })
                }
            };
            let workspace_path = req
                .params
                .get("workspacePath")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .or_else(|| state.workspace.clone());
            let Some(workspace_path) = workspace_path else {
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

            let src = PathBuf::from(&in_path);
            if !src.is_file() {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "not_found".into(),
                        message: "bundle file not found".into(),
                        details: Some(json!({ "path": in_path }))
                    }
                });
            }
            if let Err(e) = std::fs::create_dir_all(&workspace_path) {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "io_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "path": workspace_path.to_string_lossy() }))
                    }
                });
            }

            // Drop open handle before replacing file.
            state.db = None;

            let import = match backup::import_workspace_bundle(&src, &workspace_path) {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "io_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "path": src.to_string_lossy() }))
                        }
                    })
                }
            };

            match db::open_db(&workspace_path) {
                Ok(conn) => {
                    state.workspace = Some(workspace_path.clone());
                    state.db = Some(conn);
                    json!(OkResp {
                        id: req.id,
                        ok: true,
                        result: json!({
                            "ok": true,
                            "workspacePath": workspace_path.to_string_lossy(),
                            "bundleFormatDetected": import.bundle_format_detected
                        })
                    })
                }
                Err(e) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_open_failed".into(),
                        message: e.to_string(),
                        details: None
                    }
                }),
            }
        }
        "exchange.exportClassCsv" => {
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
            let out_path = match req.params.get("outPath").and_then(|v| v.as_str()) {
                Some(v) if !v.trim().is_empty() => v.trim().to_string(),
                _ => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing outPath".into(),
                            details: None
                        }
                    })
                }
            };
            let mut stmt = match conn.prepare(
                "SELECT s.id, s.last_name, s.first_name, ms.code, a.idx, a.title, sc.status, sc.raw_value
                 FROM scores sc
                 JOIN assessments a ON a.id = sc.assessment_id
                 JOIN mark_sets ms ON ms.id = a.mark_set_id
                 JOIN students s ON s.id = sc.student_id
                 WHERE s.class_id = ?
                 ORDER BY s.sort_order, ms.sort_order, a.idx",
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
            let rows = match stmt
                .query_map([&class_id], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, i64>(4)?,
                        r.get::<_, String>(5)?,
                        r.get::<_, String>(6)?,
                        r.get::<_, Option<f64>>(7)?,
                    ))
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
            let mut csv = String::from(
                "student_id,student_name,mark_set_code,assessment_idx,assessment_title,status,raw_value\n",
            );
            let rows_exported = rows.len();
            for (
                student_id,
                last,
                first,
                mark_set_code,
                assessment_idx,
                title,
                status,
                raw_value,
            ) in rows
            {
                let display_name = format!("{}, {}", last, first);
                csv.push_str(&format!(
                    "{},{},{},{},{},{},{}\n",
                    csv_quote(&student_id),
                    csv_quote(&display_name),
                    csv_quote(&mark_set_code),
                    assessment_idx,
                    csv_quote(&title),
                    csv_quote(&status),
                    raw_value.map(|v| v.to_string()).unwrap_or_default()
                ));
            }

            let out = PathBuf::from(&out_path);
            if let Some(parent) = out.parent() {
                if let Err(e) = std::fs::create_dir_all(parent) {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "io_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "path": out_path }))
                        }
                    });
                }
            }
            if let Err(e) = std::fs::write(&out, csv) {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "io_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "path": out_path }))
                    }
                });
            }

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({ "ok": true, "rowsExported": rows_exported, "path": out_path })
            })
        }
        "exchange.importClassCsv" => {
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
            let in_path = match req.params.get("inPath").and_then(|v| v.as_str()) {
                Some(v) if !v.trim().is_empty() => v.trim().to_string(),
                _ => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing inPath".into(),
                            details: None
                        }
                    })
                }
            };
            let mode = req
                .params
                .get("mode")
                .and_then(|v| v.as_str())
                .unwrap_or("upsert")
                .to_ascii_lowercase();
            let text = match std::fs::read_to_string(&in_path) {
                Ok(t) => t,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "io_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "path": in_path }))
                        }
                    })
                }
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
            if mode == "replace" {
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
            }

            let mut updated = 0usize;
            for (line_no, raw_line) in text.lines().enumerate() {
                if line_no == 0 {
                    continue;
                }
                let line = raw_line.trim();
                if line.is_empty() {
                    continue;
                }
                let fields = parse_csv_record(line);
                if fields.len() < 7 {
                    continue;
                }
                let student_id = fields[0].trim();
                let mark_set_code = fields[2].trim();
                let assessment_idx = match fields[3].trim().parse::<i64>() {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let status = fields[5].trim().to_ascii_lowercase();
                let raw_value = if fields[6].trim().is_empty() {
                    None
                } else {
                    fields[6].trim().parse::<f64>().ok()
                };

                let student_ok = tx
                    .query_row(
                        "SELECT 1 FROM students WHERE id = ? AND class_id = ?",
                        (student_id, &class_id),
                        |r| r.get::<_, i64>(0),
                    )
                    .optional()
                    .ok()
                    .flatten()
                    .is_some();
                if !student_ok {
                    continue;
                }
                let assessment_id: Option<String> = tx
                    .query_row(
                        "SELECT a.id
                         FROM assessments a
                         JOIN mark_sets ms ON ms.id = a.mark_set_id
                         WHERE ms.class_id = ? AND ms.code = ? AND a.idx = ?",
                        (&class_id, mark_set_code, assessment_idx),
                        |r| r.get(0),
                    )
                    .optional()
                    .ok()
                    .flatten();
                let Some(assessment_id) = assessment_id else {
                    continue;
                };
                let (resolved_raw, resolved_state) =
                    match resolve_score_state(Some(&status), raw_value) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                if let Err(e) = upsert_score(
                    &tx,
                    &assessment_id,
                    student_id,
                    resolved_raw,
                    resolved_state,
                ) {
                    let _ = tx.rollback();
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: e
                    });
                }
                updated += 1;
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
                result: json!({ "ok": true, "updated": updated })
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

            if let Err(e) = tx.execute(
                "DELETE FROM attendance_student_months WHERE class_id = ? AND student_id = ?",
                (&class_id, &student_id),
            ) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "attendance_student_months" }))
                    }
                });
            }

            if let Err(e) = tx.execute(
                "DELETE FROM seating_assignments WHERE class_id = ? AND student_id = ?",
                (&class_id, &student_id),
            ) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "seating_assignments" }))
                    }
                });
            }

            if let Err(e) = tx.execute(
                "DELETE FROM comment_set_remarks WHERE student_id = ?",
                (&student_id,),
            ) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "comment_set_remarks" }))
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
        "loaned.list" => {
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
            match loaned_list(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "loaned.get" => {
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
            match loaned_get(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "loaned.update" => {
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
            match loaned_update(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "devices.list" => {
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
            match devices_list(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "devices.get" => {
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
            match devices_get(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "devices.update" => {
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
            match devices_update(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "learningSkills.open" => {
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
            match learning_skills_open(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "learningSkills.updateCell" => {
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
            match learning_skills_update_cell(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "learningSkills.reportModel" => {
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
            match learning_skills_report_model(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "attendance.monthOpen" => {
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
            match attendance_month_open(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result: json!(result)
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "attendance.setTypeOfDay" => {
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
            match attendance_set_type_of_day(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result: json!(result)
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "attendance.setStudentDay" => {
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
            match attendance_set_student_day(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "attendance.bulkStampDay" => {
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
            match attendance_bulk_stamp_day(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "seating.get" => {
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
            match seating_get(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "seating.save" => {
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
            match seating_save(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "comments.sets.list" => {
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
            match comments_sets_list(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "comments.sets.open" => {
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
            match comments_sets_open(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "comments.sets.upsert" => {
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
            match comments_sets_upsert(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "comments.sets.delete" => {
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
            match comments_sets_delete(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "comments.banks.list" => {
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
            match comments_banks_list(conn) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "comments.banks.open" => {
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
            match comments_banks_open(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "comments.banks.create" => {
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
            match comments_banks_create(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "comments.banks.updateMeta" => {
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
            match comments_banks_update_meta(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "comments.banks.entryUpsert" => {
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
            match comments_banks_entry_upsert(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "comments.banks.entryDelete" => {
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
            match comments_banks_entry_delete(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "comments.banks.importBnk" => {
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
            match comments_banks_import_bnk(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "comments.banks.exportBnk" => {
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
            match comments_banks_export_bnk(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
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

            let mut attendance_imported = false;
            let mut seating_imported = false;
            let mut banks_imported = 0usize;
            let mut comment_sets_imported = 0usize;
            let mut comment_remarks_imported = 0usize;
            let mut loaned_items_imported = 0usize;
            let mut device_mappings_imported = 0usize;
            let mut combined_comment_sets_imported = 0usize;
            let mut warnings: Vec<serde_json::Value> = Vec::new();

            // Best-effort attendance import (.ATN).
            match legacy::find_attendance_file(&legacy_folder) {
                Ok(Some(att_file)) => {
                    let att = match legacy::parse_legacy_attendance_file(&att_file) {
                        Ok(v) => v,
                        Err(e) => {
                            let _ = tx.rollback();
                            return json!(ErrResp {
                                id: req.id,
                                ok: false,
                                error: ErrObj {
                                    code: "legacy_parse_failed".into(),
                                    message: e.to_string(),
                                    details: Some(
                                        json!({ "attendanceFile": att_file.to_string_lossy() })
                                    )
                                }
                            });
                        }
                    };

                    if let Err(e) = tx.execute(
                        "INSERT INTO attendance_settings(class_id, school_year_start_month)
                         VALUES(?, ?)
                         ON CONFLICT(class_id) DO UPDATE SET
                           school_year_start_month = excluded.school_year_start_month",
                        (&class_id, att.school_year_start_month as i64),
                    ) {
                        let _ = tx.rollback();
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "db_insert_failed".into(),
                                message: e.to_string(),
                                details: Some(json!({ "table": "attendance_settings" }))
                            }
                        });
                    }

                    for m in &att.months {
                        if let Err(e) = tx.execute(
                            "INSERT INTO attendance_months(class_id, month, type_of_day_codes)
                             VALUES(?, ?, ?)
                             ON CONFLICT(class_id, month) DO UPDATE SET
                               type_of_day_codes = excluded.type_of_day_codes",
                            (&class_id, m.month as i64, &m.type_of_day_codes),
                        ) {
                            let _ = tx.rollback();
                            return json!(ErrResp {
                                id: req.id,
                                ok: false,
                                error: ErrObj {
                                    code: "db_insert_failed".into(),
                                    message: e.to_string(),
                                    details: Some(json!({ "table": "attendance_months" }))
                                }
                            });
                        }

                        let max_students =
                            std::cmp::min(student_ids_by_sort.len(), m.student_day_codes.len());
                        for s_idx in 0..max_students {
                            let student_id = &student_ids_by_sort[s_idx];
                            let day_codes = &m.student_day_codes[s_idx];
                            if let Err(e) = tx.execute(
                                "INSERT INTO attendance_student_months(class_id, student_id, month, day_codes)
                                 VALUES(?, ?, ?, ?)
                                 ON CONFLICT(class_id, student_id, month) DO UPDATE SET
                                   day_codes = excluded.day_codes",
                                (&class_id, student_id, m.month as i64, day_codes),
                            ) {
                                let _ = tx.rollback();
                                return json!(ErrResp {
                                    id: req.id,
                                    ok: false,
                                    error: ErrObj {
                                        code: "db_insert_failed".into(),
                                        message: e.to_string(),
                                        details: Some(json!({ "table": "attendance_student_months" }))
                                    }
                                });
                            }
                        }
                    }

                    attendance_imported = true;
                }
                Ok(None) => {
                    warnings.push(json!({
                        "code": "legacy_missing_attendance_file",
                        "folder": legacy_folder.to_string_lossy()
                    }));
                }
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
            }

            // Best-effort seating import (.SPL).
            match legacy::find_seating_file(&legacy_folder) {
                Ok(Some(spl_file)) => {
                    let spl = match legacy::parse_legacy_seating_file(&spl_file) {
                        Ok(v) => v,
                        Err(e) => {
                            let _ = tx.rollback();
                            return json!(ErrResp {
                                id: req.id,
                                ok: false,
                                error: ErrObj {
                                    code: "legacy_parse_failed".into(),
                                    message: e.to_string(),
                                    details: Some(
                                        json!({ "seatingFile": spl_file.to_string_lossy() })
                                    )
                                }
                            });
                        }
                    };

                    if let Err(e) = tx.execute(
                        "INSERT INTO seating_plans(class_id, rows, seats_per_row, blocked_mask)
                         VALUES(?, ?, ?, ?)
                         ON CONFLICT(class_id) DO UPDATE SET
                           rows = excluded.rows,
                           seats_per_row = excluded.seats_per_row,
                           blocked_mask = excluded.blocked_mask",
                        (
                            &class_id,
                            spl.rows as i64,
                            spl.seats_per_row as i64,
                            &spl.blocked_mask,
                        ),
                    ) {
                        let _ = tx.rollback();
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "db_insert_failed".into(),
                                message: e.to_string(),
                                details: Some(json!({ "table": "seating_plans" }))
                            }
                        });
                    }
                    if let Err(e) = tx.execute(
                        "DELETE FROM seating_assignments WHERE class_id = ?",
                        [&class_id],
                    ) {
                        let _ = tx.rollback();
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "db_delete_failed".into(),
                                message: e.to_string(),
                                details: Some(json!({ "table": "seating_assignments" }))
                            }
                        });
                    }
                    let max_students =
                        std::cmp::min(student_ids_by_sort.len(), spl.seat_codes.len());
                    for s_idx in 0..max_students {
                        let seat_code = spl.seat_codes[s_idx];
                        if seat_code <= 0 {
                            continue;
                        }
                        let student_id = &student_ids_by_sort[s_idx];
                        if let Err(e) = tx.execute(
                            "INSERT INTO seating_assignments(class_id, student_id, seat_code)
                             VALUES(?, ?, ?)",
                            (&class_id, student_id, seat_code as i64),
                        ) {
                            let _ = tx.rollback();
                            return json!(ErrResp {
                                id: req.id,
                                ok: false,
                                error: ErrObj {
                                    code: "db_insert_failed".into(),
                                    message: e.to_string(),
                                    details: Some(json!({ "table": "seating_assignments" }))
                                }
                            });
                        }
                    }
                    seating_imported = true;
                }
                Ok(None) => {
                    warnings.push(json!({
                        "code": "legacy_missing_seating_file",
                        "folder": legacy_folder.to_string_lossy()
                    }));
                }
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
            }

            // Best-effort ICC import (device/class codes matrix).
            match legacy::find_icc_file(&legacy_folder) {
                Ok(Some(icc_file)) => {
                    let icc = match legacy::parse_legacy_icc_file(&icc_file) {
                        Ok(v) => v,
                        Err(e) => {
                            let _ = tx.rollback();
                            return json!(ErrResp {
                                id: req.id,
                                ok: false,
                                error: ErrObj {
                                    code: "legacy_parse_failed".into(),
                                    message: e.to_string(),
                                    details: Some(json!({ "iccFile": icc_file.to_string_lossy() }))
                                }
                            });
                        }
                    };

                    let max_students = std::cmp::min(student_ids_by_sort.len(), icc.last_student);
                    for s_idx in 0..max_students {
                        let student_id = &student_ids_by_sort[s_idx];
                        let codes_row = icc
                            .codes
                            .get(s_idx + 1)
                            .cloned()
                            .unwrap_or_else(|| vec![String::new(); icc.subject_count + 1]);
                        let primary_code = codes_row
                            .iter()
                            .skip(1)
                            .map(|s| s.trim())
                            .find(|s| !s.is_empty())
                            .unwrap_or("")
                            .to_string();
                        let raw_line = serde_json::to_string(&json!({
                            "subjectCount": icc.subject_count,
                            "codes": codes_row
                        }))
                        .unwrap_or_else(|_| "[]".to_string());
                        let did = Uuid::new_v4().to_string();
                        if let Err(e) = tx.execute(
                            "INSERT INTO student_device_map(id, class_id, student_id, device_code, raw_line)
                             VALUES(?, ?, ?, ?, ?)
                             ON CONFLICT(class_id, student_id) DO UPDATE SET
                               device_code = excluded.device_code,
                               raw_line = excluded.raw_line",
                            (&did, &class_id, student_id, &primary_code, &raw_line),
                        ) {
                            let _ = tx.rollback();
                            return json!(ErrResp {
                                id: req.id,
                                ok: false,
                                error: ErrObj {
                                    code: "db_insert_failed".into(),
                                    message: e.to_string(),
                                    details: Some(json!({ "table": "student_device_map" }))
                                }
                            });
                        }
                        device_mappings_imported += 1;
                    }
                }
                Ok(None) => {
                    warnings.push(json!({
                        "code": "legacy_missing_icc_file",
                        "folder": legacy_folder.to_string_lossy()
                    }));
                }
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
            }

            // Best-effort bank import from parent fixture folder.
            let bnk_folder = legacy_folder
                .parent()
                .unwrap_or(&legacy_folder)
                .to_path_buf();
            match legacy::find_bnk_files(&bnk_folder) {
                Ok(files) => {
                    for bnk_file in files {
                        let parsed_bnk = match legacy::parse_bnk_file(&bnk_file) {
                            Ok(v) => v,
                            Err(e) => {
                                let _ = tx.rollback();
                                return json!(ErrResp {
                                    id: req.id,
                                    ok: false,
                                    error: ErrObj {
                                        code: "legacy_parse_failed".into(),
                                        message: e.to_string(),
                                        details: Some(
                                            json!({ "bnkFile": bnk_file.to_string_lossy() })
                                        )
                                    }
                                });
                            }
                        };
                        let short_name = bnk_file
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_string();
                        if short_name.is_empty() {
                            continue;
                        }
                        let bank_id = Uuid::new_v4().to_string();
                        if let Err(e) = tx.execute(
                            "INSERT INTO comment_banks(id, short_name, is_default, fit_profile, source_path)
                             VALUES(?, ?, 0, ?, ?)
                             ON CONFLICT(short_name) DO UPDATE SET
                               fit_profile = excluded.fit_profile,
                               source_path = excluded.source_path",
                            (
                                &bank_id,
                                &short_name,
                                parsed_bnk.fit_profile.as_deref(),
                                bnk_file.to_string_lossy().as_ref(),
                            ),
                        ) {
                            let _ = tx.rollback();
                            return json!(ErrResp {
                                id: req.id,
                                ok: false,
                                error: ErrObj {
                                    code: "db_insert_failed".into(),
                                    message: e.to_string(),
                                    details: Some(json!({ "table": "comment_banks" }))
                                }
                            });
                        }

                        let resolved_bank_id: String = match tx.query_row(
                            "SELECT id FROM comment_banks WHERE short_name = ?",
                            [&short_name],
                            |r| r.get(0),
                        ) {
                            Ok(v) => v,
                            Err(e) => {
                                let _ = tx.rollback();
                                return json!(ErrResp {
                                    id: req.id,
                                    ok: false,
                                    error: ErrObj {
                                        code: "db_query_failed".into(),
                                        message: e.to_string(),
                                        details: Some(json!({ "table": "comment_banks" }))
                                    }
                                });
                            }
                        };

                        if let Err(e) = tx.execute(
                            "DELETE FROM comment_bank_entries WHERE bank_id = ?",
                            [&resolved_bank_id],
                        ) {
                            let _ = tx.rollback();
                            return json!(ErrResp {
                                id: req.id,
                                ok: false,
                                error: ErrObj {
                                    code: "db_delete_failed".into(),
                                    message: e.to_string(),
                                    details: Some(json!({ "table": "comment_bank_entries" }))
                                }
                            });
                        }

                        for (sort_order, entry) in parsed_bnk.entries.iter().enumerate() {
                            let eid = Uuid::new_v4().to_string();
                            if let Err(e) = tx.execute(
                                "INSERT INTO comment_bank_entries(id, bank_id, sort_order, type_code, level_code, text)
                                 VALUES(?, ?, ?, ?, ?, ?)",
                                (
                                    &eid,
                                    &resolved_bank_id,
                                    sort_order as i64,
                                    &entry.type_code,
                                    &entry.level_code,
                                    &entry.text,
                                ),
                            ) {
                                let _ = tx.rollback();
                                return json!(ErrResp {
                                    id: req.id,
                                    ok: false,
                                    error: ErrObj {
                                        code: "db_insert_failed".into(),
                                        message: e.to_string(),
                                        details: Some(json!({ "table": "comment_bank_entries" }))
                                    }
                                });
                            }
                        }

                        banks_imported += 1;
                    }
                }
                Err(e) => {
                    let _ = tx.rollback();
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "legacy_read_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "folder": bnk_folder.to_string_lossy() }))
                        }
                    });
                }
            }

            let mut mark_sets_imported = 0usize;
            let mut assessments_imported = 0usize;
            let mut scores_imported = 0usize;
            let mut imported_mark_files: Vec<String> = Vec::new();
            let mut missing_mark_files: Vec<serde_json::Value> = Vec::new();
            let mut mark_set_id_by_source_stem: HashMap<String, String> = HashMap::new();

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
                if let Some(stem) = mark_file.file_stem().and_then(|s| s.to_str()) {
                    mark_set_id_by_source_stem
                        .insert(stem.to_ascii_uppercase(), mark_set_id.clone());
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

                // Best-effort import IDX + per-set Rn files for comment sets.
                let idx_file = mark_file.with_extension("IDX");
                if idx_file.is_file() {
                    let parsed_idx = match legacy::parse_legacy_idx_file(&idx_file) {
                        Ok(v) => v,
                        Err(e) => {
                            return json!(ErrResp {
                                id: req.id,
                                ok: false,
                                error: ErrObj {
                                    code: "legacy_parse_failed".into(),
                                    message: e.to_string(),
                                    details: Some(json!({ "idxFile": idx_file.to_string_lossy() }))
                                }
                            });
                        }
                    };

                    // Clear existing imported sets for this mark set before writing.
                    if let Err(e) = tx.execute(
                        "DELETE FROM comment_set_remarks
                         WHERE comment_set_index_id IN (
                           SELECT id FROM comment_set_indexes WHERE mark_set_id = ?
                         )",
                        [&mark_set_id],
                    ) {
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "db_delete_failed".into(),
                                message: e.to_string(),
                                details: Some(json!({ "table": "comment_set_remarks" }))
                            }
                        });
                    }
                    if let Err(e) = tx.execute(
                        "DELETE FROM comment_set_indexes WHERE mark_set_id = ?",
                        [&mark_set_id],
                    ) {
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "db_delete_failed".into(),
                                message: e.to_string(),
                                details: Some(json!({ "table": "comment_set_indexes" }))
                            }
                        });
                    }

                    let idx_bank_short = parsed_idx.bank_short.clone();
                    for set in parsed_idx.sets {
                        let csi_id = Uuid::new_v4().to_string();
                        let bank_short = set
                            .bank_short
                            .clone()
                            .or_else(|| idx_bank_short.clone())
                            .map(|s| s.trim().to_string())
                            .and_then(|s| if s.is_empty() { None } else { Some(s) });
                        if let Err(e) = tx.execute(
                            "INSERT INTO comment_set_indexes(
                               id,
                               class_id,
                               mark_set_id,
                               set_number,
                               title,
                               fit_mode,
                               fit_font_size,
                               fit_width,
                               fit_lines,
                               fit_subj,
                               max_chars,
                               is_default,
                               bank_short
                             ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                            (
                                &csi_id,
                                &class_id,
                                &mark_set_id,
                                set.set_number as i64,
                                &set.title,
                                set.fit_mode as i64,
                                set.fit_font_size as i64,
                                set.fit_width as i64,
                                set.fit_lines as i64,
                                &set.fit_subj,
                                set.max_chars as i64,
                                if set.is_default { 1 } else { 0 },
                                bank_short.as_deref(),
                            ),
                        ) {
                            return json!(ErrResp {
                                id: req.id,
                                ok: false,
                                error: ErrObj {
                                    code: "db_insert_failed".into(),
                                    message: e.to_string(),
                                    details: Some(json!({ "table": "comment_set_indexes" }))
                                }
                            });
                        }

                        comment_sets_imported += 1;

                        let r_file = mark_file.with_extension(format!("R{}", set.set_number));
                        if !r_file.is_file() {
                            continue;
                        }
                        let parsed_r = match legacy::parse_legacy_r_comment_file(&r_file) {
                            Ok(v) => v,
                            Err(e) => {
                                return json!(ErrResp {
                                    id: req.id,
                                    ok: false,
                                    error: ErrObj {
                                        code: "legacy_parse_failed".into(),
                                        message: e.to_string(),
                                        details: Some(
                                            json!({ "remarkFile": r_file.to_string_lossy() })
                                        )
                                    }
                                });
                            }
                        };
                        let max_students =
                            std::cmp::min(student_ids_by_sort.len(), parsed_r.remarks.len());
                        for s_idx in 0..max_students {
                            let remark = parsed_r.remarks[s_idx].trim().to_string();
                            if remark.is_empty() {
                                continue;
                            }
                            let rid = Uuid::new_v4().to_string();
                            let student_id = &student_ids_by_sort[s_idx];
                            if let Err(e) = tx.execute(
                                "INSERT INTO comment_set_remarks(id, comment_set_index_id, student_id, remark)
                                 VALUES(?, ?, ?, ?)
                                 ON CONFLICT(comment_set_index_id, student_id) DO UPDATE SET
                                   remark = excluded.remark",
                                (&rid, &csi_id, student_id, &remark),
                            ) {
                                return json!(ErrResp {
                                    id: req.id,
                                    ok: false,
                                    error: ErrObj {
                                        code: "db_insert_failed".into(),
                                        message: e.to_string(),
                                        details: Some(json!({ "table": "comment_set_remarks" }))
                                    }
                                });
                            }
                            comment_remarks_imported += 1;
                        }
                    }
                }

                mark_sets_imported += 1;
                assessments_imported += parsed_mark.assessments.len();
                imported_mark_files.push(mark_filename);
            }

            // Best-effort import TBK companion files (loaned items).
            match legacy::find_tbk_files(&legacy_folder) {
                Ok(tbk_files) => {
                    if tbk_files.is_empty() {
                        warnings.push(json!({
                            "code": "legacy_missing_tbk_file",
                            "folder": legacy_folder.to_string_lossy()
                        }));
                    }
                    for tbk_file in tbk_files {
                        let parsed_tbk = match legacy::parse_legacy_tbk_file(&tbk_file) {
                            Ok(v) => v,
                            Err(e) => {
                                let _ = tx.rollback();
                                return json!(ErrResp {
                                    id: req.id,
                                    ok: false,
                                    error: ErrObj {
                                        code: "legacy_parse_failed".into(),
                                        message: e.to_string(),
                                        details: Some(
                                            json!({ "tbkFile": tbk_file.to_string_lossy() })
                                        )
                                    }
                                });
                            }
                        };
                        let source_stem = tbk_file
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_ascii_uppercase();
                        let mark_set_id = mark_set_id_by_source_stem.get(&source_stem).cloned();
                        let max_students =
                            std::cmp::min(student_ids_by_sort.len(), parsed_tbk.last_student);
                        for item in parsed_tbk.items {
                            for s_idx in 0..max_students {
                                let item_id = item
                                    .assignments
                                    .get(s_idx)
                                    .map(|a| a.item_id.trim().to_string())
                                    .unwrap_or_default();
                                let note = item
                                    .assignments
                                    .get(s_idx)
                                    .map(|a| a.note.trim().to_string())
                                    .unwrap_or_default();
                                if item_id.is_empty() && note.is_empty() {
                                    continue;
                                }
                                let raw_line = serde_json::to_string(&json!({
                                    "title": item.title,
                                    "publisher": item.publisher,
                                    "cost": item.cost,
                                    "itemId": item_id,
                                    "note": note
                                }))
                                .unwrap_or_else(|_| "{}".to_string());
                                let loaned_id = Uuid::new_v4().to_string();
                                let student_id = &student_ids_by_sort[s_idx];
                                if let Err(e) = tx.execute(
                                    "INSERT INTO loaned_items(id, class_id, student_id, mark_set_id, item_name, quantity, notes, raw_line)
                                     VALUES(?, ?, ?, ?, ?, ?, ?, ?)",
                                    (
                                        &loaned_id,
                                        &class_id,
                                        student_id,
                                        mark_set_id.as_deref(),
                                        &item.title,
                                        item.cost,
                                        if note.is_empty() { None } else { Some(note.as_str()) },
                                        &raw_line,
                                    ),
                                ) {
                                    let _ = tx.rollback();
                                    return json!(ErrResp {
                                        id: req.id,
                                        ok: false,
                                        error: ErrObj {
                                            code: "db_insert_failed".into(),
                                            message: e.to_string(),
                                            details: Some(json!({ "table": "loaned_items" }))
                                        }
                                    });
                                }
                                loaned_items_imported += 1;
                            }
                        }
                    }
                }
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
            }

            // Best-effort merge ALL!<class>.IDX combined comment sets.
            match legacy::find_all_idx_file(&legacy_folder) {
                Ok(Some(all_idx_file)) => {
                    let parsed_idx = match legacy::parse_legacy_idx_file(&all_idx_file) {
                        Ok(v) => v,
                        Err(e) => {
                            let _ = tx.rollback();
                            return json!(ErrResp {
                                id: req.id,
                                ok: false,
                                error: ErrObj {
                                    code: "legacy_parse_failed".into(),
                                    message: e.to_string(),
                                    details: Some(
                                        json!({ "idxFile": all_idx_file.to_string_lossy() })
                                    )
                                }
                            });
                        }
                    };

                    let mut mark_set_ids: Vec<String> =
                        mark_set_id_by_source_stem.values().cloned().collect();
                    mark_set_ids.sort();
                    mark_set_ids.dedup();

                    let idx_bank_short = parsed_idx.bank_short.clone();
                    for mark_set_id in mark_set_ids {
                        for set in &parsed_idx.sets {
                            let existing_id: Option<String> = match tx
                                .query_row(
                                    "SELECT id FROM comment_set_indexes WHERE mark_set_id = ? AND set_number = ?",
                                    (&mark_set_id, set.set_number as i64),
                                    |r| r.get(0),
                                )
                                .optional()
                            {
                                Ok(v) => v,
                                Err(e) => {
                                    let _ = tx.rollback();
                                    return json!(ErrResp {
                                        id: req.id,
                                        ok: false,
                                        error: ErrObj {
                                            code: "db_query_failed".into(),
                                            message: e.to_string(),
                                            details: Some(json!({ "table": "comment_set_indexes" }))
                                        }
                                    });
                                }
                            };
                            let target_set_number = if existing_id.is_some() {
                                match tx.query_row(
                                    "SELECT COALESCE(MAX(set_number), 0) FROM comment_set_indexes WHERE mark_set_id = ?",
                                    [&mark_set_id],
                                    |r| r.get::<_, i64>(0),
                                ) {
                                    Ok(v) => v + 1,
                                    Err(e) => {
                                        let _ = tx.rollback();
                                        return json!(ErrResp {
                                            id: req.id,
                                            ok: false,
                                            error: ErrObj {
                                                code: "db_query_failed".into(),
                                                message: e.to_string(),
                                                details: Some(json!({ "table": "comment_set_indexes" }))
                                            }
                                        });
                                    }
                                }
                            } else {
                                set.set_number as i64
                            };

                            let csi_id = Uuid::new_v4().to_string();
                            let bank_short = set
                                .bank_short
                                .clone()
                                .or_else(|| idx_bank_short.clone())
                                .map(|s| s.trim().to_string())
                                .and_then(|s| if s.is_empty() { None } else { Some(s) });
                            if let Err(e) = tx.execute(
                                "INSERT INTO comment_set_indexes(
                                   id,
                                   class_id,
                                   mark_set_id,
                                   set_number,
                                   title,
                                   fit_mode,
                                   fit_font_size,
                                   fit_width,
                                   fit_lines,
                                   fit_subj,
                                   max_chars,
                                   is_default,
                                   bank_short
                                 ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                                (
                                    &csi_id,
                                    &class_id,
                                    &mark_set_id,
                                    target_set_number,
                                    &set.title,
                                    set.fit_mode as i64,
                                    set.fit_font_size as i64,
                                    set.fit_width as i64,
                                    set.fit_lines as i64,
                                    &set.fit_subj,
                                    set.max_chars as i64,
                                    if set.is_default { 1 } else { 0 },
                                    bank_short.as_deref(),
                                ),
                            ) {
                                let _ = tx.rollback();
                                return json!(ErrResp {
                                    id: req.id,
                                    ok: false,
                                    error: ErrObj {
                                        code: "db_insert_failed".into(),
                                        message: e.to_string(),
                                        details: Some(json!({ "table": "comment_set_indexes" }))
                                    }
                                });
                            }
                            comment_sets_imported += 1;
                            combined_comment_sets_imported += 1;

                            let r_file =
                                all_idx_file.with_extension(format!("R{}", set.set_number));
                            if !r_file.is_file() {
                                continue;
                            }
                            let parsed_r = match legacy::parse_legacy_r_comment_file(&r_file) {
                                Ok(v) => v,
                                Err(e) => {
                                    let _ = tx.rollback();
                                    return json!(ErrResp {
                                        id: req.id,
                                        ok: false,
                                        error: ErrObj {
                                            code: "legacy_parse_failed".into(),
                                            message: e.to_string(),
                                            details: Some(
                                                json!({ "remarkFile": r_file.to_string_lossy() })
                                            )
                                        }
                                    });
                                }
                            };
                            let max_students =
                                std::cmp::min(student_ids_by_sort.len(), parsed_r.remarks.len());
                            for s_idx in 0..max_students {
                                let remark = parsed_r.remarks[s_idx].trim().to_string();
                                if remark.is_empty() {
                                    continue;
                                }
                                let rid = Uuid::new_v4().to_string();
                                let student_id = &student_ids_by_sort[s_idx];
                                if let Err(e) = tx.execute(
                                    "INSERT INTO comment_set_remarks(id, comment_set_index_id, student_id, remark)
                                     VALUES(?, ?, ?, ?)
                                     ON CONFLICT(comment_set_index_id, student_id) DO UPDATE SET
                                       remark = excluded.remark",
                                    (&rid, &csi_id, student_id, &remark),
                                ) {
                                    let _ = tx.rollback();
                                    return json!(ErrResp {
                                        id: req.id,
                                        ok: false,
                                        error: ErrObj {
                                            code: "db_insert_failed".into(),
                                            message: e.to_string(),
                                            details: Some(json!({ "table": "comment_set_remarks" }))
                                        }
                                    });
                                }
                                comment_remarks_imported += 1;
                            }
                        }
                    }
                }
                Ok(None) => {
                    warnings.push(json!({
                        "code": "legacy_missing_all_idx_file",
                        "folder": legacy_folder.to_string_lossy()
                    }));
                }
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
                    "attendanceImported": attendance_imported,
                    "seatingImported": seating_imported,
                    "banksImported": banks_imported,
                    "commentSetsImported": comment_sets_imported,
                    "commentRemarksImported": comment_remarks_imported,
                    "loanedItemsImported": loaned_items_imported,
                    "deviceMappingsImported": device_mappings_imported,
                    "combinedCommentSetsImported": combined_comment_sets_imported,
                    "sourceClFile": cl_file.to_string_lossy(),
                    "importedMarkFiles": imported_mark_files,
                    "missingMarkFiles": missing_mark_files,
                    "warnings": warnings,
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
            let (raw_value, status) = match resolve_score_state(None, value) {
                Ok(v) => v,
                Err(err) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: err
                    })
                }
            };

            let student_id = match resolve_student_id_by_row(conn, &class_id, row) {
                Ok(v) => v,
                Err(err) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: err
                    })
                }
            };

            let assessment_id = match resolve_assessment_id_by_col(conn, &mark_set_id, col) {
                Ok(v) => v,
                Err(err) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: err
                    })
                }
            };

            if let Err(err) = upsert_score(conn, &assessment_id, &student_id, raw_value, status) {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: err
                });
            }

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({ "ok": true })
            })
        }
        "grid.setState" => {
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
            let state_value = req.params.get("state").and_then(|v| v.as_str());
            let value = req.params.get("value").and_then(|v| v.as_f64());
            let (raw_value, status) = match resolve_score_state(state_value, value) {
                Ok(v) => v,
                Err(err) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: err
                    })
                }
            };

            let student_id = match resolve_student_id_by_row(conn, &class_id, row) {
                Ok(v) => v,
                Err(err) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: err
                    })
                }
            };
            let assessment_id = match resolve_assessment_id_by_col(conn, &mark_set_id, col) {
                Ok(v) => v,
                Err(err) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: err
                    })
                }
            };
            if let Err(err) = upsert_score(conn, &assessment_id, &student_id, raw_value, status) {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: err
                });
            }

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({ "ok": true })
            })
        }
        "grid.bulkUpdate" => {
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
            let Some(edits_arr) = req.params.get("edits").and_then(|v| v.as_array()) else {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "bad_params".into(),
                        message: "missing edits[]".into(),
                        details: None
                    }
                });
            };

            let mut updated: usize = 0;
            for edit in edits_arr {
                let Some(obj) = edit.as_object() else {
                    continue;
                };
                let Some(row) = obj.get("row").and_then(|v| v.as_i64()) else {
                    continue;
                };
                let Some(col) = obj.get("col").and_then(|v| v.as_i64()) else {
                    continue;
                };
                if row < 0 || col < 0 {
                    continue;
                }
                let state_value = obj.get("state").and_then(|v| v.as_str());
                let value = obj.get("value").and_then(|v| v.as_f64());
                let (raw_value, status) = match resolve_score_state(state_value, value) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let student_id = match resolve_student_id_by_row(conn, &class_id, row) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let assessment_id = match resolve_assessment_id_by_col(conn, &mark_set_id, col) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                if upsert_score(conn, &assessment_id, &student_id, raw_value, status).is_ok() {
                    updated += 1;
                }
            }

            json!(OkResp {
                id: req.id,
                ok: true,
                result: json!({ "ok": true, "updated": updated })
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

            let assessments = match calc::compute_assessment_stats(
                &calc::CalcContext {
                    conn,
                    class_id: &class_id,
                    mark_set_id: &mark_set_id,
                },
                &filters,
            ) {
                Ok(v) => v,
                Err(err) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: calc_err_to_err_obj(err)
                    })
                }
            };
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
                    result: json!(result)
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

            let filters = calc::SummaryFilters::default();
            match compute_markset_summary(conn, &class_id, &mark_set_id, &filters) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result: json!(result)
                }),
                Err(err) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: err
                }),
            }
        }
        "reports.categoryAnalysisModel" => {
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
                Ok(summary) => {
                    let result = json!({
                        "class": summary.class,
                        "markSet": summary.mark_set,
                        "settings": summary.settings,
                        "filters": summary.filters,
                        "categories": summary.categories,
                        "perCategory": summary.per_category,
                        "perAssessment": summary.per_assessment,
                    });
                    json!(OkResp {
                        id: req.id,
                        ok: true,
                        result
                    })
                }
                Err(err) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: err
                }),
            }
        }
        "reports.studentSummaryModel" => {
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
                Ok(summary) => {
                    let student = summary
                        .per_student
                        .iter()
                        .find(|s| s.student_id == student_id)
                        .cloned();
                    let Some(student) = student else {
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "not_found".into(),
                                message: "student not found in mark set".into(),
                                details: None
                            }
                        });
                    };

                    let result = json!({
                        "class": summary.class,
                        "markSet": summary.mark_set,
                        "settings": summary.settings,
                        "filters": summary.filters,
                        "student": student,
                        "assessments": summary.assessments,
                        "perAssessment": summary.per_assessment,
                    });
                    json!(OkResp {
                        id: req.id,
                        ok: true,
                        result
                    })
                }
                Err(err) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: err
                }),
            }
        }
        "reports.attendanceMonthlyModel" => {
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
            let month = match req.params.get("month").and_then(|v| v.as_str()) {
                Some(v) => v.to_string(),
                None => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "bad_params".into(),
                            message: "missing month".into(),
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
            match attendance_month_open(conn, &json!({ "classId": class_id, "month": month })) {
                Ok(model) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result: json!({
                        "class": { "id": class_id, "name": class_name },
                        "attendance": model
                    })
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
                }),
            }
        }
        "reports.classListModel" => {
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

            let mut stmt = match conn.prepare(
                "SELECT s.id, s.last_name, s.first_name, s.student_no, s.birth_date, s.active, s.sort_order, sn.note
                 FROM students s
                 LEFT JOIN student_notes sn
                   ON sn.class_id = s.class_id AND sn.student_id = s.id
                 WHERE s.class_id = ?
                 ORDER BY s.sort_order",
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
            let students = match stmt
                .query_map([&class_id], |r| {
                    let id: String = r.get(0)?;
                    let last: String = r.get(1)?;
                    let first: String = r.get(2)?;
                    let student_no: Option<String> = r.get(3)?;
                    let birth_date: Option<String> = r.get(4)?;
                    let active: i64 = r.get(5)?;
                    let sort_order: i64 = r.get(6)?;
                    let note: Option<String> = r.get(7)?;
                    Ok(json!({
                        "id": id,
                        "displayName": format!("{}, {}", last, first),
                        "studentNo": student_no,
                        "birthDate": birth_date,
                        "active": active != 0,
                        "sortOrder": sort_order,
                        "note": note.unwrap_or_default()
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
                    "class": { "id": class_id, "name": class_name },
                    "students": students
                })
            })
        }
        "reports.learningSkillsSummaryModel" => {
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
            match learning_skills_report_model(conn, &req.params) {
                Ok(result) => json!(OkResp {
                    id: req.id,
                    ok: true,
                    result
                }),
                Err(error) => json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error
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

fn calc_err_to_err_obj(err: calc::CalcError) -> ErrObj {
    ErrObj {
        code: err.code,
        message: err.message,
        details: err.details,
    }
}

fn parse_summary_filters(raw: Option<&serde_json::Value>) -> Result<calc::SummaryFilters, ErrObj> {
    calc::parse_summary_filters(raw).map_err(calc_err_to_err_obj)
}

fn compute_markset_summary(
    conn: &Connection,
    class_id: &str,
    mark_set_id: &str,
    filters: &calc::SummaryFilters,
) -> Result<calc::SummaryModel, ErrObj> {
    calc::compute_mark_set_summary(
        &calc::CalcContext {
            conn,
            class_id,
            mark_set_id,
        },
        filters,
    )
    .map_err(calc_err_to_err_obj)
}

#[derive(Debug, Clone)]
struct BasicStudent {
    id: String,
    display_name: String,
    sort_order: i64,
    active: bool,
}

fn get_required_str(params: &serde_json::Value, key: &str) -> Result<String, ErrObj> {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| ErrObj {
            code: "bad_params".into(),
            message: format!("missing {}", key),
            details: None,
        })
}

fn list_students_for_class(conn: &Connection, class_id: &str) -> Result<Vec<BasicStudent>, ErrObj> {
    let mut stmt = conn
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
    .map_err(|e| ErrObj {
        code: "db_query_failed".into(),
        message: e.to_string(),
        details: None,
    })
}

fn class_exists(conn: &Connection, class_id: &str) -> Result<bool, ErrObj> {
    conn.query_row("SELECT 1 FROM classes WHERE id = ?", [class_id], |r| {
        r.get::<_, i64>(0)
    })
    .optional()
    .map(|v| v.is_some())
    .map_err(|e| ErrObj {
        code: "db_query_failed".into(),
        message: e.to_string(),
        details: None,
    })
}

fn mark_set_exists(conn: &Connection, class_id: &str, mark_set_id: &str) -> Result<bool, ErrObj> {
    conn.query_row(
        "SELECT 1 FROM mark_sets WHERE id = ? AND class_id = ?",
        (mark_set_id, class_id),
        |r| r.get::<_, i64>(0),
    )
    .optional()
    .map(|v| v.is_some())
    .map_err(|e| ErrObj {
        code: "db_query_failed".into(),
        message: e.to_string(),
        details: None,
    })
}

fn resolve_score_state(
    explicit_state: Option<&str>,
    value: Option<f64>,
) -> Result<(Option<f64>, &'static str), ErrObj> {
    if let Some(v) = value {
        if v < 0.0 {
            return Err(ErrObj {
                code: "bad_params".into(),
                message: "negative marks are not allowed".into(),
                details: Some(json!({ "value": v })),
            });
        }
    }

    match explicit_state.map(|s| s.to_ascii_lowercase()) {
        Some(s) if s == "no_mark" => Ok((Some(0.0), "no_mark")),
        Some(s) if s == "zero" => Ok((None, "zero")),
        Some(s) if s == "scored" => {
            let Some(v) = value else {
                return Err(ErrObj {
                    code: "bad_params".into(),
                    message: "scored state requires numeric value".into(),
                    details: None,
                });
            };
            if v <= 0.0 {
                return Err(ErrObj {
                    code: "bad_params".into(),
                    message: "scored marks must be > 0".into(),
                    details: Some(json!({ "value": v })),
                });
            }
            Ok((Some(v), "scored"))
        }
        Some(other) => Err(ErrObj {
            code: "bad_params".into(),
            message: "state must be one of: scored, zero, no_mark".into(),
            details: Some(json!({ "state": other })),
        }),
        None => {
            // Legacy parity default for grid edits:
            // blank/null/0 => no_mark, positive => scored.
            match value {
                Some(v) if v > 0.0 => Ok((Some(v), "scored")),
                _ => Ok((Some(0.0), "no_mark")),
            }
        }
    }
}

fn resolve_student_id_by_row(
    conn: &Connection,
    class_id: &str,
    row: i64,
) -> Result<String, ErrObj> {
    let student_id: Option<String> = conn
        .query_row(
            "SELECT id FROM students WHERE class_id = ? AND sort_order = ?",
            (class_id, row),
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    student_id.ok_or_else(|| ErrObj {
        code: "not_found".into(),
        message: "student not found".into(),
        details: Some(json!({ "row": row })),
    })
}

fn resolve_assessment_id_by_col(
    conn: &Connection,
    mark_set_id: &str,
    col: i64,
) -> Result<String, ErrObj> {
    let assessment_id: Option<String> = conn
        .query_row(
            "SELECT id FROM assessments WHERE mark_set_id = ? AND idx = ?",
            (mark_set_id, col),
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    assessment_id.ok_or_else(|| ErrObj {
        code: "not_found".into(),
        message: "assessment not found".into(),
        details: Some(json!({ "col": col })),
    })
}

fn upsert_score(
    conn: &Connection,
    assessment_id: &str,
    student_id: &str,
    raw_value: Option<f64>,
    status: &str,
) -> Result<(), ErrObj> {
    let score_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO scores(id, assessment_id, student_id, raw_value, status)
         VALUES(?, ?, ?, ?, ?)
         ON CONFLICT(assessment_id, student_id) DO UPDATE SET
           raw_value = excluded.raw_value,
           status = excluded.status",
        (&score_id, assessment_id, student_id, raw_value, status),
    )
    .map_err(|e| ErrObj {
        code: "db_insert_failed".into(),
        message: e.to_string(),
        details: Some(json!({ "table": "scores" })),
    })?;
    Ok(())
}

fn csv_quote(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn parse_csv_record(line: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut buf = String::new();
    let mut in_quotes = false;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '"' {
            if in_quotes && i + 1 < chars.len() && chars[i + 1] == '"' {
                buf.push('"');
                i += 2;
                continue;
            }
            in_quotes = !in_quotes;
            i += 1;
            continue;
        }
        if ch == ',' && !in_quotes {
            out.push(buf);
            buf = String::new();
            i += 1;
            continue;
        }
        buf.push(ch);
        i += 1;
    }
    out.push(buf);
    out
}

fn parse_month_key(month: &str) -> Result<(i32, u32), ErrObj> {
    let t = month.trim();
    if let Ok(m) = t.parse::<u32>() {
        if (1..=12).contains(&m) {
            return Ok((2001, m));
        }
    }
    let Some((y, m)) = t.split_once('-') else {
        return Err(ErrObj {
            code: "bad_params".into(),
            message: "month must be MM or YYYY-MM".into(),
            details: None,
        });
    };
    let year = y.parse::<i32>().map_err(|_| ErrObj {
        code: "bad_params".into(),
        message: "month year must be numeric".into(),
        details: None,
    })?;
    let month_num = m.parse::<u32>().map_err(|_| ErrObj {
        code: "bad_params".into(),
        message: "month must be YYYY-MM".into(),
        details: None,
    })?;
    if !(1..=12).contains(&month_num) {
        return Err(ErrObj {
            code: "bad_params".into(),
            message: "month must be between 01 and 12".into(),
            details: None,
        });
    }
    Ok((year, month_num))
}

fn days_in_month(year: i32, month: u32) -> usize {
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 30,
    }
}

fn normalize_day_codes(raw: &str, days: usize) -> String {
    let mut chars: Vec<char> = raw.chars().collect();
    if chars.len() < days {
        chars.extend(std::iter::repeat(' ').take(days - chars.len()));
    } else if chars.len() > days {
        chars.truncate(days);
    }
    chars.into_iter().collect()
}

fn patch_day_code(existing: &str, days: usize, day: usize, code: Option<char>) -> String {
    let mut chars: Vec<char> = normalize_day_codes(existing, days).chars().collect();
    let idx = day.saturating_sub(1);
    if idx < chars.len() {
        chars[idx] = code.unwrap_or(' ');
    }
    chars.into_iter().collect()
}

fn parse_optional_code_char(v: Option<&serde_json::Value>) -> Result<Option<char>, ErrObj> {
    let Some(v) = v else { return Ok(None) };
    if v.is_null() {
        return Ok(None);
    }
    let Some(s) = v.as_str() else {
        return Err(ErrObj {
            code: "bad_params".into(),
            message: "code must be string or null".into(),
            details: None,
        });
    };
    let t = s.trim();
    if t.is_empty() {
        return Ok(None);
    }
    Ok(t.chars().next())
}

fn attendance_month_open(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, ErrObj> {
    let class_id = get_required_str(params, "classId")?;
    let month_key = get_required_str(params, "month")?;
    let (year, month_num) = parse_month_key(&month_key)?;
    let days = days_in_month(year, month_num);

    if !class_exists(conn, &class_id)? {
        return Err(ErrObj {
            code: "not_found".into(),
            message: "class not found".into(),
            details: None,
        });
    }
    let students = list_students_for_class(conn, &class_id)?;
    let school_year_start_month: i64 = conn
        .query_row(
            "SELECT school_year_start_month FROM attendance_settings WHERE class_id = ?",
            [&class_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?
        .unwrap_or(9);

    let type_of_day_codes_raw: Option<String> = conn
        .query_row(
            "SELECT type_of_day_codes FROM attendance_months WHERE class_id = ? AND month = ?",
            (&class_id, &month_key),
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    let type_of_day_codes =
        normalize_day_codes(type_of_day_codes_raw.as_deref().unwrap_or(""), days);

    let mut by_student: HashMap<String, String> = HashMap::new();
    let mut stmt = conn
        .prepare(
            "SELECT student_id, day_codes
             FROM attendance_student_months
             WHERE class_id = ? AND month = ?",
        )
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    let rows = stmt
        .query_map((&class_id, &month_key), |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    for (student_id, day_codes) in rows {
        by_student.insert(student_id, normalize_day_codes(&day_codes, days));
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
            let day_codes = by_student
                .get(&s.id)
                .cloned()
                .unwrap_or_else(|| normalize_day_codes("", days));
            json!({
                "studentId": s.id,
                "dayCodes": day_codes
            })
        })
        .collect();

    Ok(json!({
        "schoolYearStartMonth": school_year_start_month,
        "month": month_key,
        "daysInMonth": days,
        "typeOfDayCodes": type_of_day_codes,
        "students": students_json,
        "rows": rows_json
    }))
}

fn attendance_set_type_of_day(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, ErrObj> {
    let class_id = get_required_str(params, "classId")?;
    let month_key = get_required_str(params, "month")?;
    let day = params
        .get("day")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| ErrObj {
            code: "bad_params".into(),
            message: "missing day".into(),
            details: None,
        })? as usize;
    let code = parse_optional_code_char(params.get("code"))?;
    let (year, month_num) = parse_month_key(&month_key)?;
    let days = days_in_month(year, month_num);
    if day == 0 || day > days {
        return Err(ErrObj {
            code: "bad_params".into(),
            message: "day out of range for month".into(),
            details: None,
        });
    }
    let existing: Option<String> = conn
        .query_row(
            "SELECT type_of_day_codes FROM attendance_months WHERE class_id = ? AND month = ?",
            (&class_id, &month_key),
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    let patched = patch_day_code(existing.as_deref().unwrap_or(""), days, day, code);
    conn.execute(
        "INSERT INTO attendance_months(class_id, month, type_of_day_codes)
         VALUES(?, ?, ?)
         ON CONFLICT(class_id, month) DO UPDATE SET
           type_of_day_codes = excluded.type_of_day_codes",
        (&class_id, &month_key, &patched),
    )
    .map_err(|e| ErrObj {
        code: "db_update_failed".into(),
        message: e.to_string(),
        details: Some(json!({ "table": "attendance_months" })),
    })?;
    Ok(json!({ "ok": true }))
}

fn attendance_set_student_day(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, ErrObj> {
    let class_id = get_required_str(params, "classId")?;
    let month_key = get_required_str(params, "month")?;
    let student_id = get_required_str(params, "studentId")?;
    let day = params
        .get("day")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| ErrObj {
            code: "bad_params".into(),
            message: "missing day".into(),
            details: None,
        })? as usize;
    let code = parse_optional_code_char(params.get("code"))?;
    let (year, month_num) = parse_month_key(&month_key)?;
    let days = days_in_month(year, month_num);
    if day == 0 || day > days {
        return Err(ErrObj {
            code: "bad_params".into(),
            message: "day out of range for month".into(),
            details: None,
        });
    }
    let student_exists = conn
        .query_row(
            "SELECT 1 FROM students WHERE class_id = ? AND id = ?",
            (&class_id, &student_id),
            |r| r.get::<_, i64>(0),
        )
        .optional()
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?
        .is_some();
    if !student_exists {
        return Err(ErrObj {
            code: "not_found".into(),
            message: "student not found".into(),
            details: None,
        });
    }
    let existing: Option<String> = conn
        .query_row(
            "SELECT day_codes FROM attendance_student_months WHERE class_id = ? AND student_id = ? AND month = ?",
            (&class_id, &student_id, &month_key),
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    let patched = patch_day_code(existing.as_deref().unwrap_or(""), days, day, code);
    conn.execute(
        "INSERT INTO attendance_student_months(class_id, student_id, month, day_codes)
         VALUES(?, ?, ?, ?)
         ON CONFLICT(class_id, student_id, month) DO UPDATE SET
           day_codes = excluded.day_codes",
        (&class_id, &student_id, &month_key, &patched),
    )
    .map_err(|e| ErrObj {
        code: "db_update_failed".into(),
        message: e.to_string(),
        details: Some(json!({ "table": "attendance_student_months" })),
    })?;
    Ok(json!({ "ok": true }))
}

fn attendance_bulk_stamp_day(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, ErrObj> {
    let class_id = get_required_str(params, "classId")?;
    let month_key = get_required_str(params, "month")?;
    let day = params
        .get("day")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| ErrObj {
            code: "bad_params".into(),
            message: "missing day".into(),
            details: None,
        })? as usize;
    let code = parse_optional_code_char(params.get("code"))?;
    let Some(student_ids_json) = params.get("studentIds").and_then(|v| v.as_array()) else {
        return Err(ErrObj {
            code: "bad_params".into(),
            message: "missing studentIds".into(),
            details: None,
        });
    };
    let student_ids: Vec<String> = student_ids_json
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();
    let (year, month_num) = parse_month_key(&month_key)?;
    let days = days_in_month(year, month_num);
    if day == 0 || day > days {
        return Err(ErrObj {
            code: "bad_params".into(),
            message: "day out of range for month".into(),
            details: None,
        });
    }

    let tx = conn.unchecked_transaction().map_err(|e| ErrObj {
        code: "db_tx_failed".into(),
        message: e.to_string(),
        details: None,
    })?;
    for student_id in student_ids {
        let exists = tx
            .query_row(
                "SELECT 1 FROM students WHERE class_id = ? AND id = ?",
                (&class_id, &student_id),
                |r| r.get::<_, i64>(0),
            )
            .optional()
            .map_err(|e| ErrObj {
                code: "db_query_failed".into(),
                message: e.to_string(),
                details: None,
            })?
            .is_some();
        if !exists {
            continue;
        }
        let existing: Option<String> = tx
            .query_row(
                "SELECT day_codes FROM attendance_student_months WHERE class_id = ? AND student_id = ? AND month = ?",
                (&class_id, &student_id, &month_key),
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| ErrObj {
                code: "db_query_failed".into(),
                message: e.to_string(),
                details: None,
            })?;
        let patched = patch_day_code(existing.as_deref().unwrap_or(""), days, day, code);
        tx.execute(
            "INSERT INTO attendance_student_months(class_id, student_id, month, day_codes)
             VALUES(?, ?, ?, ?)
             ON CONFLICT(class_id, student_id, month) DO UPDATE SET
               day_codes = excluded.day_codes",
            (&class_id, &student_id, &month_key, &patched),
        )
        .map_err(|e| ErrObj {
            code: "db_update_failed".into(),
            message: e.to_string(),
            details: Some(json!({ "table": "attendance_student_months" })),
        })?;
    }
    tx.commit().map_err(|e| ErrObj {
        code: "db_commit_failed".into(),
        message: e.to_string(),
        details: None,
    })?;
    Ok(json!({ "ok": true }))
}

fn seating_get(conn: &Connection, params: &serde_json::Value) -> Result<serde_json::Value, ErrObj> {
    let class_id = get_required_str(params, "classId")?;
    if !class_exists(conn, &class_id)? {
        return Err(ErrObj {
            code: "not_found".into(),
            message: "class not found".into(),
            details: None,
        });
    }
    let default_rows = 6_i64;
    let default_seats = 5_i64;
    let plan_row: Option<(i64, i64, String)> = conn
        .query_row(
            "SELECT rows, seats_per_row, blocked_mask FROM seating_plans WHERE class_id = ?",
            [&class_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    let (rows, seats_per_row, blocked_mask) =
        plan_row.unwrap_or((default_rows, default_seats, "0".repeat(100)));
    let seat_count = ((rows.max(1) * seats_per_row.max(1)) as usize).max(1);
    let blocked = normalize_day_codes(&blocked_mask, 100);
    let blocked_codes: Vec<usize> = blocked
        .chars()
        .enumerate()
        .filter_map(|(i, ch)| if ch == '1' { Some(i + 1) } else { None })
        .collect();

    let students = list_students_for_class(conn, &class_id)?;
    let sort_by_student: HashMap<String, i64> = students
        .iter()
        .map(|s| (s.id.clone(), s.sort_order))
        .collect();
    let mut assignments: Vec<Option<i64>> = vec![None; seat_count];
    let mut stmt = conn
        .prepare(
            "SELECT student_id, seat_code
             FROM seating_assignments
             WHERE class_id = ?",
        )
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    let rows_iter = stmt
        .query_map([&class_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    for (student_id, seat_code) in rows_iter {
        let Some(idx) = seat_code_to_index(seat_code, rows, seats_per_row) else {
            continue;
        };
        if idx >= assignments.len() {
            continue;
        }
        let Some(sort_order) = sort_by_student.get(&student_id).copied() else {
            continue;
        };
        assignments[idx] = Some(sort_order);
    }

    Ok(json!({
        "rows": rows,
        "seatsPerRow": seats_per_row,
        "blockedSeatCodes": blocked_codes,
        "assignments": assignments
    }))
}

fn seating_save(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, ErrObj> {
    let class_id = get_required_str(params, "classId")?;
    let rows = params
        .get("rows")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| ErrObj {
            code: "bad_params".into(),
            message: "missing rows".into(),
            details: None,
        })?
        .max(1);
    let seats_per_row = params
        .get("seatsPerRow")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| ErrObj {
            code: "bad_params".into(),
            message: "missing seatsPerRow".into(),
            details: None,
        })?
        .max(1);
    let seat_count = (rows * seats_per_row) as usize;
    let assignments_json = params
        .get("assignments")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ErrObj {
            code: "bad_params".into(),
            message: "missing assignments".into(),
            details: None,
        })?;

    let blocked_codes: Vec<usize> = match params.get("blockedSeatCodes") {
        Some(v) if v.is_string() => {
            let s = v.as_str().unwrap_or_default();
            s.chars()
                .enumerate()
                .filter_map(|(i, ch)| if ch == '1' { Some(i + 1) } else { None })
                .collect()
        }
        Some(v) => v
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_u64().map(|n| n as usize))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        None => Vec::new(),
    };
    let mut blocked_mask_chars = vec!['0'; 100];
    for code in blocked_codes {
        if (1..=100).contains(&code) {
            blocked_mask_chars[code - 1] = '1';
        }
    }
    let blocked_mask: String = blocked_mask_chars.into_iter().collect();

    let students = list_students_for_class(conn, &class_id)?;
    let by_sort_order: HashMap<i64, String> = students
        .iter()
        .map(|s| (s.sort_order, s.id.clone()))
        .collect();

    let tx = conn.unchecked_transaction().map_err(|e| ErrObj {
        code: "db_tx_failed".into(),
        message: e.to_string(),
        details: None,
    })?;

    tx.execute(
        "INSERT INTO seating_plans(class_id, rows, seats_per_row, blocked_mask)
         VALUES(?, ?, ?, ?)
         ON CONFLICT(class_id) DO UPDATE SET
           rows = excluded.rows,
           seats_per_row = excluded.seats_per_row,
           blocked_mask = excluded.blocked_mask",
        (&class_id, rows, seats_per_row, &blocked_mask),
    )
    .map_err(|e| ErrObj {
        code: "db_update_failed".into(),
        message: e.to_string(),
        details: Some(json!({ "table": "seating_plans" })),
    })?;
    tx.execute(
        "DELETE FROM seating_assignments WHERE class_id = ?",
        [&class_id],
    )
    .map_err(|e| ErrObj {
        code: "db_delete_failed".into(),
        message: e.to_string(),
        details: Some(json!({ "table": "seating_assignments" })),
    })?;

    let mut seen_students: HashSet<String> = HashSet::new();
    for (idx, v) in assignments_json.iter().enumerate() {
        if idx >= seat_count {
            break;
        }
        let Some(sort_order) = v.as_i64() else {
            continue;
        };
        let Some(student_id) = by_sort_order.get(&sort_order).cloned() else {
            continue;
        };
        if seen_students.contains(&student_id) {
            continue;
        }
        seen_students.insert(student_id.clone());
        tx.execute(
            "INSERT INTO seating_assignments(class_id, student_id, seat_code) VALUES(?, ?, ?)",
            (
                &class_id,
                &student_id,
                seat_index_to_code(idx, seats_per_row),
            ),
        )
        .map_err(|e| ErrObj {
            code: "db_insert_failed".into(),
            message: e.to_string(),
            details: Some(json!({ "table": "seating_assignments" })),
        })?;
    }
    tx.commit().map_err(|e| ErrObj {
        code: "db_commit_failed".into(),
        message: e.to_string(),
        details: None,
    })?;
    Ok(json!({ "ok": true }))
}

fn seat_index_to_code(index: usize, seats_per_row: i64) -> i64 {
    let row = (index as i64) / seats_per_row.max(1);
    let col = (index as i64) % seats_per_row.max(1) + 1;
    row * 10 + col
}

fn seat_code_to_index(seat_code: i64, rows: i64, seats_per_row: i64) -> Option<usize> {
    if seat_code <= 0 {
        return None;
    }
    let row = seat_code / 10;
    let col = seat_code % 10;
    if row < 0 || row >= rows || col < 1 || col > seats_per_row {
        return None;
    }
    Some((row * seats_per_row + (col - 1)) as usize)
}

fn loaned_list(conn: &Connection, params: &serde_json::Value) -> Result<serde_json::Value, ErrObj> {
    let class_id = get_required_str(params, "classId")?;
    if !class_exists(conn, &class_id)? {
        return Err(ErrObj {
            code: "not_found".into(),
            message: "class not found".into(),
            details: None,
        });
    }
    let mark_set_id = params
        .get("markSetId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let (sql, binds): (&str, Vec<&dyn rusqlite::ToSql>) = if let Some(ref msid) = mark_set_id {
        (
            "SELECT li.id, li.student_id, s.last_name, s.first_name, li.mark_set_id, li.item_name, li.quantity, li.notes, li.raw_line
             FROM loaned_items li
             JOIN students s ON s.id = li.student_id
             WHERE li.class_id = ? AND li.mark_set_id = ?
             ORDER BY s.sort_order, li.item_name",
            vec![&class_id as &dyn rusqlite::ToSql, msid as &dyn rusqlite::ToSql],
        )
    } else {
        (
            "SELECT li.id, li.student_id, s.last_name, s.first_name, li.mark_set_id, li.item_name, li.quantity, li.notes, li.raw_line
             FROM loaned_items li
             JOIN students s ON s.id = li.student_id
             WHERE li.class_id = ?
             ORDER BY s.sort_order, li.item_name",
            vec![&class_id as &dyn rusqlite::ToSql],
        )
    };

    let mut stmt = conn.prepare(sql).map_err(|e| ErrObj {
        code: "db_query_failed".into(),
        message: e.to_string(),
        details: None,
    })?;
    let rows = stmt
        .query_map(rusqlite::params_from_iter(binds), |r| {
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;

    Ok(json!({ "items": rows }))
}

fn loaned_get(conn: &Connection, params: &serde_json::Value) -> Result<serde_json::Value, ErrObj> {
    let class_id = get_required_str(params, "classId")?;
    let item_id = get_required_str(params, "itemId")?;
    let mut stmt = conn
        .prepare(
            "SELECT li.id, li.student_id, s.last_name, s.first_name, li.mark_set_id, li.item_name, li.quantity, li.notes, li.raw_line
             FROM loaned_items li
             JOIN students s ON s.id = li.student_id
             WHERE li.class_id = ? AND li.id = ?",
        )
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    let Some(item) = row else {
        return Err(ErrObj {
            code: "not_found".into(),
            message: "loaned item not found".into(),
            details: None,
        });
    };
    Ok(json!({ "item": item }))
}

fn loaned_update(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, ErrObj> {
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?
        .is_some();
    if !student_exists {
        return Err(ErrObj {
            code: "not_found".into(),
            message: "student not found".into(),
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
    .map_err(|e| ErrObj {
        code: "db_update_failed".into(),
        message: e.to_string(),
        details: Some(json!({ "table": "loaned_items" })),
    })?;
    Ok(json!({ "ok": true, "itemId": item_id }))
}

fn devices_list(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, ErrObj> {
    let class_id = get_required_str(params, "classId")?;
    if !class_exists(conn, &class_id)? {
        return Err(ErrObj {
            code: "not_found".into(),
            message: "class not found".into(),
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;

    Ok(json!({ "devices": rows }))
}

fn devices_get(conn: &Connection, params: &serde_json::Value) -> Result<serde_json::Value, ErrObj> {
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    let Some(device) = row else {
        return Err(ErrObj {
            code: "not_found".into(),
            message: "student not found".into(),
            details: None,
        });
    };
    Ok(json!({ "device": device }))
}

fn devices_update(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, ErrObj> {
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?
        .is_some();
    if !student_exists {
        return Err(ErrObj {
            code: "not_found".into(),
            message: "student not found".into(),
            details: None,
        });
    }

    if device_code.is_empty() && raw_line.is_empty() {
        conn.execute(
            "DELETE FROM student_device_map WHERE class_id = ? AND student_id = ?",
            (&class_id, &student_id),
        )
        .map_err(|e| ErrObj {
            code: "db_delete_failed".into(),
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
    .map_err(|e| ErrObj {
        code: "db_update_failed".into(),
        message: e.to_string(),
        details: Some(json!({ "table": "student_device_map" })),
    })?;

    Ok(json!({ "ok": true }))
}

fn learning_skills_open(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, ErrObj> {
    let class_id = get_required_str(params, "classId")?;
    if !class_exists(conn, &class_id)? {
        return Err(ErrObj {
            code: "not_found".into(),
            message: "class not found".into(),
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
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
) -> Result<serde_json::Value, ErrObj> {
    let class_id = get_required_str(params, "classId")?;
    let student_id = get_required_str(params, "studentId")?;
    let skill_code = get_required_str(params, "skillCode")?.to_ascii_uppercase();
    if skill_code.is_empty() || skill_code.len() > 8 {
        return Err(ErrObj {
            code: "bad_params".into(),
            message: "skillCode must be 1..8 chars".into(),
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?
        .is_some();
    if !student_exists {
        return Err(ErrObj {
            code: "not_found".into(),
            message: "student not found".into(),
            details: None,
        });
    }

    if value.is_empty() {
        conn.execute(
            "DELETE FROM learning_skills_cells
             WHERE class_id = ? AND student_id = ? AND term = ? AND skill_code = ?",
            (&class_id, &student_id, term, &skill_code),
        )
        .map_err(|e| ErrObj {
            code: "db_delete_failed".into(),
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
    .map_err(|e| ErrObj {
        code: "db_update_failed".into(),
        message: e.to_string(),
        details: Some(json!({ "table": "learning_skills_cells" })),
    })?;
    Ok(json!({ "ok": true }))
}

fn learning_skills_report_model(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, ErrObj> {
    let mut open = learning_skills_open(conn, params)?;
    let class_id = get_required_str(params, "classId")?;
    let class_name: Option<String> = conn
        .query_row("SELECT name FROM classes WHERE id = ?", [&class_id], |r| {
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
    let obj = open.as_object_mut().ok_or_else(|| ErrObj {
        code: "server_error".into(),
        message: "invalid learning skills model".into(),
        details: None,
    })?;
    obj.insert(
        "class".to_string(),
        json!({ "id": class_id, "name": class_name }),
    );
    Ok(open)
}

fn comments_sets_list(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, ErrObj> {
    let class_id = get_required_str(params, "classId")?;
    let mark_set_id = get_required_str(params, "markSetId")?;
    if !mark_set_exists(conn, &class_id, &mark_set_id)? {
        return Err(ErrObj {
            code: "not_found".into(),
            message: "mark set not found".into(),
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    Ok(json!({ "sets": sets }))
}

fn comments_sets_open(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, ErrObj> {
    let class_id = get_required_str(params, "classId")?;
    let mark_set_id = get_required_str(params, "markSetId")?;
    let set_number = params
        .get("setNumber")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| ErrObj {
            code: "bad_params".into(),
            message: "missing setNumber".into(),
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
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
        return Err(ErrObj {
            code: "not_found".into(),
            message: "comment set not found".into(),
            details: None,
        });
    };

    let students = list_students_for_class(conn, &class_id)?;
    let mut remark_by_student: HashMap<String, String> = HashMap::new();
    let mut stmt = conn
        .prepare(
            "SELECT student_id, remark FROM comment_set_remarks WHERE comment_set_index_id = ?",
        )
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    let rows = stmt
        .query_map([&set_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
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

fn parse_remarks_by_student(
    raw: Option<&serde_json::Value>,
) -> Result<Vec<(String, String)>, ErrObj> {
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
    Err(ErrObj {
        code: "bad_params".into(),
        message: "remarksByStudent must be array or object".into(),
        details: None,
    })
}

fn comments_sets_upsert(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, ErrObj> {
    let class_id = get_required_str(params, "classId")?;
    let mark_set_id = get_required_str(params, "markSetId")?;
    if !mark_set_exists(conn, &class_id, &mark_set_id)? {
        return Err(ErrObj {
            code: "not_found".into(),
            message: "mark set not found".into(),
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
    let fit_font_size = params
        .get("fitFontSize")
        .and_then(|v| v.as_i64())
        .unwrap_or(9);
    let fit_width = params
        .get("fitWidth")
        .and_then(|v| v.as_i64())
        .unwrap_or(83);
    let fit_lines = params
        .get("fitLines")
        .and_then(|v| v.as_i64())
        .unwrap_or(12);
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
    let is_default = params
        .get("isDefault")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let bank_short = params
        .get("bankShort")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .and_then(|s| if s.is_empty() { None } else { Some(s) });
    let requested_set_number = params.get("setNumber").and_then(|v| v.as_i64());
    let remarks_by_student = parse_remarks_by_student(params.get("remarksByStudent"))?;

    let tx = conn.unchecked_transaction().map_err(|e| ErrObj {
        code: "db_tx_failed".into(),
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?
    };

    if is_default {
        tx.execute(
            "UPDATE comment_set_indexes SET is_default = 0 WHERE mark_set_id = ?",
            [&mark_set_id],
        )
        .map_err(|e| ErrObj {
            code: "db_update_failed".into(),
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
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
    .map_err(|e| ErrObj {
        code: "db_insert_failed".into(),
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
            .map_err(|e| ErrObj {
                code: "db_delete_failed".into(),
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
        .map_err(|e| ErrObj {
            code: "db_insert_failed".into(),
            message: e.to_string(),
            details: Some(json!({ "table": "comment_set_remarks" })),
        })?;
    }

    tx.commit().map_err(|e| ErrObj {
        code: "db_commit_failed".into(),
        message: e.to_string(),
        details: None,
    })?;
    Ok(json!({ "setNumber": set_number }))
}

fn comments_sets_delete(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, ErrObj> {
    let class_id = get_required_str(params, "classId")?;
    let mark_set_id = get_required_str(params, "markSetId")?;
    let set_number = params
        .get("setNumber")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| ErrObj {
            code: "bad_params".into(),
            message: "missing setNumber".into(),
            details: None,
        })?;
    let tx = conn.unchecked_transaction().map_err(|e| ErrObj {
        code: "db_tx_failed".into(),
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    let Some(set_id) = set_id else {
        return Err(ErrObj {
            code: "not_found".into(),
            message: "comment set not found".into(),
            details: None,
        });
    };
    tx.execute(
        "DELETE FROM comment_set_remarks WHERE comment_set_index_id = ?",
        [&set_id],
    )
    .map_err(|e| ErrObj {
        code: "db_delete_failed".into(),
        message: e.to_string(),
        details: Some(json!({ "table": "comment_set_remarks" })),
    })?;
    tx.execute("DELETE FROM comment_set_indexes WHERE id = ?", [&set_id])
        .map_err(|e| ErrObj {
            code: "db_delete_failed".into(),
            message: e.to_string(),
            details: Some(json!({ "table": "comment_set_indexes" })),
        })?;
    tx.commit().map_err(|e| ErrObj {
        code: "db_commit_failed".into(),
        message: e.to_string(),
        details: None,
    })?;
    Ok(json!({ "ok": true }))
}

fn comments_banks_list(conn: &Connection) -> Result<serde_json::Value, ErrObj> {
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    Ok(json!({ "banks": banks }))
}

fn comments_banks_open(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, ErrObj> {
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    let Some(bank) = bank else {
        return Err(ErrObj {
            code: "not_found".into(),
            message: "bank not found".into(),
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    Ok(json!({ "bank": bank, "entries": entries }))
}

fn comments_banks_create(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, ErrObj> {
    let short_name = get_required_str(params, "shortName")?.trim().to_string();
    if short_name.is_empty() {
        return Err(ErrObj {
            code: "bad_params".into(),
            message: "shortName must not be empty".into(),
            details: None,
        });
    }
    let bank_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO comment_banks(id, short_name, is_default, fit_profile, source_path)
         VALUES(?, ?, 0, NULL, NULL)",
        (&bank_id, &short_name),
    )
    .map_err(|e| ErrObj {
        code: "db_insert_failed".into(),
        message: e.to_string(),
        details: Some(json!({ "table": "comment_banks" })),
    })?;
    Ok(json!({ "bankId": bank_id }))
}

fn comments_banks_update_meta(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, ErrObj> {
    let bank_id = get_required_str(params, "bankId")?;
    let Some(patch) = params.get("patch").and_then(|v| v.as_object()) else {
        return Err(ErrObj {
            code: "bad_params".into(),
            message: "missing patch".into(),
            details: None,
        });
    };
    let tx = conn.unchecked_transaction().map_err(|e| ErrObj {
        code: "db_tx_failed".into(),
        message: e.to_string(),
        details: None,
    })?;
    if patch
        .get("isDefault")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        tx.execute("UPDATE comment_banks SET is_default = 0", [])
            .map_err(|e| ErrObj {
                code: "db_update_failed".into(),
                message: e.to_string(),
                details: Some(json!({ "table": "comment_banks" })),
            })?;
    }

    let mut set_parts: Vec<String> = Vec::new();
    let mut binds: Vec<Value> = Vec::new();
    if let Some(v) = patch.get("shortName") {
        let Some(s) = v.as_str() else {
            return Err(ErrObj {
                code: "bad_params".into(),
                message: "patch.shortName must be string".into(),
                details: None,
            });
        };
        set_parts.push("short_name = ?".into());
        binds.push(Value::Text(s.trim().to_string()));
    }
    if let Some(v) = patch.get("isDefault") {
        let Some(b) = v.as_bool() else {
            return Err(ErrObj {
                code: "bad_params".into(),
                message: "patch.isDefault must be boolean".into(),
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
            return Err(ErrObj {
                code: "bad_params".into(),
                message: "patch.fitProfile must be string|null".into(),
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
            return Err(ErrObj {
                code: "bad_params".into(),
                message: "patch.sourcePath must be string|null".into(),
                details: None,
            });
        }
    }

    if !set_parts.is_empty() {
        let sql = format!(
            "UPDATE comment_banks SET {} WHERE id = ?",
            set_parts.join(", ")
        );
        binds.push(Value::Text(bank_id.clone()));
        tx.execute(&sql, params_from_iter(binds))
            .map_err(|e| ErrObj {
                code: "db_update_failed".into(),
                message: e.to_string(),
                details: Some(json!({ "table": "comment_banks" })),
            })?;
    }
    tx.commit().map_err(|e| ErrObj {
        code: "db_commit_failed".into(),
        message: e.to_string(),
        details: None,
    })?;
    Ok(json!({ "ok": true }))
}

fn comments_banks_entry_upsert(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, ErrObj> {
    let bank_id = get_required_str(params, "bankId")?;
    let type_code = get_required_str(params, "typeCode")?;
    let level_code = get_required_str(params, "levelCode")?;
    let text = get_required_str(params, "text")?;
    let requested_sort = params.get("sortOrder").and_then(|v| v.as_i64());
    let entry_id = params
        .get("entryId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let tx = conn.unchecked_transaction().map_err(|e| ErrObj {
        code: "db_tx_failed".into(),
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
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
                .map_err(|e| ErrObj {
                    code: "db_update_failed".into(),
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
                .map_err(|e| ErrObj {
                    code: "db_update_failed".into(),
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
        .map_err(|e| ErrObj {
            code: "db_update_failed".into(),
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
    .map_err(|e| ErrObj {
        code: "db_insert_failed".into(),
        message: e.to_string(),
        details: Some(json!({ "table": "comment_bank_entries" })),
    })?;

    tx.commit().map_err(|e| ErrObj {
        code: "db_commit_failed".into(),
        message: e.to_string(),
        details: None,
    })?;
    Ok(json!({ "entryId": resolved_entry_id }))
}

fn comments_banks_entry_delete(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, ErrObj> {
    let bank_id = get_required_str(params, "bankId")?;
    let entry_id = get_required_str(params, "entryId")?;
    let tx = conn.unchecked_transaction().map_err(|e| ErrObj {
        code: "db_tx_failed".into(),
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    let Some(sort_order) = sort_order else {
        return Err(ErrObj {
            code: "not_found".into(),
            message: "entry not found".into(),
            details: None,
        });
    };
    tx.execute(
        "DELETE FROM comment_bank_entries WHERE id = ? AND bank_id = ?",
        (&entry_id, &bank_id),
    )
    .map_err(|e| ErrObj {
        code: "db_delete_failed".into(),
        message: e.to_string(),
        details: Some(json!({ "table": "comment_bank_entries" })),
    })?;
    tx.execute(
        "UPDATE comment_bank_entries
         SET sort_order = sort_order - 1
         WHERE bank_id = ? AND sort_order > ?",
        (&bank_id, sort_order),
    )
    .map_err(|e| ErrObj {
        code: "db_update_failed".into(),
        message: e.to_string(),
        details: Some(json!({ "table": "comment_bank_entries" })),
    })?;
    tx.commit().map_err(|e| ErrObj {
        code: "db_commit_failed".into(),
        message: e.to_string(),
        details: None,
    })?;
    Ok(json!({ "ok": true }))
}

fn comments_banks_import_bnk(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, ErrObj> {
    let path = get_required_str(params, "path")?;
    let file_path = PathBuf::from(&path);
    let short_name = file_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| ErrObj {
            code: "bad_params".into(),
            message: "invalid path".into(),
            details: None,
        })?
        .to_string();
    let parsed = legacy::parse_bnk_file(&file_path).map_err(|e| ErrObj {
        code: "legacy_parse_failed".into(),
        message: e.to_string(),
        details: Some(json!({ "path": path })),
    })?;

    let tx = conn.unchecked_transaction().map_err(|e| ErrObj {
        code: "db_tx_failed".into(),
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
    .map_err(|e| ErrObj {
        code: "db_insert_failed".into(),
        message: e.to_string(),
        details: Some(json!({ "table": "comment_banks" })),
    })?;
    let bank_id: String = tx
        .query_row(
            "SELECT id FROM comment_banks WHERE short_name = ?",
            [&short_name],
            |r| r.get(0),
        )
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    tx.execute(
        "DELETE FROM comment_bank_entries WHERE bank_id = ?",
        [&bank_id],
    )
    .map_err(|e| ErrObj {
        code: "db_delete_failed".into(),
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
        .map_err(|e| ErrObj {
            code: "db_insert_failed".into(),
            message: e.to_string(),
            details: Some(json!({ "table": "comment_bank_entries" })),
        })?;
    }
    tx.commit().map_err(|e| ErrObj {
        code: "db_commit_failed".into(),
        message: e.to_string(),
        details: None,
    })?;
    Ok(json!({ "bankId": bank_id }))
}

fn comments_banks_export_bnk(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, ErrObj> {
    let bank_id = get_required_str(params, "bankId")?;
    let out_path = get_required_str(params, "path")?;
    let bank_meta: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT short_name, fit_profile FROM comment_banks WHERE id = ?",
            [&bank_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    let Some((_short_name, fit_profile)) = bank_meta else {
        return Err(ErrObj {
            code: "not_found".into(),
            message: "bank not found".into(),
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
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
        .map_err(|e| ErrObj {
            code: "db_query_failed".into(),
            message: e.to_string(),
            details: None,
        })?;
    let text = legacy::serialize_bnk_file(&legacy::ParsedBnkFile {
        fit_profile,
        entries,
    });
    let out = PathBuf::from(&out_path);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| ErrObj {
            code: "io_failed".into(),
            message: e.to_string(),
            details: Some(json!({ "path": out_path })),
        })?;
    }
    std::fs::write(&out, text).map_err(|e| ErrObj {
        code: "io_failed".into(),
        message: e.to_string(),
        details: Some(json!({ "path": out_path })),
    })?;
    Ok(json!({ "ok": true }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_path(rel: &str) -> PathBuf {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        base.join("../../").join(rel)
    }

    fn temp_workspace() -> PathBuf {
        std::env::temp_dir().join(format!(
            "markbook-ipc-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ))
    }

    fn request_ok(
        state: &mut AppState,
        method: &str,
        params: serde_json::Value,
    ) -> serde_json::Value {
        let res = handle_request(
            state,
            Request {
                id: "test-1".to_string(),
                method: method.to_string(),
                params,
            },
        );
        let ok = res.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        assert!(
            ok,
            "{} failed: {}",
            method,
            res.get("error")
                .and_then(|e| e.get("message"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error")
        );
        res.get("result").cloned().unwrap_or_else(|| json!({}))
    }

    #[test]
    fn final_marks_match_sample25_expected_named_students() {
        let workspace = temp_workspace();
        let mut state = AppState {
            workspace: None,
            db: None,
        };

        let fixture_folder = fixture_path("fixtures/legacy/Sample25/MB8D25");
        request_ok(
            &mut state,
            "workspace.select",
            json!({ "path": workspace.to_string_lossy() }),
        );
        let import_res = request_ok(
            &mut state,
            "class.importLegacy",
            json!({ "legacyClassFolderPath": fixture_folder.to_string_lossy() }),
        );
        let class_id = import_res
            .get("classId")
            .and_then(|v| v.as_str())
            .expect("classId")
            .to_string();

        let marksets = request_ok(&mut state, "marksets.list", json!({ "classId": class_id }));
        let mut markset_ids_by_code: HashMap<String, String> = HashMap::new();
        for ms in marksets
            .get("markSets")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
        {
            if let (Some(code), Some(id)) = (
                ms.get("code").and_then(|v| v.as_str()),
                ms.get("id").and_then(|v| v.as_str()),
            ) {
                markset_ids_by_code.insert(code.to_string(), id.to_string());
            }
        }

        let expected_text = std::fs::read_to_string(fixture_path(
            "fixtures/legacy/Sample25/expected/final-marks.json",
        ))
        .expect("read expected final marks");
        let expected: serde_json::Value =
            serde_json::from_str(&expected_text).expect("parse expected final marks");

        for mark_set_code in ["MAT1", "SNC1"] {
            let mark_set_id = markset_ids_by_code
                .get(mark_set_code)
                .expect("mark set id")
                .to_string();
            let summary = request_ok(
                &mut state,
                "calc.markSetSummary",
                json!({ "classId": class_id, "markSetId": mark_set_id }),
            );
            let mut actual_map: HashMap<String, Option<f64>> = HashMap::new();
            for student in summary
                .get("perStudent")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default()
            {
                let Some(name) = student.get("displayName").and_then(|v| v.as_str()) else {
                    continue;
                };
                let mark = student.get("finalMark").and_then(|v| v.as_f64());
                actual_map.insert(name.to_string(), mark);
            }

            let expected_set = expected
                .get(mark_set_code)
                .and_then(|v| v.as_object())
                .expect("expected set object");
            for (name, expected_mark) in expected_set {
                let expected_mark = expected_mark.as_f64().expect("expected mark as number");
                let actual_mark = actual_map
                    .get(name)
                    .copied()
                    .flatten()
                    .unwrap_or_else(|| panic!("missing actual final mark for {}", name));
                let diff = (actual_mark - expected_mark).abs();
                assert!(
                    diff <= 0.05,
                    "final mark mismatch {} {}: expected {}, got {}",
                    mark_set_code,
                    name,
                    expected_mark,
                    actual_mark
                );
            }
        }

        let _ = std::fs::remove_dir_all(workspace);
    }

    #[test]
    fn import_includes_all_idx_tbk_and_icc_companions() {
        let workspace = temp_workspace();
        let mut state = AppState {
            workspace: None,
            db: None,
        };
        let fixture_folder = fixture_path("fixtures/legacy/Sample25/MB8D25");
        request_ok(
            &mut state,
            "workspace.select",
            json!({ "path": workspace.to_string_lossy() }),
        );
        let import_res = request_ok(
            &mut state,
            "class.importLegacy",
            json!({ "legacyClassFolderPath": fixture_folder.to_string_lossy() }),
        );
        let class_id = import_res
            .get("classId")
            .and_then(|v| v.as_str())
            .expect("classId")
            .to_string();

        let combined = import_res
            .get("combinedCommentSetsImported")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let loaned = import_res
            .get("loanedItemsImported")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let devices = import_res
            .get("deviceMappingsImported")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        assert!(
            combined > 0,
            "expected ALL! IDX combined sets to be imported"
        );
        assert!(loaned > 0, "expected TBK loaned items to be imported");
        assert!(devices > 0, "expected ICC device mappings to be imported");

        let marksets = request_ok(&mut state, "marksets.list", json!({ "classId": class_id }));
        let first_mark_set_id = marksets
            .get("markSets")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("id"))
            .and_then(|v| v.as_str())
            .expect("mark set id")
            .to_string();

        let sets = request_ok(
            &mut state,
            "comments.sets.list",
            json!({ "classId": class_id, "markSetId": first_mark_set_id }),
        );
        assert!(
            sets.get("sets")
                .and_then(|v| v.as_array())
                .map(|arr| arr.len())
                .unwrap_or(0)
                >= 2,
            "expected mark set-specific and ALL! combined comment sets"
        );

        let _ = std::fs::remove_dir_all(workspace);
    }
}
