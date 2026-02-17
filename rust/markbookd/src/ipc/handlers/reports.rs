use crate::calc;
use crate::ipc::error::{err, ok};
use crate::ipc::router::handle_request_legacy;
use crate::ipc::types::{AppState, Request};
use rusqlite::Connection;
use serde_json::json;

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
    let filters = match parse_filters(req, true) {
        Ok(v) => v,
        Err(e) => return e,
    };

    match calc::compute_mark_set_summary(&calc_context(conn, &class_id, &mark_set_id), &filters) {
        Ok(summary) => ok(&req.id, json!(summary)),
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

    match calc::compute_mark_set_summary(&calc_context(conn, &class_id, &mark_set_id), &filters) {
        Ok(summary) => {
            let student = summary
                .per_student
                .iter()
                .find(|s| s.student_id == student_id)
                .cloned();
            let Some(student) = student else {
                return err(&req.id, "not_found", "student not found in mark set", None);
            };
            ok(
                &req.id,
                json!({
                    "class": summary.class,
                    "markSet": summary.mark_set,
                    "settings": summary.settings,
                    "filters": summary.filters,
                    "student": student,
                    "assessments": summary.assessments,
                    "perAssessment": summary.per_assessment,
                }),
            )
        }
        Err(e) => calc_err(req, e),
    }
}

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "calc.assessmentStats" => Some(handle_calc_assessment_stats(state, req)),
        "calc.markSetSummary" => Some(handle_calc_markset_summary(state, req)),
        "reports.markSetSummaryModel" => Some(handle_reports_markset_summary_model(state, req)),
        "reports.categoryAnalysisModel" => Some(handle_reports_category_analysis_model(state, req)),
        "reports.studentSummaryModel" => Some(handle_reports_student_summary_model(state, req)),
        "reports.attendanceMonthlyModel"
        | "reports.classListModel"
        | "reports.learningSkillsSummaryModel"
        | "reports.markSetGridModel" => Some(handle_request_legacy(state, req.clone())),
        _ => None,
    }
}
