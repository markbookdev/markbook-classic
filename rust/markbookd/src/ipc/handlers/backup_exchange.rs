use crate::backup;
use crate::db;
use crate::ipc::error::{err, ok};
use crate::ipc::types::{AppState, Request};
use rusqlite::{Connection, OptionalExtension};
use serde_json::json;
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

#[derive(Clone, Debug)]
struct ParsedExchangeRow {
    line_no: usize,
    student_id: String,
    mark_set_code: String,
    assessment_idx: i64,
    status: String,
    raw_value: Option<f64>,
}

fn parse_exchange_rows(text: &str) -> (Vec<ParsedExchangeRow>, Vec<serde_json::Value>, usize) {
    let mut rows = Vec::new();
    let mut warnings = Vec::new();
    let mut total = 0usize;
    for (line_no, raw_line) in text.lines().enumerate() {
        if line_no == 0 {
            continue;
        }
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        total += 1;
        let fields = parse_csv_record(line);
        if fields.len() < 7 {
            warnings.push(json!({
                "line": line_no + 1,
                "code": "bad_columns",
                "message": "expected at least 7 CSV columns"
            }));
            continue;
        }
        let student_id = fields[0].trim().to_string();
        let mark_set_code = fields[2].trim().to_string();
        let assessment_idx = match fields[3].trim().parse::<i64>() {
            Ok(v) => v,
            Err(_) => {
                warnings.push(json!({
                    "line": line_no + 1,
                    "code": "bad_assessment_idx",
                    "message": "assessment_idx must be an integer"
                }));
                continue;
            }
        };
        let status = fields[5].trim().to_ascii_lowercase();
        let raw_value = if fields[6].trim().is_empty() {
            None
        } else {
            match fields[6].trim().parse::<f64>() {
                Ok(v) => Some(v),
                Err(_) => {
                    warnings.push(json!({
                        "line": line_no + 1,
                        "code": "bad_raw_value",
                        "message": "raw_value must be numeric when provided"
                    }));
                    continue;
                }
            }
        };
        rows.push(ParsedExchangeRow {
            line_no: line_no + 1,
            student_id,
            mark_set_code,
            assessment_idx,
            status,
            raw_value,
        });
    }
    (rows, warnings, total)
}

fn resolve_score_state(
    explicit_state: Option<&str>,
    value: Option<f64>,
) -> Result<(Option<f64>, &'static str), HandlerErr> {
    if let Some(v) = value {
        if v < 0.0 {
            return Err(HandlerErr {
                code: "bad_params",
                message: "negative marks are not allowed".to_string(),
                details: Some(json!({ "value": v })),
            });
        }
    }

    match explicit_state.map(|s| s.to_ascii_lowercase()) {
        Some(s) if s == "no_mark" => Ok((Some(0.0), "no_mark")),
        Some(s) if s == "zero" => Ok((None, "zero")),
        Some(s) if s == "scored" => {
            let Some(v) = value else {
                return Err(HandlerErr {
                    code: "bad_params",
                    message: "scored state requires numeric value".to_string(),
                    details: None,
                });
            };
            if v <= 0.0 {
                return Err(HandlerErr {
                    code: "bad_params",
                    message: "scored marks must be > 0".to_string(),
                    details: Some(json!({ "value": v })),
                });
            }
            Ok((Some(v), "scored"))
        }
        Some(other) => Err(HandlerErr {
            code: "bad_params",
            message: "state must be one of: scored, zero, no_mark".to_string(),
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

fn upsert_score(
    conn: &Connection,
    assessment_id: &str,
    student_id: &str,
    raw_value: Option<f64>,
    status: &str,
) -> Result<(), HandlerErr> {
    let score_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO scores(id, assessment_id, student_id, raw_value, status)
         VALUES(?, ?, ?, ?, ?)
         ON CONFLICT(assessment_id, student_id) DO UPDATE SET
           raw_value = excluded.raw_value,
           status = excluded.status",
        (&score_id, assessment_id, student_id, raw_value, status),
    )
    .map_err(|e| HandlerErr {
        code: "db_insert_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "scores" })),
    })?;
    Ok(())
}

fn handle_backup_export_workspace_bundle(state: &mut AppState, req: &Request) -> serde_json::Value {
    let out_path = match req.params.get("outPath").and_then(|v| v.as_str()) {
        Some(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => return err(&req.id, "bad_params", "missing outPath", None),
    };
    let workspace_path = req
        .params
        .get("workspacePath")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .or_else(|| state.workspace.clone());
    let Some(workspace_path) = workspace_path else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    if let Some(conn) = state.db.as_ref() {
        let _ = conn.execute_batch("PRAGMA wal_checkpoint(FULL)");
    }

    let out = PathBuf::from(&out_path);
    let export = match backup::export_workspace_bundle(&workspace_path, &out) {
        Ok(v) => v,
        Err(e) => {
            return err(
                &req.id,
                "io_failed",
                e.to_string(),
                Some(json!({ "path": out_path })),
            )
        }
    };

    ok(
        &req.id,
        json!({
            "ok": true,
            "path": out_path,
            "bundleFormat": export.bundle_format,
            "entryCount": export.entry_count
        }),
    )
}

fn handle_backup_import_workspace_bundle(state: &mut AppState, req: &Request) -> serde_json::Value {
    let in_path = match req.params.get("inPath").and_then(|v| v.as_str()) {
        Some(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => return err(&req.id, "bad_params", "missing inPath", None),
    };
    let workspace_path = req
        .params
        .get("workspacePath")
        .and_then(|v| v.as_str())
        .map(PathBuf::from)
        .or_else(|| state.workspace.clone());
    let Some(workspace_path) = workspace_path else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let src = PathBuf::from(&in_path);
    if !src.is_file() {
        return err(
            &req.id,
            "not_found",
            "bundle file not found",
            Some(json!({ "path": in_path })),
        );
    }
    if let Err(e) = std::fs::create_dir_all(&workspace_path) {
        return err(
            &req.id,
            "io_failed",
            e.to_string(),
            Some(json!({ "path": workspace_path.to_string_lossy() })),
        );
    }

    // Drop open handle before replacing file.
    state.db = None;

    let import = match backup::import_workspace_bundle(&src, &workspace_path) {
        Ok(v) => v,
        Err(e) => {
            return err(
                &req.id,
                "io_failed",
                e.to_string(),
                Some(json!({ "path": src.to_string_lossy() })),
            )
        }
    };

    match db::open_db(&workspace_path) {
        Ok(conn) => {
            state.workspace = Some(workspace_path.clone());
            state.db = Some(conn);
            ok(
                &req.id,
                json!({
                    "ok": true,
                    "workspacePath": workspace_path.to_string_lossy(),
                    "bundleFormatDetected": import.bundle_format_detected
                }),
            )
        }
        Err(e) => err(&req.id, "db_open_failed", e.to_string(), None),
    }
}

fn handle_exchange_export_class_csv(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };
    let out_path = match req.params.get("outPath").and_then(|v| v.as_str()) {
        Some(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => return err(&req.id, "bad_params", "missing outPath", None),
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
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
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
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    let mut csv = String::from(
        "student_id,student_name,mark_set_code,assessment_idx,assessment_title,status,raw_value\n",
    );
    let rows_exported = rows.len();
    for (student_id, last, first, mark_set_code, assessment_idx, title, status, raw_value) in rows {
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
            return err(
                &req.id,
                "io_failed",
                e.to_string(),
                Some(json!({ "path": out_path })),
            );
        }
    }
    if let Err(e) = std::fs::write(&out, csv) {
        return err(
            &req.id,
            "io_failed",
            e.to_string(),
            Some(json!({ "path": out_path })),
        );
    }

    ok(
        &req.id,
        json!({ "ok": true, "rowsExported": rows_exported, "path": out_path }),
    )
}

fn read_exchange_input(req: &Request) -> Result<(String, String, String, String), serde_json::Value> {
    let class_id = req
        .params
        .get("classId")
        .and_then(|v| v.as_str())
        .map(|v| v.to_string())
        .ok_or_else(|| err(&req.id, "bad_params", "missing classId", None))?;
    let in_path = req
        .params
        .get("inPath")
        .and_then(|v| v.as_str())
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| err(&req.id, "bad_params", "missing inPath", None))?;
    let mode = req
        .params
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("upsert")
        .to_ascii_lowercase();
    let text = match std::fs::read_to_string(&in_path) {
        Ok(t) => t,
        Err(e) => {
            return Err(err(
                &req.id,
                "io_failed",
                e.to_string(),
                Some(json!({ "path": in_path })),
            ))
        }
    };
    Ok((class_id, in_path, mode, text))
}

fn handle_exchange_preview_class_csv(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    let (class_id, in_path, mode, text) = match read_exchange_input(req) {
        Ok(v) => v,
        Err(e) => return e,
    };

    let (parsed_rows, mut warnings, rows_total) = parse_exchange_rows(&text);
    let mut matched = 0usize;
    let mut unmatched = 0usize;
    let mut preview_rows = Vec::new();
    for row in &parsed_rows {
        let student_ok = conn
            .query_row(
                "SELECT 1 FROM students WHERE id = ? AND class_id = ?",
                (&row.student_id, &class_id),
                |r| r.get::<_, i64>(0),
            )
            .optional()
            .ok()
            .flatten()
            .is_some();
        let assessment_id: Option<String> = conn
            .query_row(
                "SELECT a.id
                 FROM assessments a
                 JOIN mark_sets ms ON ms.id = a.mark_set_id
                 WHERE ms.class_id = ? AND ms.code = ? AND a.idx = ?",
                (&class_id, &row.mark_set_code, row.assessment_idx),
                |r| r.get(0),
            )
            .optional()
            .ok()
            .flatten();

        let mut status = "matched";
        if !student_ok {
            status = "missing_student";
            warnings.push(json!({
                "line": row.line_no,
                "code": "missing_student",
                "message": "student_id does not belong to target class"
            }));
        } else if assessment_id.is_none() {
            status = "missing_assessment";
            warnings.push(json!({
                "line": row.line_no,
                "code": "missing_assessment",
                "message": "assessment not found in target class/mark set"
            }));
        } else if let Err(e) = resolve_score_state(Some(&row.status), row.raw_value) {
            status = "invalid_state";
            warnings.push(json!({
                "line": row.line_no,
                "code": e.code,
                "message": e.message
            }));
        }

        if status == "matched" {
            matched += 1;
        } else {
            unmatched += 1;
        }
        if preview_rows.len() < 250 {
            preview_rows.push(json!({
                "line": row.line_no,
                "studentId": row.student_id,
                "markSetCode": row.mark_set_code,
                "assessmentIdx": row.assessment_idx,
                "status": status
            }));
        }
    }
    ok(
        &req.id,
        json!({
            "ok": true,
            "path": in_path,
            "mode": mode,
            "rowsTotal": rows_total,
            "rowsParsed": parsed_rows.len(),
            "rowsMatched": matched,
            "rowsUnmatched": unmatched,
            "warningsCount": warnings.len(),
            "warnings": warnings,
            "previewRows": preview_rows
        }),
    )
}

fn handle_exchange_apply_class_csv(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    let (class_id, in_path, mode, text) = match read_exchange_input(req) {
        Ok(v) => v,
        Err(e) => return e,
    };

    let (parsed_rows, mut warnings, rows_total) = parse_exchange_rows(&text);
    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return err(&req.id, "db_tx_failed", e.to_string(), None),
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
            return err(
                &req.id,
                "db_delete_failed",
                e.to_string(),
                Some(json!({ "table": "scores" })),
            );
        }
    }

    let mut updated = 0usize;
    let mut skipped = 0usize;
    for row in &parsed_rows {
        let student_id = row.student_id.as_str();
        let mark_set_code = row.mark_set_code.as_str();
        let assessment_idx = row.assessment_idx;
        let status = row.status.as_str();
        let raw_value = row.raw_value;

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
            skipped += 1;
            warnings.push(json!({
                "line": row.line_no,
                "code": "missing_student",
                "message": "student_id does not belong to target class"
            }));
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
            skipped += 1;
            warnings.push(json!({
                "line": row.line_no,
                "code": "missing_assessment",
                "message": "assessment not found in target class/mark set"
            }));
            continue;
        };
        let (resolved_raw, resolved_state) = match resolve_score_state(Some(&status), raw_value) {
            Ok(v) => v,
            Err(e) => {
                skipped += 1;
                warnings.push(json!({
                    "line": row.line_no,
                    "code": e.code,
                    "message": e.message
                }));
                continue;
            }
        };
        if let Err(e) = upsert_score(
            &tx,
            &assessment_id,
            student_id,
            resolved_raw,
            resolved_state,
        ) {
            let _ = tx.rollback();
            return e.response(&req.id);
        }
        updated += 1;
    }

    if let Err(e) = tx.commit() {
        return err(&req.id, "db_commit_failed", e.to_string(), None);
    }

    ok(
        &req.id,
        json!({
            "ok": true,
            "updated": updated,
            "rowsTotal": rows_total,
            "rowsParsed": parsed_rows.len(),
            "skipped": skipped,
            "warningsCount": warnings.len(),
            "warnings": warnings,
            "mode": mode,
            "path": in_path
        }),
    )
}

fn handle_exchange_import_class_csv(state: &mut AppState, req: &Request) -> serde_json::Value {
    handle_exchange_apply_class_csv(state, req)
}

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "backup.exportWorkspaceBundle" => Some(handle_backup_export_workspace_bundle(state, req)),
        "backup.importWorkspaceBundle" => Some(handle_backup_import_workspace_bundle(state, req)),
        "exchange.exportClassCsv" => Some(handle_exchange_export_class_csv(state, req)),
        "exchange.previewClassCsv" => Some(handle_exchange_preview_class_csv(state, req)),
        "exchange.applyClassCsv" => Some(handle_exchange_apply_class_csv(state, req)),
        "exchange.importClassCsv" => Some(handle_exchange_import_class_csv(state, req)),
        _ => None,
    }
}
