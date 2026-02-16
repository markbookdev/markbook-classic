use rusqlite::{params_from_iter, types::Value, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScoreState {
    NoMark,
    Zero,
    Scored(f64),
}

/// VB6-compatible 1-decimal rounding used in MarkBook:
/// `Int(10*x + 0.5) / 10`
#[allow(dead_code)]
pub fn round_off_1_decimal(x: f64) -> f64 {
    ((10.0 * x) + 0.5).floor() / 10.0
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssessmentAverage {
    pub avg_raw: f64,
    pub avg_percent: f64,
    pub scored_count: usize,
    pub zero_count: usize,
    pub no_mark_count: usize,
}

pub fn assessment_average<I>(scores: I, out_of: f64) -> AssessmentAverage
where
    I: IntoIterator<Item = ScoreState>,
{
    let mut denom: usize = 0;
    let mut sum_raw: f64 = 0.0;
    let mut scored_count: usize = 0;
    let mut zero_count: usize = 0;
    let mut no_mark_count: usize = 0;

    for s in scores {
        match s {
            ScoreState::NoMark => {
                no_mark_count += 1;
            }
            ScoreState::Zero => {
                zero_count += 1;
                denom += 1;
            }
            ScoreState::Scored(v) => {
                scored_count += 1;
                denom += 1;
                sum_raw += v;
            }
        }
    }

    let avg_raw = if denom > 0 {
        sum_raw / (denom as f64)
    } else {
        0.0
    };
    let avg_percent = if out_of > 0.0 {
        100.0 * avg_raw / out_of
    } else {
        0.0
    };

    AssessmentAverage {
        avg_raw,
        avg_percent,
        scored_count,
        zero_count,
        no_mark_count,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CalcError {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl CalcError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            details: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryFilters {
    pub term: Option<i64>,
    pub category_name: Option<String>,
    pub types_mask: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct CalcContext<'a> {
    pub conn: &'a Connection,
    pub class_id: &'a str,
    pub mark_set_id: &'a str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassSummary {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkSetSummary {
    pub id: String,
    pub code: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MarkSetSettings {
    pub full_code: Option<String>,
    pub room: Option<String>,
    pub day: Option<String>,
    pub period: Option<String>,
    pub weight_method: i64,
    pub calc_method: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryDef {
    pub name: String,
    pub weight: f64,
    pub sort_order: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentDef {
    pub assessment_id: String,
    pub idx: i64,
    pub date: Option<String>,
    pub category_name: Option<String>,
    pub title: String,
    pub term: Option<i64>,
    pub legacy_type: Option<i64>,
    pub weight: f64,
    pub out_of: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssessmentStats {
    pub assessment_id: String,
    pub idx: i64,
    pub date: Option<String>,
    pub category_name: Option<String>,
    pub title: String,
    pub out_of: f64,
    pub avg_raw: f64,
    pub avg_percent: f64,
    pub median_percent: f64,
    pub scored_count: usize,
    pub zero_count: usize,
    pub no_mark_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentFinal {
    pub student_id: String,
    pub display_name: String,
    pub sort_order: i64,
    pub active: bool,
    pub final_mark: Option<f64>,
    pub no_mark_count: i64,
    pub zero_count: i64,
    pub scored_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryAggregate {
    pub name: String,
    pub weight: f64,
    pub sort_order: Option<i64>,
    pub class_avg: f64,
    pub student_count: usize,
    pub assessment_count: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryModel {
    pub class: ClassSummary,
    #[serde(rename = "markSet")]
    pub mark_set: MarkSetSummary,
    pub settings: MarkSetSettings,
    pub filters: SummaryFilters,
    pub categories: Vec<CategoryDef>,
    pub assessments: Vec<AssessmentDef>,
    #[serde(rename = "perAssessment")]
    pub per_assessment: Vec<AssessmentStats>,
    #[serde(rename = "perCategory")]
    pub per_category: Vec<CategoryAggregate>,
    #[serde(rename = "perStudent")]
    pub per_student: Vec<StudentFinal>,
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

pub fn parse_summary_filters(raw: Option<&serde_json::Value>) -> Result<SummaryFilters, CalcError> {
    let Some(raw) = raw else {
        return Ok(SummaryFilters::default());
    };
    let Some(obj) = raw.as_object() else {
        return Err(CalcError::new("bad_params", "filters must be an object"));
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
                return Err(CalcError::new(
                    "bad_params",
                    "filters.term must be integer or 'ALL'",
                ));
            };
            Some(n)
        }
    };

    let category_name = match obj.get("categoryName") {
        None => None,
        Some(v) if v.is_null() => None,
        Some(v) => {
            let Some(s) = v.as_str() else {
                return Err(CalcError::new(
                    "bad_params",
                    "filters.categoryName must be string or null",
                ));
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
                return Err(CalcError::new(
                    "bad_params",
                    "filters.typesMask must be an integer bitmask",
                ));
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

pub fn compute_assessment_stats(
    ctx: &CalcContext<'_>,
    filters: &SummaryFilters,
) -> Result<Vec<AssessmentStats>, CalcError> {
    Ok(compute_mark_set_summary(ctx, filters)?.per_assessment)
}

pub fn compute_mark_set_summary(
    ctx: &CalcContext<'_>,
    filters: &SummaryFilters,
) -> Result<SummaryModel, CalcError> {
    let conn = ctx.conn;
    let class_id = ctx.class_id;
    let mark_set_id = ctx.mark_set_id;

    let class_name: Option<String> = conn
        .query_row("SELECT name FROM classes WHERE id = ?", [class_id], |r| {
            r.get(0)
        })
        .optional()
        .map_err(|e| CalcError::new("db_query_failed", e.to_string()))?;
    let Some(class_name) = class_name else {
        return Err(CalcError::new("not_found", "class not found"));
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
        .map_err(|e| CalcError::new("db_query_failed", e.to_string()))?;
    let Some((ms_code, ms_desc, full_code, room, day, period, weight_method, calc_method)) =
        mark_set_row
    else {
        return Err(CalcError::new("not_found", "mark set not found"));
    };

    let mut students_stmt = conn
        .prepare(
            "SELECT id, last_name, first_name, sort_order, active
             FROM students
             WHERE class_id = ?
             ORDER BY sort_order",
        )
        .map_err(|e| CalcError::new("db_query_failed", e.to_string()))?;
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
        .map_err(|e| CalcError::new("db_query_failed", e.to_string()))?;

    let mut categories_stmt = conn
        .prepare(
            "SELECT name, COALESCE(weight, 0), sort_order
             FROM categories
             WHERE mark_set_id = ?
             ORDER BY sort_order",
        )
        .map_err(|e| CalcError::new("db_query_failed", e.to_string()))?;
    let categories: Vec<SummaryCategory> = categories_stmt
        .query_map([mark_set_id], |r| {
            Ok(SummaryCategory {
                name: r.get(0)?,
                weight: r.get::<_, f64>(1)?,
                sort_order: r.get(2)?,
            })
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| CalcError::new("db_query_failed", e.to_string()))?;

    let mut assessments_stmt = conn
        .prepare(
            "SELECT id, idx, date, category_name, title, term, legacy_type, weight, out_of
             FROM assessments
             WHERE mark_set_id = ?
             ORDER BY idx",
        )
        .map_err(|e| CalcError::new("db_query_failed", e.to_string()))?;
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
        .map_err(|e| CalcError::new("db_query_failed", e.to_string()))?;

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

    let mut score_by_pair: HashMap<(String, String), ScoreState> = HashMap::new();
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

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| CalcError::new("db_query_failed", e.to_string()))?;
        let rows = stmt
            .query_map(params_from_iter(bind_values), |r| {
                let assessment_id: String = r.get(0)?;
                let student_id: String = r.get(1)?;
                let raw_value: Option<f64> = r.get(2)?;
                let status: String = r.get(3)?;
                Ok((assessment_id, student_id, raw_value, status))
            })
            .map_err(|e| CalcError::new("db_query_failed", e.to_string()))?;
        for row in rows {
            let (assessment_id, student_id, raw_value, status) =
                row.map_err(|e| CalcError::new("db_query_failed", e.to_string()))?;
            let state = match status.as_str() {
                "no_mark" => ScoreState::NoMark,
                "zero" => ScoreState::Zero,
                "scored" => ScoreState::Scored(raw_value.unwrap_or(0.0)),
                _ => raw_value
                    .map(ScoreState::Scored)
                    .unwrap_or(ScoreState::NoMark),
            };
            score_by_pair.insert((assessment_id, student_id), state);
        }
    }

    let mut per_assessment: Vec<AssessmentStats> = Vec::new();
    for a in &selected_assessments {
        let mut score_states: Vec<ScoreState> = Vec::new();
        let mut median_values: Vec<f64> = Vec::new();
        for s in &students {
            if !s.active {
                continue;
            }
            let state = score_by_pair
                .get(&(a.id.clone(), s.id.clone()))
                .copied()
                .unwrap_or(ScoreState::NoMark);
            match state {
                ScoreState::NoMark => {}
                ScoreState::Zero => median_values.push(0.0),
                ScoreState::Scored(v) => {
                    if a.out_of > 0.0 {
                        median_values.push(100.0 * v / a.out_of);
                    } else {
                        median_values.push(0.0);
                    }
                }
            }
            score_states.push(state);
        }

        let stats = assessment_average(score_states, a.out_of);
        per_assessment.push(AssessmentStats {
            assessment_id: a.id.clone(),
            idx: a.idx,
            date: a.date.clone(),
            category_name: a.category_name.clone(),
            title: a.title.clone(),
            out_of: a.out_of,
            avg_raw: round_off_1_decimal(stats.avg_raw),
            avg_percent: round_off_1_decimal(stats.avg_percent),
            median_percent: round_off_1_decimal(compute_median(&median_values)),
            scored_count: stats.scored_count,
            zero_count: stats.zero_count,
            no_mark_count: stats.no_mark_count,
        });
    }

    let mut category_weight_map: HashMap<String, f64> = HashMap::new();
    for c in &categories {
        category_weight_map.insert(c.name.to_ascii_lowercase(), c.weight);
    }

    let mut per_student: Vec<StudentFinal> = Vec::new();
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
                .unwrap_or(ScoreState::NoMark);
            let assessment_weight = if a.weight > 0.0 { a.weight } else { 1.0 };
            let percent_opt = match state {
                ScoreState::NoMark => {
                    no_mark_count += 1;
                    None
                }
                ScoreState::Zero => {
                    zero_count += 1;
                    Some(0.0)
                }
                ScoreState::Scored(v) => {
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

        let final_mark = final_mark_raw.map(round_off_1_decimal);
        per_student.push(StudentFinal {
            student_id: s.id.clone(),
            display_name: s.display_name.clone(),
            sort_order: s.sort_order,
            active: s.active,
            final_mark,
            no_mark_count,
            zero_count,
            scored_count,
        });

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

    let mut per_category: Vec<CategoryAggregate> = per_category_totals
        .into_iter()
        .map(|(name, (sum, count, sort_order, weight))| {
            let class_avg = if count > 0 {
                round_off_1_decimal(sum / (count as f64))
            } else {
                0.0
            };
            let assessment_count = per_category_assessment_counts
                .get(&name)
                .copied()
                .unwrap_or(0);
            CategoryAggregate {
                name,
                weight,
                sort_order: if sort_order == i64::MAX {
                    None
                } else {
                    Some(sort_order)
                },
                class_avg,
                student_count: count,
                assessment_count,
            }
        })
        .collect();
    per_category.sort_by(|a, b| {
        let a_sort = a.sort_order.unwrap_or(i64::MAX);
        let b_sort = b.sort_order.unwrap_or(i64::MAX);
        a_sort.cmp(&b_sort)
    });

    let categories_out: Vec<CategoryDef> = categories
        .iter()
        .map(|c| CategoryDef {
            name: c.name.clone(),
            weight: c.weight,
            sort_order: c.sort_order,
        })
        .collect();

    let assessments: Vec<AssessmentDef> = selected_assessments
        .iter()
        .map(|a| AssessmentDef {
            assessment_id: a.id.clone(),
            idx: a.idx,
            date: a.date.clone(),
            category_name: a.category_name.clone(),
            title: a.title.clone(),
            term: a.term,
            legacy_type: a.legacy_type,
            weight: a.weight,
            out_of: a.out_of,
        })
        .collect();

    Ok(SummaryModel {
        class: ClassSummary {
            id: class_id.to_string(),
            name: class_name,
        },
        mark_set: MarkSetSummary {
            id: mark_set_id.to_string(),
            code: ms_code,
            description: ms_desc,
        },
        settings: MarkSetSettings {
            full_code,
            room,
            day,
            period,
            weight_method,
            calc_method,
        },
        filters: filters.clone(),
        categories: categories_out,
        assessments,
        per_assessment,
        per_category,
        per_student,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy::{parse_legacy_mark_file, LegacyScore};
    use std::path::PathBuf;

    fn fixture_path(rel: &str) -> PathBuf {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        base.join("../../").join(rel)
    }

    #[test]
    fn round_off_matches_vb6() {
        assert_eq!(round_off_1_decimal(0.0), 0.0);
        assert_eq!(round_off_1_decimal(3.54), 3.5);
        assert_eq!(round_off_1_decimal(3.55), 3.6);
        assert_eq!(round_off_1_decimal(35.6818), 35.7);
    }

    #[test]
    fn assessment_average_counts_no_mark_vs_zero() {
        let p = fixture_path("fixtures/legacy/Sample25/MB8D25/MAT18D.Y25");
        let mf = parse_legacy_mark_file(&p).expect("parse mark file");
        let a0 = &mf.assessments[0];

        let avg = assessment_average(
            a0.raw_scores.iter().copied().map(|s| match s {
                LegacyScore::NoMark => ScoreState::NoMark,
                LegacyScore::Zero => ScoreState::Zero,
                LegacyScore::Scored(v) => ScoreState::Scored(v),
            }),
            a0.out_of,
        );

        assert_eq!(avg.no_mark_count, 5);
        assert_eq!(avg.zero_count, 2);
        assert_eq!(avg.scored_count, 20);

        // VB6 Calculate semantics: denom counts all non-NoMark rows (Scored + Zero).
        // avg_raw here is computed from the fixture's raw values, not the file summary line.
        let expected_avg_raw = 78.5 / 22.0;
        assert!((avg.avg_raw - expected_avg_raw).abs() < 1e-9);
    }

    #[test]
    fn parse_filters_accepts_all_term_string() {
        let raw = serde_json::json!({
            "term": "ALL",
            "categoryName": "ALL",
            "typesMask": null
        });
        let parsed = parse_summary_filters(Some(&raw)).expect("parse filters");
        assert_eq!(parsed.term, None);
        assert_eq!(parsed.category_name, None);
        assert_eq!(parsed.types_mask, None);
    }
}
