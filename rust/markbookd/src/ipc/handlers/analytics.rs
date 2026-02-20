use crate::calc;
use crate::ipc::error::{err, ok};
use crate::ipc::types::{AppState, Request};
use rusqlite::{params_from_iter, types::Value, Connection, OptionalExtension};
use serde_json::json;
use std::collections::{HashMap, HashSet};

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

fn parse_filters(req: &Request) -> Result<calc::SummaryFilters, serde_json::Value> {
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
) -> Result<Option<HashSet<String>>, calc::CalcError> {
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

    let mut keep = HashSet::new();
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

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 1 {
        Some(values[mid])
    } else {
        Some((values[mid - 1] + values[mid]) / 2.0)
    }
}

fn apply_scope(
    summary: &mut calc::SummaryModel,
    allowed_ids: Option<&HashSet<String>>,
) -> Vec<calc::StudentFinal> {
    if let Some(allowed) = allowed_ids {
        summary
            .per_student
            .retain(|s| allowed.contains(s.student_id.as_str()));
        if let Some(rows) = summary.per_student_categories.as_mut() {
            rows.retain(|r| allowed.contains(r.student_id.as_str()));
        }
    }
    summary.per_student.clone()
}

fn handle_analytics_filters_options(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let mark_set_id = match required_str(req, "markSetId") {
        Ok(v) => v,
        Err(e) => return e,
    };

    let mut stmt = match conn.prepare(
        "SELECT DISTINCT term, category_name
         FROM assessments
         WHERE mark_set_id = ?
         ORDER BY term, category_name",
    ) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    let rows = match stmt.query_map([&mark_set_id], |r| {
        let term: Option<i64> = r.get(0)?;
        let category_name: Option<String> = r.get(1)?;
        Ok((term, category_name))
    }) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    let mut terms_set: HashSet<i64> = HashSet::new();
    let mut categories_set: HashSet<String> = HashSet::new();
    for (term, category_name) in rows.flatten() {
        if let Some(t) = term {
            terms_set.insert(t);
        }
        if let Some(cat) = category_name {
            let c = cat.trim();
            if !c.is_empty() {
                categories_set.insert(c.to_string());
            }
        }
    }

    let mut terms: Vec<i64> = terms_set.into_iter().collect();
    terms.sort_unstable();
    let mut categories: Vec<String> = categories_set.into_iter().collect();
    categories.sort();

    ok(
        &req.id,
        json!({
            "terms": terms,
            "categories": categories,
            "types": [
                { "bit": 0, "key": "summative", "label": "Summative" },
                { "bit": 1, "key": "formative", "label": "Formative" },
                { "bit": 2, "key": "diagnostic", "label": "Diagnostic" },
                { "bit": 3, "key": "self", "label": "Self" },
                { "bit": 4, "key": "peer", "label": "Peer" }
            ],
            "studentScopes": ["all", "active", "valid"]
        }),
    )
}

fn handle_analytics_class_open(state: &mut AppState, req: &Request) -> serde_json::Value {
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
    let filters = match parse_filters(req) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let student_scope = match parse_student_scope(req) {
        Ok(v) => v,
        Err(e) => return e,
    };

    let mut summary = match calc::compute_mark_set_summary(
        &calc_context(conn, &class_id, &mark_set_id),
        &filters,
    ) {
        Ok(v) => v,
        Err(e) => return calc_err(req, e),
    };

    let allowed = match student_id_scope_filter(conn, &class_id, &mark_set_id, student_scope) {
        Ok(v) => v,
        Err(e) => return calc_err(req, e),
    };
    let rows = apply_scope(&mut summary, allowed.as_ref());

    let mut final_marks: Vec<f64> = rows.iter().filter_map(|r| r.final_mark).collect();
    let final_mark_count = final_marks.len();
    let class_average = if final_marks.is_empty() {
        None
    } else {
        Some(final_marks.iter().sum::<f64>() / (final_marks.len() as f64))
    };
    let class_median = median(final_marks.as_mut_slice());

    let total_no_mark: i64 = rows.iter().map(|r| r.no_mark_count).sum();
    let total_zero: i64 = rows.iter().map(|r| r.zero_count).sum();
    let total_scored: i64 = rows.iter().map(|r| r.scored_count).sum();
    let total_counts = total_no_mark + total_zero + total_scored;

    let no_mark_rate = if total_counts > 0 {
        (total_no_mark as f64) / (total_counts as f64)
    } else {
        0.0
    };
    let zero_rate = if total_counts > 0 {
        (total_zero as f64) / (total_counts as f64)
    } else {
        0.0
    };

    let bins = vec![
        ("0-49", 0.0_f64, 49.9_f64),
        ("50-59", 50.0_f64, 59.9_f64),
        ("60-69", 60.0_f64, 69.9_f64),
        ("70-79", 70.0_f64, 79.9_f64),
        ("80-89", 80.0_f64, 89.9_f64),
        ("90-100", 90.0_f64, 100.0_f64),
    ];
    let distributions = bins
        .iter()
        .map(|(label, min, max)| {
            let count = rows
                .iter()
                .filter_map(|r| r.final_mark)
                .filter(|v| *v >= *min && *v <= *max)
                .count();
            json!({
                "label": label,
                "min": min,
                "max": max,
                "count": count
            })
        })
        .collect::<Vec<_>>();

    let no_final_mark_count = rows.iter().filter(|r| r.final_mark.is_none()).count();

    let mut ranked = rows
        .iter()
        .filter(|r| r.final_mark.is_some())
        .cloned()
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| {
        let a_key = a.final_mark.unwrap_or(f64::MIN);
        let b_key = b.final_mark.unwrap_or(f64::MIN);
        b_key
            .partial_cmp(&a_key)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.sort_order.cmp(&b.sort_order))
    });
    let top = ranked.iter().take(5).cloned().collect::<Vec<_>>();
    let mut ranked_bottom = ranked.clone();
    ranked_bottom.reverse();
    let bottom = ranked_bottom.iter().take(5).cloned().collect::<Vec<_>>();

    ok(
        &req.id,
        json!({
            "class": summary.class,
            "markSet": summary.mark_set,
            "settings": summary.settings,
            "filters": summary.filters,
            "studentScope": student_scope.as_str(),
            "kpis": {
                "classAverage": class_average,
                "classMedian": class_median,
                "studentCount": rows.len(),
                "finalMarkCount": final_mark_count,
                "noMarkRate": no_mark_rate,
                "zeroRate": zero_rate
            },
            "distributions": {
                "bins": distributions,
                "noFinalMarkCount": no_final_mark_count
            },
            "perAssessment": summary.per_assessment,
            "perCategory": summary.per_category,
            "topBottom": {
                "top": top,
                "bottom": bottom
            },
            "rows": rows
        }),
    )
}

fn handle_analytics_student_open(state: &mut AppState, req: &Request) -> serde_json::Value {
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
    let filters = match parse_filters(req) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let student_scope = match parse_student_scope(req) {
        Ok(v) => v,
        Err(e) => return e,
    };

    let mut summary = match calc::compute_mark_set_summary(
        &calc_context(conn, &class_id, &mark_set_id),
        &filters,
    ) {
        Ok(v) => v,
        Err(e) => return calc_err(req, e),
    };

    let allowed = match student_id_scope_filter(conn, &class_id, &mark_set_id, student_scope) {
        Ok(v) => v,
        Err(e) => return calc_err(req, e),
    };
    apply_scope(&mut summary, allowed.as_ref());

    let Some(student) = summary
        .per_student
        .iter()
        .find(|s| s.student_id == student_id)
        .cloned()
    else {
        return err(&req.id, "not_found", "student not found in mark set", None);
    };

    let mut category_breakdown = Vec::new();
    if let Some(rows) = summary.per_student_categories.as_ref() {
        if let Some(found) = rows.iter().find(|r| r.student_id == student_id) {
            category_breakdown = found.categories.clone();
        }
    }

    let mut class_stats_by_assessment: HashMap<&str, &calc::AssessmentStats> = HashMap::new();
    for stat in &summary.per_assessment {
        class_stats_by_assessment.insert(stat.assessment_id.as_str(), stat);
    }

    let mut score_by_assessment: HashMap<String, (Option<f64>, String)> = HashMap::new();
    if !summary.assessments.is_empty() {
        let placeholders = std::iter::repeat("?")
            .take(summary.assessments.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT assessment_id, raw_value, status
             FROM scores
             WHERE student_id = ? AND assessment_id IN ({})",
            placeholders
        );
        let mut values: Vec<Value> = Vec::with_capacity(summary.assessments.len() + 1);
        values.push(Value::Text(student_id.clone()));
        for a in &summary.assessments {
            values.push(Value::Text(a.assessment_id.clone()));
        }

        let mut stmt = match conn.prepare(&sql) {
            Ok(v) => v,
            Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
        };
        let rows = match stmt.query_map(params_from_iter(values), |r| {
            let assessment_id: String = r.get(0)?;
            let raw_value: Option<f64> = r.get(1)?;
            let status: String = r.get(2)?;
            Ok((assessment_id, raw_value, status))
        }) {
            Ok(v) => v,
            Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
        };
        for (assessment_id, raw_value, status) in rows.flatten() {
            score_by_assessment.insert(assessment_id, (raw_value, status));
        }
    }

    let mut assessment_trail = Vec::with_capacity(summary.assessments.len());
    for a in &summary.assessments {
        let (raw_value, status) = score_by_assessment
            .get(a.assessment_id.as_str())
            .cloned()
            .unwrap_or((None, "no_mark".to_string()));
        let score = match status.as_str() {
            "zero" => Some(0.0),
            "scored" => raw_value,
            _ => None,
        };
        let percent = score.map(|s| {
            if a.out_of > 0.0 {
                calc::round_off_1_decimal((s * 100.0) / a.out_of)
            } else {
                0.0
            }
        });
        let class_stats = class_stats_by_assessment.get(a.assessment_id.as_str());
        assessment_trail.push(json!({
            "assessmentId": a.assessment_id,
            "idx": a.idx,
            "title": a.title,
            "date": a.date,
            "categoryName": a.category_name,
            "term": a.term,
            "legacyType": a.legacy_type,
            "weight": a.weight,
            "outOf": a.out_of,
            "status": status,
            "score": score,
            "percent": percent,
            "classAvgRaw": class_stats.map(|s| s.avg_raw),
            "classAvgPercent": class_stats.map(|s| s.avg_percent)
        }));
    }

    let mut attendance_months_with_data = 0usize;
    let mut attendance_coded_days = 0usize;
    let mut attendance_stmt = match conn.prepare(
        "SELECT day_codes
         FROM attendance_student_months
         WHERE class_id = ? AND student_id = ?",
    ) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let attendance_rows = match attendance_stmt.query_map([&class_id, &student_id], |r| {
        let day_codes: String = r.get(0)?;
        Ok(day_codes)
    }) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    for day_codes in attendance_rows.flatten() {
        attendance_months_with_data += 1;
        attendance_coded_days += day_codes.chars().filter(|c| !c.is_whitespace()).count();
    }
    let attendance_summary = if attendance_months_with_data > 0 {
        Some(json!({
            "monthsWithData": attendance_months_with_data,
            "codedDays": attendance_coded_days
        }))
    } else {
        None
    };

    let mut payload = json!({
        "class": summary.class,
        "markSet": summary.mark_set,
        "settings": summary.settings,
        "filters": summary.filters,
        "studentScope": student_scope.as_str(),
        "student": student,
        "finalMark": student.final_mark,
        "counts": {
            "noMark": student.no_mark_count,
            "zero": student.zero_count,
            "scored": student.scored_count
        },
        "categoryBreakdown": category_breakdown,
        "assessmentTrail": assessment_trail
    });
    if let Some(a) = attendance_summary {
        if let Some(obj) = payload.as_object_mut() {
            obj.insert("attendanceSummary".to_string(), a);
        }
    }
    ok(&req.id, payload)
}

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "analytics.class.open" => Some(handle_analytics_class_open(state, req)),
        "analytics.student.open" => Some(handle_analytics_student_open(state, req)),
        "analytics.filters.options" => Some(handle_analytics_filters_options(state, req)),
        _ => None,
    }
}
