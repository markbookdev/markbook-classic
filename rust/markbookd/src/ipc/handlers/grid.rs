use crate::ipc::error::{err, ok};
use crate::ipc::types::{AppState, Request};
use rusqlite::types::Value;
use rusqlite::{params_from_iter, Connection, OptionalExtension};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

const GRID_GET_MAX_ROWS: i64 = 2000;
const GRID_GET_MAX_COLS: i64 = 256;
const GRID_BULK_UPDATE_MAX_EDITS: usize = 5000;

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

fn resolve_student_id_by_row(
    conn: &Connection,
    class_id: &str,
    row: i64,
) -> Result<String, HandlerErr> {
    let student_id: Option<String> = conn
        .query_row(
            "SELECT id FROM students WHERE class_id = ? AND sort_order = ?",
            (class_id, row),
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;

    student_id.ok_or_else(|| HandlerErr {
        code: "not_found",
        message: "student not found".to_string(),
        details: Some(json!({ "row": row })),
    })
}

fn resolve_assessment_id_by_col(
    conn: &Connection,
    mark_set_id: &str,
    col: i64,
) -> Result<String, HandlerErr> {
    let assessment_id: Option<String> = conn
        .query_row(
            "SELECT id FROM assessments WHERE mark_set_id = ? AND idx = ?",
            (mark_set_id, col),
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;

    assessment_id.ok_or_else(|| HandlerErr {
        code: "not_found",
        message: "assessment not found".to_string(),
        details: Some(json!({ "col": col })),
    })
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

fn handle_grid_get(state: &mut AppState, req: &Request) -> serde_json::Value {
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

    if row_start < 0 || col_start < 0 {
        return err(
            &req.id,
            "bad_params",
            "rowStart/colStart must be >= 0",
            Some(json!({
                "rowStart": row_start,
                "colStart": col_start
            })),
        );
    }
    if row_count_req < 0 || col_count_req < 0 {
        return err(
            &req.id,
            "bad_params",
            "rowCount/colCount must be >= 0",
            Some(json!({
                "rowCount": row_count_req,
                "colCount": col_count_req
            })),
        );
    }
    if row_count_req > GRID_GET_MAX_ROWS || col_count_req > GRID_GET_MAX_COLS {
        return err(
            &req.id,
            "bad_params",
            "requested grid range is too large",
            Some(json!({
                "rowCount": row_count_req,
                "colCount": col_count_req,
                "maxRows": GRID_GET_MAX_ROWS,
                "maxCols": GRID_GET_MAX_COLS
            })),
        );
    }

    let mut student_stmt = match conn
        .prepare("SELECT id FROM students WHERE class_id = ? ORDER BY sort_order LIMIT ? OFFSET ?")
    {
        Ok(s) => s,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let student_ids = match student_stmt
        .query_map((&class_id, row_count_req, row_start), |row| {
            row.get::<_, String>(0)
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    let mut assess_stmt = match conn
        .prepare("SELECT id FROM assessments WHERE mark_set_id = ? ORDER BY idx LIMIT ? OFFSET ?")
    {
        Ok(s) => s,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let assessment_ids = match assess_stmt
        .query_map((&mark_set_id, col_count_req, col_start), |row| {
            row.get::<_, String>(0)
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    let row_count = student_ids.len();
    let col_count = assessment_ids.len();
    let mut cells: Vec<Vec<Option<f64>>> = vec![vec![None; col_count]; row_count];

    if row_count > 0 && col_count > 0 {
        let assess_placeholders = std::iter::repeat_n("?", col_count)
            .collect::<Vec<_>>()
            .join(",");
        let stud_placeholders = std::iter::repeat_n("?", row_count)
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
            Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
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
            Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
        }
    }

    ok(
        &req.id,
        json!({
            "rowStart": row_start,
            "rowCount": row_count,
            "colStart": col_start,
            "colCount": col_count,
            "cells": cells
        }),
    )
}

fn handle_grid_update_cell(state: &mut AppState, req: &Request) -> serde_json::Value {
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
    let row = match req.params.get("row").and_then(|v| v.as_i64()) {
        Some(v) if v >= 0 => v,
        _ => return err(&req.id, "bad_params", "missing/invalid row", None),
    };
    let col = match req.params.get("col").and_then(|v| v.as_i64()) {
        Some(v) if v >= 0 => v,
        _ => return err(&req.id, "bad_params", "missing/invalid col", None),
    };

    let value = req.params.get("value").and_then(|v| v.as_f64());
    let (raw_value, status) = match resolve_score_state(None, value) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };

    let student_id = match resolve_student_id_by_row(conn, &class_id, row) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let assessment_id = match resolve_assessment_id_by_col(conn, &mark_set_id, col) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };

    if let Err(e) = upsert_score(conn, &assessment_id, &student_id, raw_value, status) {
        return e.response(&req.id);
    }

    ok(&req.id, json!({ "ok": true }))
}

fn handle_grid_set_state(state: &mut AppState, req: &Request) -> serde_json::Value {
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
    let row = match req.params.get("row").and_then(|v| v.as_i64()) {
        Some(v) if v >= 0 => v,
        _ => return err(&req.id, "bad_params", "missing/invalid row", None),
    };
    let col = match req.params.get("col").and_then(|v| v.as_i64()) {
        Some(v) if v >= 0 => v,
        _ => return err(&req.id, "bad_params", "missing/invalid col", None),
    };

    let state_value = req.params.get("state").and_then(|v| v.as_str());
    let value = req.params.get("value").and_then(|v| v.as_f64());
    let (raw_value, status) = match resolve_score_state(state_value, value) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };

    let student_id = match resolve_student_id_by_row(conn, &class_id, row) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let assessment_id = match resolve_assessment_id_by_col(conn, &mark_set_id, col) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };

    if let Err(e) = upsert_score(conn, &assessment_id, &student_id, raw_value, status) {
        return e.response(&req.id);
    }

    ok(&req.id, json!({ "ok": true }))
}

fn handle_grid_bulk_update(state: &mut AppState, req: &Request) -> serde_json::Value {
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
    let Some(edits_arr) = req.params.get("edits").and_then(|v| v.as_array()) else {
        return err(&req.id, "bad_params", "missing edits[]", None);
    };

    if edits_arr.len() > GRID_BULK_UPDATE_MAX_EDITS {
        let rejected = edits_arr.len();
        return ok(
            &req.id,
            json!({
                "ok": true,
                "updated": 0,
                "rejected": rejected,
                "limitExceeded": true,
                "errors": [{
                    "row": -1,
                    "col": -1,
                    "code": "too_many_edits",
                    "message": format!(
                        "bulk payload exceeds max edits: {} > {}",
                        rejected, GRID_BULK_UPDATE_MAX_EDITS
                    )
                }]
            }),
        );
    }

    let mut updated: usize = 0;
    let mut errors: Vec<serde_json::Value> = Vec::new();

    for (i, edit) in edits_arr.iter().enumerate() {
        let Some(obj) = edit.as_object() else {
            errors.push(json!({
                "row": -1,
                "col": -1,
                "code": "bad_params",
                "message": format!("edit at index {} must be an object", i),
            }));
            continue;
        };

        let row = match obj.get("row").and_then(|v| v.as_i64()) {
            Some(v) if v >= 0 => v,
            _ => {
                errors.push(json!({
                    "row": -1,
                    "col": -1,
                    "code": "bad_params",
                    "message": format!("edit at index {} missing/invalid row", i),
                }));
                continue;
            }
        };

        let col = match obj.get("col").and_then(|v| v.as_i64()) {
            Some(v) if v >= 0 => v,
            _ => {
                errors.push(json!({
                    "row": row,
                    "col": -1,
                    "code": "bad_params",
                    "message": format!("edit at index {} missing/invalid col", i),
                }));
                continue;
            }
        };

        let state_value = obj.get("state").and_then(|v| v.as_str());
        let value = obj.get("value").and_then(|v| v.as_f64());

        let (raw_value, status) = match resolve_score_state(state_value, value) {
            Ok(v) => v,
            Err(e) => {
                errors.push(json!({
                    "row": row,
                    "col": col,
                    "code": e.code,
                    "message": e.message,
                }));
                continue;
            }
        };

        let student_id = match resolve_student_id_by_row(conn, &class_id, row) {
            Ok(v) => v,
            Err(e) => {
                errors.push(json!({
                    "row": row,
                    "col": col,
                    "code": e.code,
                    "message": e.message,
                }));
                continue;
            }
        };
        let assessment_id = match resolve_assessment_id_by_col(conn, &mark_set_id, col) {
            Ok(v) => v,
            Err(e) => {
                errors.push(json!({
                    "row": row,
                    "col": col,
                    "code": e.code,
                    "message": e.message,
                }));
                continue;
            }
        };

        match upsert_score(conn, &assessment_id, &student_id, raw_value, status) {
            Ok(()) => updated += 1,
            Err(e) => errors.push(json!({
                "row": row,
                "col": col,
                "code": e.code,
                "message": e.message,
            })),
        }
    }

    let rejected = errors.len();
    let mut result = json!({ "ok": true, "updated": updated });
    if rejected > 0 {
        result
            .as_object_mut()
            .expect("result should be object")
            .insert("rejected".into(), json!(rejected));
        result
            .as_object_mut()
            .expect("result should be object")
            .insert("errors".into(), json!(errors));
    }

    ok(&req.id, result)
}

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "grid.get" => Some(handle_grid_get(state, req)),
        "grid.updateCell" => Some(handle_grid_update_cell(state, req)),
        "grid.setState" => Some(handle_grid_set_state(state, req)),
        "grid.bulkUpdate" => Some(handle_grid_bulk_update(state, req)),
        _ => None,
    }
}
