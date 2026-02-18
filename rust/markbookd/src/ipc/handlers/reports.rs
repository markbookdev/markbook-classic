use crate::calc;
use crate::ipc::error::{err, ok};
use crate::ipc::types::{AppState, Request};
use rusqlite::{params_from_iter, types::Value, Connection, OptionalExtension};
use serde_json::json;
use std::collections::HashMap;

use super::{assets, attendance};

fn required_str(req: &Request, key: &str) -> Result<String, serde_json::Value> {
    req.params
        .get(key)
        .and_then(|v| v.as_str())
        .map(|v| v.to_string())
        .ok_or_else(|| err(&req.id, "bad_params", format!("missing {}", key), None))
}

fn db_conn<'a>(state: &'a AppState, req: &Request) -> Result<&'a Connection, serde_json::Value> {
    state
        .db
        .as_ref()
        .ok_or_else(|| err(&req.id, "no_workspace", "select a workspace first", None))
}

fn parse_filters(req: &Request, default: bool) -> Result<calc::SummaryFilters, serde_json::Value> {
    if default {
        return Ok(calc::SummaryFilters::default());
    }
    calc::parse_summary_filters(req.params.get("filters")).map_err(|e| {
        err(
            &req.id,
            &e.code,
            e.message,
            e.details.map(|d| json!(d)).or(None),
        )
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StudentScope {
    All,
    Active,
    Valid,
}

impl StudentScope {
    fn as_str(self) -> &'static str {
        match self {
            StudentScope::All => "all",
            StudentScope::Active => "active",
            StudentScope::Valid => "valid",
        }
    }
}

fn parse_student_scope(req: &Request) -> Result<StudentScope, serde_json::Value> {
    match req
        .params
        .get("studentScope")
        .and_then(|v| v.as_str())
        .map(|s| s.to_ascii_lowercase())
        .as_deref()
    {
        None | Some("all") => Ok(StudentScope::All),
        Some("active") => Ok(StudentScope::Active),
        Some("valid") => Ok(StudentScope::Valid),
        Some(other) => Err(err(
            &req.id,
            "bad_params",
            "studentScope must be one of: all, active, valid",
            Some(json!({ "studentScope": other })),
        )),
    }
}

fn student_id_scope_filter(
    conn: &Connection,
    class_id: &str,
    mark_set_id: &str,
    scope: StudentScope,
) -> Result<Option<std::collections::HashSet<String>>, calc::CalcError> {
    if scope == StudentScope::All {
        return Ok(None);
    }

    let mark_set_sort_order: i64 = conn
        .query_row(
            "SELECT sort_order FROM mark_sets WHERE id = ? AND class_id = ?",
            (mark_set_id, class_id),
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| calc::CalcError::new("db_query_failed", e.to_string()))?
        .ok_or_else(|| calc::CalcError::new("not_found", "mark set not found"))?;

    let mut stmt = conn
        .prepare(
            "SELECT id, active, COALESCE(mark_set_mask, 'TBA')
             FROM students
             WHERE class_id = ?",
        )
        .map_err(|e| calc::CalcError::new("db_query_failed", e.to_string()))?;

    let ids = stmt
        .query_map([class_id], |r| {
            let id: String = r.get(0)?;
            let active: i64 = r.get(1)?;
            let mask: String = r.get(2)?;
            Ok((id, active != 0, mask))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| calc::CalcError::new("db_query_failed", e.to_string()))?;

    let mut keep = std::collections::HashSet::new();
    for (id, active, mask) in ids {
        let include = match scope {
            StudentScope::All => true,
            StudentScope::Active => active,
            StudentScope::Valid => calc::is_valid_kid(active, &mask, mark_set_sort_order),
        };
        if include {
            keep.insert(id);
        }
    }
    Ok(Some(keep))
}

fn calc_context<'a>(
    conn: &'a Connection,
    class_id: &'a str,
    mark_set_id: &'a str,
) -> calc::CalcContext<'a> {
    calc::CalcContext {
        conn,
        class_id,
        mark_set_id,
    }
}

fn calc_err(req: &Request, e: calc::CalcError) -> serde_json::Value {
    err(
        &req.id,
        &e.code,
        e.message,
        e.details.map(|d| json!(d)).or(None),
    )
}

fn handle_calc_assessment_stats(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let mark_set_id = match required_str(req, "markSetId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let filters = match parse_filters(req, false) {
        Ok(v) => v,
        Err(e) => return e,
    };

    match calc::compute_assessment_stats(&calc_context(conn, &class_id, &mark_set_id), &filters) {
        Ok(assessments) => ok(&req.id, json!({ "assessments": assessments })),
        Err(e) => calc_err(req, e),
    }
}

fn handle_calc_markset_summary(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let mark_set_id = match required_str(req, "markSetId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let filters = match parse_filters(req, false) {
        Ok(v) => v,
        Err(e) => return e,
    };

    match calc::compute_mark_set_summary(&calc_context(conn, &class_id, &mark_set_id), &filters) {
        Ok(summary) => ok(&req.id, json!(summary)),
        Err(e) => calc_err(req, e),
    }
}

fn handle_reports_markset_summary_model(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let mark_set_id = match required_str(req, "markSetId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let filters = match parse_filters(req, false) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let student_scope = match parse_student_scope(req) {
        Ok(v) => v,
        Err(e) => return e,
    };

    match calc::compute_mark_set_summary(&calc_context(conn, &class_id, &mark_set_id), &filters) {
        Ok(mut summary) => {
            if student_scope != StudentScope::All {
                let allowed = match student_id_scope_filter(
                    conn,
                    &class_id,
                    &mark_set_id,
                    student_scope,
                ) {
                    Ok(Some(v)) => v,
                    Ok(None) => std::collections::HashSet::new(),
                    Err(e) => return calc_err(req, e),
                };
                summary
                    .per_student
                    .retain(|s| allowed.contains(&s.student_id));
                if let Some(rows) = summary.per_student_categories.as_mut() {
                    rows.retain(|r| allowed.contains(&r.student_id));
                }
            }
            let mut payload = json!(summary);
            if let Some(obj) = payload.as_object_mut() {
                obj.insert(
                    "studentScope".to_string(),
                    serde_json::Value::String(student_scope.as_str().to_string()),
                );
            }
            ok(&req.id, payload)
        }
        Err(e) => calc_err(req, e),
    }
}

fn handle_reports_category_analysis_model(
    state: &mut AppState,
    req: &Request,
) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let mark_set_id = match required_str(req, "markSetId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let filters = match parse_filters(req, false) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let student_scope = match parse_student_scope(req) {
        Ok(v) => v,
        Err(e) => return e,
    };

    match calc::compute_mark_set_summary(&calc_context(conn, &class_id, &mark_set_id), &filters) {
        Ok(summary) => ok(
            &req.id,
            json!({
                "class": summary.class,
                "markSet": summary.mark_set,
                "settings": summary.settings,
                "filters": summary.filters,
                "categories": summary.categories,
                "perCategory": summary.per_category,
                "perAssessment": summary.per_assessment,
                "studentScope": student_scope.as_str(),
            }),
        ),
        Err(e) => calc_err(req, e),
    }
}

fn handle_reports_student_summary_model(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let mark_set_id = match required_str(req, "markSetId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let student_id = match required_str(req, "studentId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let filters = match parse_filters(req, false) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let student_scope = match parse_student_scope(req) {
        Ok(v) => v,
        Err(e) => return e,
    };

    match calc::compute_mark_set_summary(&calc_context(conn, &class_id, &mark_set_id), &filters) {
        Ok(summary) => {
            let student_scope_filter = match student_id_scope_filter(
                conn,
                &class_id,
                &mark_set_id,
                student_scope,
            ) {
                Ok(v) => v,
                Err(e) => return calc_err(req, e),
            };
            let student = summary
                .per_student
                .iter()
                .find(|s| s.student_id == student_id)
                .cloned();
            let Some(student) = student else {
                return err(&req.id, "not_found", "student not found in mark set", None);
            };
            if let Some(filter) = student_scope_filter {
                if !filter.contains(&student.student_id) {
                    return err(&req.id, "not_found", "student not found in mark set", None);
                }
            }
            ok(
                &req.id,
                json!({
                    "class": summary.class,
                    "markSet": summary.mark_set,
                    "settings": summary.settings,
                    "filters": summary.filters,
                    "studentScope": student_scope.as_str(),
                    "student": student,
                    "assessments": summary.assessments,
                    "perAssessment": summary.per_assessment,
                }),
            )
        }
        Err(e) => calc_err(req, e),
    }
}

fn handle_reports_attendance_monthly_model(
    state: &mut AppState,
    req: &Request,
) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let month = match required_str(req, "month") {
        Ok(v) => v,
        Err(e) => return e,
    };

    let class_name: Option<String> = match conn
        .query_row("SELECT name FROM classes WHERE id = ?", [&class_id], |r| {
            r.get(0)
        })
        .optional()
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let Some(class_name) = class_name else {
        return err(&req.id, "not_found", "class not found", None);
    };

    // Reuse the canonical month-open handler for the model shape and error semantics.
    let month_req = Request {
        id: req.id.clone(),
        method: "attendance.monthOpen".to_string(),
        params: json!({ "classId": class_id, "month": month }),
    };
    let Some(month_resp) = attendance::try_handle(state, &month_req) else {
        return err(
            &req.id,
            "server_error",
            "attendance.monthOpen handler missing",
            None,
        );
    };
    if month_resp.get("ok").and_then(|v| v.as_bool()) == Some(false) {
        return month_resp;
    }
    let model = month_resp
        .get("result")
        .cloned()
        .unwrap_or_else(|| json!({}));

    ok(
        &req.id,
        json!({
            "class": { "id": class_id, "name": class_name },
            "attendance": model,
        }),
    )
}

fn handle_reports_class_list_model(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };

    let class_name: Option<String> = match conn
        .query_row("SELECT name FROM classes WHERE id = ?", [&class_id], |r| {
            r.get(0)
        })
        .optional()
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let Some(class_name) = class_name else {
        return err(&req.id, "not_found", "class not found", None);
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
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
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
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    ok(
        &req.id,
        json!({
            "class": { "id": class_id, "name": class_name },
            "students": students
        }),
    )
}

fn handle_reports_learning_skills_summary_model(
    state: &mut AppState,
    req: &Request,
) -> serde_json::Value {
    // reports.learningSkillsSummaryModel matches the learningSkills.reportModel shape.
    let proxy_req = Request {
        id: req.id.clone(),
        method: "learningSkills.reportModel".to_string(),
        params: req.params.clone(),
    };
    match assets::try_handle(state, &proxy_req) {
        Some(resp) => resp,
        None => err(
            &req.id,
            "server_error",
            "learningSkills.reportModel handler missing",
            None,
        ),
    }
}

fn handle_reports_mark_set_grid_model(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(v) => v,
        Err(e) => return e,
    };

    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let mark_set_id = match required_str(req, "markSetId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let filters = match parse_filters(req, false) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let student_scope = match parse_student_scope(req) {
        Ok(v) => v,
        Err(e) => return e,
    };

    let class_name: Option<String> = match conn
        .query_row("SELECT name FROM classes WHERE id = ?", [&class_id], |r| {
            r.get(0)
        })
        .optional()
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let Some(class_name) = class_name else {
        return err(&req.id, "not_found", "class not found", None);
    };

    let ms_row: Option<(String, String, String, i64)> = match conn
        .query_row(
            "SELECT id, code, description, sort_order FROM mark_sets WHERE id = ? AND class_id = ?",
            (&mark_set_id, &class_id),
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let Some((ms_id, ms_code, ms_desc, mark_set_sort_order)) = ms_row else {
        return err(&req.id, "not_found", "mark set not found", None);
    };

    let mut stud_stmt = match conn.prepare(
        "SELECT id, last_name, first_name, sort_order, active, COALESCE(mark_set_mask, 'TBA')
         FROM students
         WHERE class_id = ?
         ORDER BY sort_order",
    ) {
        Ok(s) => s,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let student_rows: Vec<(String, serde_json::Value, bool, String)> = match stud_stmt
        .query_map([&class_id], |row| {
            let id: String = row.get(0)?;
            let id2 = id.clone();
            let last: String = row.get(1)?;
            let first: String = row.get(2)?;
            let sort_order: i64 = row.get(3)?;
            let active: i64 = row.get(4)?;
            let mask: String = row.get(5)?;
            let active_b = active != 0;
            let display_name = format!("{}, {}", last, first);
            let j = json!({
                "id": id,
                "displayName": display_name,
                "sortOrder": sort_order,
                "active": active_b
            });
            Ok((id2, j, active_b, mask))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    let mut student_ids: Vec<String> = Vec::with_capacity(student_rows.len());
    let mut students_json: Vec<serde_json::Value> = Vec::with_capacity(student_rows.len());
    let mut student_valid: Vec<bool> = Vec::with_capacity(student_rows.len());
    for (id, j, active_b, mask) in student_rows {
        student_ids.push(id);
        students_json.push(j);
        student_valid.push(calc::is_valid_kid(
            active_b,
            &mask,
            mark_set_sort_order,
        ));
    }

    let mut assess_stmt = match conn.prepare(
        "SELECT id, idx, date, category_name, title, weight, out_of FROM assessments WHERE mark_set_id = ? ORDER BY idx",
    ) {
        Ok(s) => s,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
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
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    let mut assessment_ids: Vec<String> = Vec::with_capacity(assessment_rows.len());
    let mut assessments_json: Vec<serde_json::Value> = Vec::with_capacity(assessment_rows.len());
    for (id, j) in assessment_rows {
        assessment_ids.push(id);
        assessments_json.push(j);
    }

    let source_row_count = student_ids.len();
    let col_count = assessment_ids.len();

    let mut source_cells: Vec<Vec<Option<f64>>> = vec![vec![None; col_count]; source_row_count];

    if source_row_count > 0 && col_count > 0 {
        let assess_placeholders = std::iter::repeat("?")
            .take(col_count)
            .collect::<Vec<_>>()
            .join(",");
        let stud_placeholders = std::iter::repeat("?")
            .take(source_row_count)
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT assessment_id, student_id, raw_value, status FROM scores
             WHERE assessment_id IN ({}) AND student_id IN ({})",
            assess_placeholders, stud_placeholders
        );

        let mut bind_values: Vec<Value> = Vec::with_capacity(col_count + source_row_count);
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
                    source_cells[r_i][c_i] = display_value;
                }
            }
            Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
        }
    }

    let keep_row = |row_idx: usize| -> bool {
        match student_scope {
            StudentScope::All => true,
            StudentScope::Active => students_json
                .get(row_idx)
                .and_then(|s| s.get("active"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            StudentScope::Valid => *student_valid.get(row_idx).unwrap_or(&false),
        }
    };

    let kept_row_indices: Vec<usize> = (0..source_row_count).filter(|i| keep_row(*i)).collect();
    let students_json: Vec<serde_json::Value> = kept_row_indices
        .iter()
        .filter_map(|idx| students_json.get(*idx).cloned())
        .collect();
    let cells: Vec<Vec<Option<f64>>> = kept_row_indices
        .iter()
        .filter_map(|idx| source_cells.get(*idx).cloned())
        .collect();
    let row_count = students_json.len();

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
            kept_row_indices.iter().filter_map(|r_i| {
                if !*student_valid.get(*r_i).unwrap_or(&true) {
                    return None;
                }
                match source_cells[*r_i][c_i] {
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

    ok(
        &req.id,
        json!({
            "class": { "id": class_id, "name": class_name },
            "markSet": { "id": ms_id, "code": ms_code, "description": ms_desc },
            "students": students_json,
            "assessments": assessments_json,
            "rowCount": row_count,
            "colCount": col_count,
            "assessmentAverages": assessment_averages,
            "cells": cells,
            "filters": filters,
            "studentScope": student_scope.as_str()
        }),
    )
}

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "calc.assessmentStats" => Some(handle_calc_assessment_stats(state, req)),
        "calc.markSetSummary" => Some(handle_calc_markset_summary(state, req)),
        "reports.markSetSummaryModel" => Some(handle_reports_markset_summary_model(state, req)),
        "reports.categoryAnalysisModel" => Some(handle_reports_category_analysis_model(state, req)),
        "reports.studentSummaryModel" => Some(handle_reports_student_summary_model(state, req)),
        "reports.attendanceMonthlyModel" => {
            Some(handle_reports_attendance_monthly_model(state, req))
        }
        "reports.classListModel" => Some(handle_reports_class_list_model(state, req)),
        "reports.learningSkillsSummaryModel" => {
            Some(handle_reports_learning_skills_summary_model(state, req))
        }
        "reports.markSetGridModel" => Some(handle_reports_mark_set_grid_model(state, req)),
        _ => None,
    }
}
