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

#[derive(Debug, Clone)]
struct CombinedMarkSetMeta {
    id: String,
    code: String,
    description: String,
    sort_order: i64,
    weight: f64,
    deleted_at: Option<String>,
}

#[derive(Debug, Clone)]
struct ClassStudentRow {
    id: String,
    display_name: String,
    sort_order: i64,
    active: bool,
    mask: String,
}

fn analytics_types_json() -> Vec<serde_json::Value> {
    vec![
        json!({ "bit": 0, "key": "summative", "label": "Summative" }),
        json!({ "bit": 1, "key": "formative", "label": "Formative" }),
        json!({ "bit": 2, "key": "diagnostic", "label": "Diagnostic" }),
        json!({ "bit": 3, "key": "self", "label": "Self" }),
        json!({ "bit": 4, "key": "peer", "label": "Peer" }),
    ]
}

fn parse_mark_set_ids(req: &Request) -> Result<Vec<String>, serde_json::Value> {
    let Some(raw) = req.params.get("markSetIds").and_then(|v| v.as_array()) else {
        return Err(err(&req.id, "bad_params", "missing markSetIds", None));
    };
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for v in raw {
        let Some(id) = v.as_str() else {
            return Err(err(
                &req.id,
                "bad_params",
                "markSetIds must contain only strings",
                None,
            ));
        };
        let trimmed = id.trim();
        if trimmed.is_empty() {
            return Err(err(
                &req.id,
                "bad_params",
                "markSetIds must not contain empty ids",
                None,
            ));
        }
        let owned = trimmed.to_string();
        if seen.insert(owned.clone()) {
            out.push(owned);
        }
    }
    if out.is_empty() {
        return Err(err(
            &req.id,
            "bad_params",
            "markSetIds must contain at least one mark set id",
            None,
        ));
    }
    Ok(out)
}

fn load_class_students(conn: &Connection, class_id: &str) -> Result<Vec<ClassStudentRow>, calc::CalcError> {
    let mut stmt = conn
        .prepare(
            "SELECT id, last_name, first_name, sort_order, active, COALESCE(mark_set_mask, 'TBA')
             FROM students
             WHERE class_id = ?
             ORDER BY sort_order",
        )
        .map_err(|e| calc::CalcError::new("db_query_failed", e.to_string()))?;
    stmt.query_map([class_id], |r| {
        let last: String = r.get(1)?;
        let first: String = r.get(2)?;
        Ok(ClassStudentRow {
            id: r.get(0)?,
            display_name: format!("{}, {}", last, first),
            sort_order: r.get(3)?,
            active: r.get::<_, i64>(4)? != 0,
            mask: r.get(5)?,
        })
    })
    .and_then(|it| it.collect::<Result<Vec<_>, _>>())
    .map_err(|e| calc::CalcError::new("db_query_failed", e.to_string()))
}

fn load_mark_sets_for_class(
    conn: &Connection,
    class_id: &str,
    mark_set_ids: Option<&[String]>,
) -> Result<Vec<CombinedMarkSetMeta>, calc::CalcError> {
    let mut out = Vec::new();
    if let Some(ids) = mark_set_ids {
        let placeholders = std::iter::repeat("?")
            .take(ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT id, code, description, sort_order, weight, deleted_at
             FROM mark_sets
             WHERE class_id = ? AND id IN ({})
             ORDER BY sort_order",
            placeholders
        );
        let mut values: Vec<Value> = Vec::with_capacity(ids.len() + 1);
        values.push(Value::Text(class_id.to_string()));
        for id in ids {
            values.push(Value::Text(id.clone()));
        }
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| calc::CalcError::new("db_query_failed", e.to_string()))?;
        let rows = stmt
            .query_map(params_from_iter(values), |r| {
                Ok(CombinedMarkSetMeta {
                    id: r.get(0)?,
                    code: r.get(1)?,
                    description: r.get(2)?,
                    sort_order: r.get(3)?,
                    weight: r.get::<_, f64>(4).unwrap_or(0.0),
                    deleted_at: r.get(5)?,
                })
            })
            .and_then(|it| it.collect::<Result<Vec<_>, _>>())
            .map_err(|e| calc::CalcError::new("db_query_failed", e.to_string()))?;
        out.extend(rows);
    } else {
        let mut stmt = conn
            .prepare(
                "SELECT id, code, description, sort_order, weight, deleted_at
                 FROM mark_sets
                 WHERE class_id = ? AND deleted_at IS NULL
                 ORDER BY sort_order",
            )
            .map_err(|e| calc::CalcError::new("db_query_failed", e.to_string()))?;
        let rows = stmt
            .query_map([class_id], |r| {
                Ok(CombinedMarkSetMeta {
                    id: r.get(0)?,
                    code: r.get(1)?,
                    description: r.get(2)?,
                    sort_order: r.get(3)?,
                    weight: r.get::<_, f64>(4).unwrap_or(0.0),
                    deleted_at: r.get(5)?,
                })
            })
            .and_then(|it| it.collect::<Result<Vec<_>, _>>())
            .map_err(|e| calc::CalcError::new("db_query_failed", e.to_string()))?;
        out.extend(rows);
    }
    Ok(out)
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

fn combined_distribution_bins(rows: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let bins = vec![
        ("0-49", 0.0_f64, 49.9_f64),
        ("50-59", 50.0_f64, 59.9_f64),
        ("60-69", 60.0_f64, 69.9_f64),
        ("70-79", 70.0_f64, 79.9_f64),
        ("80-89", 80.0_f64, 89.9_f64),
        ("90-100", 90.0_f64, 100.0_f64),
    ];
    bins.iter()
        .map(|(label, min, max)| {
            let count = rows
                .iter()
                .filter_map(|r| r.get("combinedFinal").and_then(|v| v.as_f64()))
                .filter(|v| *v >= *min && *v <= *max)
                .count();
            json!({
                "label": label,
                "min": min,
                "max": max,
                "count": count
            })
        })
        .collect::<Vec<_>>()
}

fn selected_mark_set_validity(
    student: &ClassStudentRow,
    mark_sets: &[CombinedMarkSetMeta],
) -> HashMap<String, bool> {
    let mut out = HashMap::new();
    for ms in mark_sets {
        out.insert(
            ms.id.clone(),
            calc::is_valid_kid(student.active, &student.mask, ms.sort_order),
        );
    }
    out
}

fn combined_student_is_in_scope(
    student: &ClassStudentRow,
    mark_sets: &[CombinedMarkSetMeta],
    scope: StudentScope,
) -> bool {
    match scope {
        StudentScope::All => true,
        StudentScope::Active => student.active,
        StudentScope::Valid => {
            mark_sets
                .iter()
                .any(|ms| calc::is_valid_kid(student.active, &student.mask, ms.sort_order))
        }
    }
}

fn normalize_mark_set_selection(
    req_id: &str,
    selected_ids: &[String],
    mark_sets: &[CombinedMarkSetMeta],
) -> Result<(), serde_json::Value> {
    if selected_ids.len() != mark_sets.len() {
        let found_ids: HashSet<&str> = mark_sets.iter().map(|m| m.id.as_str()).collect();
        let missing = selected_ids
            .iter()
            .filter(|id| !found_ids.contains(id.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        return Err(err(
            req_id,
            "bad_params",
            "markSetIds contains unknown mark set ids",
            Some(json!({ "missingMarkSetIds": missing })),
        ));
    }
    let deleted = mark_sets
        .iter()
        .filter_map(|m| {
            if m.deleted_at.is_some() {
                Some(json!({ "id": m.id, "code": m.code }))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if !deleted.is_empty() {
        return Err(err(
            req_id,
            "bad_params",
            "markSetIds must not include deleted mark sets",
            Some(json!({ "deletedMarkSets": deleted })),
        ));
    }
    Ok(())
}

fn combined_open_value(
    conn: &Connection,
    req_id: &str,
    class_id: &str,
    mark_set_ids: &[String],
    filters: &calc::SummaryFilters,
    student_scope: StudentScope,
) -> Result<serde_json::Value, serde_json::Value> {
    let mark_sets = load_mark_sets_for_class(conn, class_id, Some(mark_set_ids))
        .map_err(|e| err(req_id, &e.code, e.message, e.details.map(|d| json!(d))))?;
    if let Err(e) = normalize_mark_set_selection(req_id, mark_set_ids, &mark_sets) {
        return Err(e);
    }

    if mark_sets.is_empty() {
        return Err(err(
            req_id,
            "bad_params",
            "markSetIds must contain at least one mark set id",
            None,
        ));
    }

    let class_name: String = conn
        .query_row("SELECT name FROM classes WHERE id = ?", [class_id], |r| r.get(0))
        .optional()
        .map_err(|e| err(req_id, "db_query_failed", e.to_string(), None))?
        .ok_or_else(|| err(req_id, "not_found", "class not found", None))?;

    let students = load_class_students(conn, class_id)
        .map_err(|e| err(req_id, &e.code, e.message, e.details.map(|d| json!(d))))?;

    let mut summaries_by_mark_set: HashMap<String, calc::SummaryModel> = HashMap::new();
    for ms in &mark_sets {
        let summary = calc::compute_mark_set_summary(
            &calc_context(conn, class_id, ms.id.as_str()),
            filters,
        )
        .map_err(|e| err(req_id, &e.code, e.message, e.details.map(|d| json!(d))))?;
        summaries_by_mark_set.insert(ms.id.clone(), summary);
    }

    let mut student_final_by_mark_set: HashMap<String, HashMap<String, Option<f64>>> = HashMap::new();
    for (mark_set_id, summary) in &summaries_by_mark_set {
        let mut map = HashMap::new();
        for s in &summary.per_student {
            map.insert(s.student_id.clone(), s.final_mark);
        }
        student_final_by_mark_set.insert(mark_set_id.clone(), map);
    }

    let mut rows = Vec::new();
    let mut fallback_used_count = 0usize;
    for s in &students {
        if !combined_student_is_in_scope(s, &mark_sets, student_scope) {
            continue;
        }
        let validity = selected_mark_set_validity(s, &mark_sets);
        let mut per_set = Vec::new();
        let mut weighted_sum = 0.0_f64;
        let mut weighted_denom = 0.0_f64;
        let mut equal_vals = Vec::new();

        for ms in &mark_sets {
            let valid = *validity.get(ms.id.as_str()).unwrap_or(&true);
            let final_mark = if valid {
                student_final_by_mark_set
                    .get(ms.id.as_str())
                    .and_then(|m| m.get(s.id.as_str()))
                    .cloned()
                    .unwrap_or(None)
            } else {
                None
            };
            if let Some(v) = final_mark {
                equal_vals.push(v);
                if ms.weight > 0.0 {
                    weighted_sum += v * ms.weight;
                    weighted_denom += ms.weight;
                }
            }
            per_set.push(json!({
                "markSetId": ms.id,
                "code": ms.code,
                "description": ms.description,
                "weight": ms.weight,
                "valid": valid,
                "finalMark": final_mark
            }));
        }

        let combined_final = if equal_vals.is_empty() {
            None
        } else if weighted_denom > 0.0 {
            Some(calc::round_off_1_decimal(weighted_sum / weighted_denom))
        } else {
            fallback_used_count += 1;
            Some(calc::round_off_1_decimal(
                equal_vals.iter().sum::<f64>() / (equal_vals.len() as f64),
            ))
        };

        rows.push(json!({
            "studentId": s.id,
            "displayName": s.display_name,
            "sortOrder": s.sort_order,
            "active": s.active,
            "combinedFinal": combined_final,
            "perMarkSet": per_set
        }));
    }

    let mut combined_marks: Vec<f64> = rows
        .iter()
        .filter_map(|r| r.get("combinedFinal").and_then(|v| v.as_f64()))
        .collect();
    let final_mark_count = combined_marks.len();
    let class_average = if combined_marks.is_empty() {
        None
    } else {
        Some(calc::round_off_1_decimal(
            combined_marks.iter().sum::<f64>() / (combined_marks.len() as f64),
        ))
    };
    let class_median = median(combined_marks.as_mut_slice()).map(calc::round_off_1_decimal);
    let no_final_mark_count = rows
        .iter()
        .filter(|r| r.get("combinedFinal").and_then(|v| v.as_f64()).is_none())
        .count();

    let mut ranked = rows
        .iter()
        .filter_map(|r| {
            let mark = r.get("combinedFinal").and_then(|v| v.as_f64())?;
            let sort_order = r.get("sortOrder").and_then(|v| v.as_i64()).unwrap_or(i64::MAX);
            Some((mark, sort_order, r.clone()))
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.1.cmp(&b.1))
    });
    let top = ranked
        .iter()
        .take(5)
        .map(|(_, _, row)| row.clone())
        .collect::<Vec<_>>();
    let bottom = ranked
        .iter()
        .rev()
        .take(5)
        .map(|(_, _, row)| row.clone())
        .collect::<Vec<_>>();

    let per_mark_set = mark_sets
        .iter()
        .map(|ms| {
            let mut finals = rows
                .iter()
                .filter_map(|r| {
                    let per = r.get("perMarkSet")?.as_array()?;
                    let entry = per.iter().find(|e| {
                        e.get("markSetId")
                            .and_then(|v| v.as_str())
                            .map(|id| id == ms.id.as_str())
                            .unwrap_or(false)
                    })?;
                    entry.get("finalMark").and_then(|v| v.as_f64())
                })
                .collect::<Vec<_>>();
            let class_avg = if finals.is_empty() {
                None
            } else {
                Some(calc::round_off_1_decimal(
                    finals.iter().sum::<f64>() / (finals.len() as f64),
                ))
            };
            let class_median = median(finals.as_mut_slice()).map(calc::round_off_1_decimal);
            json!({
                "markSetId": ms.id,
                "code": ms.code,
                "description": ms.description,
                "weight": ms.weight,
                "finalMarkCount": finals.len(),
                "classAverage": class_avg,
                "classMedian": class_median
            })
        })
        .collect::<Vec<_>>();

    let mark_sets_json = mark_sets
        .iter()
        .map(|m| {
            json!({
                "id": m.id,
                "code": m.code,
                "description": m.description,
                "sortOrder": m.sort_order,
                "weight": m.weight
            })
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "class": { "id": class_id, "name": class_name },
        "markSets": mark_sets_json,
        "filters": filters,
        "studentScope": student_scope.as_str(),
        "settingsApplied": {
            "combineMethod": "weighted_markset",
            "fallbackUsedCount": fallback_used_count
        },
        "kpis": {
            "classAverage": class_average,
            "classMedian": class_median,
            "studentCount": rows.len(),
            "finalMarkCount": final_mark_count,
            "noCombinedFinalCount": no_final_mark_count
        },
        "distributions": {
            "bins": combined_distribution_bins(&rows),
            "noCombinedFinalCount": no_final_mark_count
        },
        "perMarkSet": per_mark_set,
        "rows": rows,
        "topBottom": {
            "top": top,
            "bottom": bottom
        }
    }))
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
            "types": analytics_types_json(),
            "studentScopes": ["all", "active", "valid"]
        }),
    )
}

fn handle_analytics_combined_options(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };

    let mark_sets = match load_mark_sets_for_class(conn, &class_id, None) {
        Ok(v) => v,
        Err(e) => return calc_err(req, e),
    };

    let mut stmt = match conn.prepare(
        "SELECT DISTINCT a.term, a.category_name
         FROM assessments a
         JOIN mark_sets ms ON ms.id = a.mark_set_id
         WHERE ms.class_id = ? AND ms.deleted_at IS NULL
         ORDER BY a.term, a.category_name",
    ) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let rows = match stmt.query_map([&class_id], |r| {
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

    let mark_sets_json = mark_sets
        .into_iter()
        .map(|m| {
            json!({
                "id": m.id,
                "code": m.code,
                "description": m.description,
                "sortOrder": m.sort_order,
                "weight": m.weight,
                "deletedAt": m.deleted_at
            })
        })
        .collect::<Vec<_>>();

    ok(
        &req.id,
        json!({
            "markSets": mark_sets_json,
            "terms": terms,
            "categories": categories,
            "types": analytics_types_json(),
            "studentScopes": ["all", "active", "valid"]
        }),
    )
}

fn handle_analytics_combined_open(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(v) => v,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let mark_set_ids = match parse_mark_set_ids(req) {
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

    match combined_open_value(conn, &req.id, &class_id, &mark_set_ids, &filters, student_scope) {
        Ok(v) => ok(&req.id, v),
        Err(e) => e,
    }
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
        "analytics.combined.options" => Some(handle_analytics_combined_options(state, req)),
        "analytics.combined.open" => Some(handle_analytics_combined_open(state, req)),
        _ => None,
    }
}
