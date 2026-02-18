use rusqlite::{params_from_iter, types::Value, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;

use crate::db;

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
pub struct ParityDiagnostics {
    pub calc_method_applied: i64,
    pub weight_method_applied: i64,
    pub selected_assessment_count: usize,
    pub selected_category_count: usize,
    pub selected_assessments_for_stats: usize,
    pub selected_assessments_for_calc: usize,
    pub excluded_by_weight_count: usize,
    pub excluded_by_category_weight_count: usize,
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
    // Optional diagnostic/UX helpers; older clients ignore unknown keys.
    #[serde(skip_serializing_if = "Option::is_none", rename = "settingsApplied")]
    pub settings_applied: Option<SettingsApplied>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "perStudentCategories"
    )]
    pub per_student_categories: Option<Vec<StudentCategoryBreakdown>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parity_diagnostics: Option<ParityDiagnostics>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsApplied {
    pub weight_method_applied: i64,
    pub calc_method_applied: i64,
    pub roff_applied: bool,
    pub mode_active_levels: i64,
    pub mode_level_vals: Vec<i64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentCategoryValue {
    pub name: String,
    pub value: Option<f64>,
    pub weight: f64,
    pub has_data: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentCategoryBreakdown {
    pub student_id: String,
    pub categories: Vec<StudentCategoryValue>,
}

#[derive(Debug, Clone)]
struct SummaryStudent {
    id: String,
    display_name: String,
    sort_order: i64,
    active: bool,
    mark_set_mask: String,
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

#[derive(Debug, Clone)]
struct ModeConfig {
    active_levels: usize,
    level_vals: Vec<i64>, // length 22, indices 0..21
    roff: bool,
}

fn default_mode_config() -> ModeConfig {
    let mut vals = vec![0_i64; 22];
    vals[0] = 0;
    vals[1] = 50;
    vals[2] = 60;
    vals[3] = 70;
    vals[4] = 80;
    ModeConfig {
        active_levels: 4,
        level_vals: vals,
        roff: true,
    }
}

fn load_mode_config(conn: &Connection) -> Result<ModeConfig, CalcError> {
    let mut cfg = default_mode_config();
    let mut mode = db::settings_get_json(conn, "user_cfg.override.mode_levels")
        .map_err(|e| CalcError::new("db_query_failed", e.to_string()))?;
    if mode.is_none() {
        mode = db::settings_get_json(conn, "user_cfg.mode_levels")
            .map_err(|e| CalcError::new("db_query_failed", e.to_string()))?;
    }
    if let Some(v) = mode {
        if let Some(obj) = v.as_object() {
            if let Some(n) = obj.get("activeLevels").and_then(|v| v.as_u64()) {
                cfg.active_levels = (n as usize).min(21);
            }
            if let Some(arr) = obj.get("vals").and_then(|v| v.as_array()) {
                let mut vals: Vec<i64> = Vec::with_capacity(22);
                for x in arr.iter().take(22) {
                    vals.push(x.as_i64().unwrap_or(0));
                }
                while vals.len() < 22 {
                    vals.push(0);
                }
                cfg.level_vals = vals;
            }
        }
    }
    let mut roff = db::settings_get_json(conn, "user_cfg.override.roff")
        .map_err(|e| CalcError::new("db_query_failed", e.to_string()))?;
    if roff.is_none() {
        roff = db::settings_get_json(conn, "user_cfg.roff")
            .map_err(|e| CalcError::new("db_query_failed", e.to_string()))?;
    }
    if let Some(v) = roff {
        if let Some(obj) = v.as_object() {
            if let Some(b) = obj.get("roff").and_then(|v| v.as_bool()) {
                cfg.roff = b;
            }
        }
    }
    Ok(cfg)
}

fn vb6_level_from_mark(cfg: &ModeConfig, mark_pct: f64) -> usize {
    let cmp = if cfg.roff {
        round_off_1_decimal(mark_pct)
    } else {
        mark_pct
    };
    let max_lvl = cfg.active_levels.min(21);
    let mut ml = 0usize;
    for lvl in 0..=max_lvl {
        if cmp >= cfg.level_vals.get(lvl).copied().unwrap_or(0) as f64 {
            ml = lvl;
        }
    }
    ml
}

fn vb6_midrange_mode(cfg: &ModeConfig, lvl: usize) -> f64 {
    let max_lvl = cfg.active_levels.min(21);
    let l = cfg.level_vals.get(lvl).copied().unwrap_or(0) as f64;
    let t = if lvl >= max_lvl {
        100.0
    } else {
        cfg.level_vals.get(lvl + 1).copied().unwrap_or(0) as f64
    };
    l + ((t - l) / 2.0)
}

#[derive(Debug, Clone, Copy)]
struct StudentEntry {
    pct: f64,
    entry_wt: f64,
    cat_idx: usize,
}

fn vb6_mode_mark(
    cfg: &ModeConfig,
    entries: &[StudentEntry],
    wrk_wt_meth: i64,
    cat_wt_sum: &[f64],
    wrk_cat_wt: &[f64],
    total_wt0: f64,
    cat_filter: Option<usize>,
) -> Option<f64> {
    if entries.is_empty() {
        return None;
    }
    let max_lvl = cfg.active_levels.min(21);
    let mut level_totals: Vec<f64> = vec![0.0; max_lvl + 1];
    let mut total = 0.0_f64;

    for e in entries {
        if let Some(cat) = cat_filter {
            if e.cat_idx != cat {
                continue;
            }
        }
        let lvl = vb6_level_from_mark(cfg, e.pct);
        let cat = e.cat_idx;
        let mut mode_val = 0.0_f64;

        if wrk_wt_meth == 1 {
            let denom_cat = cat_wt_sum.get(cat).copied().unwrap_or(0.0);
            if denom_cat > 0.0 {
                if let Some(_) = cat_filter {
                    // EvalOne_ModeCats / MedianCat != 0: no category-weight ratio.
                    mode_val = 100.0 * (e.entry_wt / denom_cat);
                } else if total_wt0 > 0.0 {
                    mode_val =
                        100.0 * (e.entry_wt / denom_cat) * (wrk_cat_wt.get(cat).copied().unwrap_or(0.0) / total_wt0);
                }
            }
        } else if total_wt0 > 0.0 {
            // Entry weighting: VB6 uses overall denom (EV_CatWT(k,0)) even for ModeCats.
            mode_val = 100.0 * (e.entry_wt / total_wt0);
        }

        if mode_val > 0.0 {
            if lvl <= max_lvl {
                level_totals[lvl] += mode_val;
                total += mode_val;
            }
        }
    }

    if total <= 0.0 {
        return None;
    }

    let mut best_lvl = 0usize;
    let mut best = 0.0_f64;
    for lvl in 0..=max_lvl {
        let next_best = round_off_1_decimal(100.0 * level_totals[lvl] / total);
        if next_best >= best {
            best = next_best;
            best_lvl = lvl;
        }
    }
    Some(vb6_midrange_mode(cfg, best_lvl))
}

fn vb6_median_mark(
    entries: &[StudentEntry],
    ev_wt_meth_for_weights: i64,
    wrk_wt_meth: i64,
    cat_wt_sum: &[f64],
    wrk_cat_wt: &[f64],
    total_wt0: f64,
    cat_filter: Option<usize>,
) -> Option<f64> {
    let mut pts: Vec<StudentEntry> = entries
        .iter()
        .copied()
        .filter(|e| cat_filter.map(|c| e.cat_idx == c).unwrap_or(true))
        .collect();
    if pts.is_empty() {
        return None;
    }
    pts.sort_by(|a, b| a.pct.partial_cmp(&b.pct).unwrap_or(Ordering::Equal));

    let n = pts.len();
    if ev_wt_meth_for_weights == 2 {
        // Equal weighting: plain median.
        if n == 1 {
            return Some(pts[0].pct);
        }
        if n % 2 == 1 {
            return Some(pts[n / 2].pct);
        }
        return Some((pts[(n / 2) - 1].pct + pts[n / 2].pct) / 2.0);
    }

    if n == 2 && (pts[0].entry_wt - pts[1].entry_wt).abs() < 1e-9 {
        return Some((pts[0].pct + pts[1].pct) / 2.0);
    }

    let denom_overall = if cat_filter.is_some() {
        let cat = cat_filter.unwrap();
        cat_wt_sum.get(cat).copied().unwrap_or(0.0)
    } else {
        total_wt0
    };
    if denom_overall <= 0.0 {
        return None;
    }

    let mut count_to_50 = 0.0_f64;
    for (idx, e) in pts.iter().enumerate() {
        let cat = e.cat_idx;
        let mut jump = 0.0_f64;
        if wrk_wt_meth == 1 {
            let denom_cat = cat_wt_sum.get(cat).copied().unwrap_or(0.0);
            if denom_cat > 0.0 {
                if let Some(_) = cat_filter {
                    jump = 100.0 * (e.entry_wt / denom_cat);
                } else if total_wt0 > 0.0 {
                    jump = 100.0
                        * (e.entry_wt / denom_cat)
                        * (wrk_cat_wt.get(cat).copied().unwrap_or(0.0) / total_wt0);
                }
            }
        } else {
            jump = 100.0 * (e.entry_wt / denom_overall);
        }

        count_to_50 += jump;
        if count_to_50 >= 50.0 {
            if (count_to_50 - 50.0).abs() < 1e-9 {
                if let Some(next) = pts.get(idx + 1) {
                    return Some((e.pct + next.pct) / 2.0);
                }
            }
            return Some(e.pct);
        }
    }

    pts.last().map(|e| e.pct)
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

pub(crate) fn is_valid_kid(active: bool, mark_set_mask: &str, mark_set_sort_order: i64) -> bool {
    if !active {
        return false;
    }
    let t = mark_set_mask.trim();
    if t.is_empty() {
        return true;
    }
    if t.eq_ignore_ascii_case("TBA") {
        return true;
    }
    let Ok(idx) = usize::try_from(mark_set_sort_order) else {
        return true;
    };
    let up = t.to_ascii_uppercase();
    if !up.chars().all(|ch| ch == '0' || ch == '1') {
        return true;
    }
    let bytes = up.as_bytes();
    if idx >= bytes.len() {
        return true;
    }
    bytes[idx] == b'1'
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

fn weighted_average(values: &[(f64, f64)]) -> Option<f64> {
    let mut sum = 0.0_f64;
    let mut denom = 0.0_f64;
    for (v, w) in values {
        if *w <= 0.0 {
            continue;
        }
        sum += *v * *w;
        denom += *w;
    }
    if denom > 0.0 {
        Some(sum / denom)
    } else {
        None
    }
}

fn weighted_median(values: &[(f64, f64)]) -> Option<f64> {
    let mut pts: Vec<(f64, f64)> = values.iter().copied().filter(|(_, w)| *w > 0.0).collect();
    if pts.is_empty() {
        return None;
    }
    pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));
    let total_weight: f64 = pts.iter().map(|(_, w)| *w).sum();
    if total_weight <= 0.0 {
        return None;
    }

    let mut cum = 0.0_f64;
    for (idx, (v, w)) in pts.iter().enumerate() {
        cum += *w;
        if (cum - (total_weight / 2.0)).abs() < 1e-9 {
            if let Some((next_v, _)) = pts.get(idx + 1) {
                return Some((v + next_v) / 2.0);
            }
            return Some(*v);
        }
        if cum > (total_weight / 2.0) {
            return Some(*v);
        }
    }
    pts.last().map(|(v, _)| *v)
}

fn mode_bucket_key(v: f64) -> i64 {
    (round_off_1_decimal(v) * 10.0).round() as i64
}

fn weighted_mode(values: &[(f64, f64)]) -> Option<f64> {
    let mut by_bucket: HashMap<i64, f64> = HashMap::new();
    for (v, w) in values {
        if *w <= 0.0 {
            continue;
        }
        let key = mode_bucket_key(*v);
        *by_bucket.entry(key).or_insert(0.0) += *w;
    }
    if by_bucket.is_empty() {
        return None;
    }
    let mut best_key = 0_i64;
    let mut best_weight = -1.0_f64;
    for (k, w) in by_bucket {
        // Match VB6 tie-break style (`>=`) by choosing the higher bucket on ties.
        if w > best_weight || ((w - best_weight).abs() < 1e-9 && k > best_key) {
            best_key = k;
            best_weight = w;
        }
    }
    Some((best_key as f64) / 10.0)
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
        i64,
    )> = conn
        .query_row(
            "SELECT code, description, full_code, room, day, period, weight_method, calc_method, sort_order
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
                    r.get(8)?,
                ))
            },
        )
        .optional()
        .map_err(|e| CalcError::new("db_query_failed", e.to_string()))?;
    let Some((
        ms_code,
        ms_desc,
        full_code,
        room,
        day,
        period,
        weight_method,
        calc_method,
        mark_set_sort_order,
    )) = mark_set_row
    else {
        return Err(CalcError::new("not_found", "mark set not found"));
    };

    let calc_method_applied = if (0..=4).contains(&calc_method) {
        calc_method
    } else {
        0
    };
    let mut filters_applied = filters.clone();
    if calc_method_applied > 2 {
        // VB6 EvalOne_Calculate forces category filter to [ALL] for blended methods.
        filters_applied.category_name = None;
    }

    let mut students_stmt = conn
        .prepare(
            "SELECT id, last_name, first_name, sort_order, active, COALESCE(mark_set_mask, 'TBA')
             FROM students
             WHERE class_id = ?
             ORDER BY sort_order",
        )
        .map_err(|e| CalcError::new("db_query_failed", e.to_string()))?;
    let students: Vec<SummaryStudent> = students_stmt
        .query_map([class_id], |r| {
            let last: String = r.get(1)?;
            let first: String = r.get(2)?;
            let mask: String = r.get(5)?;
            Ok(SummaryStudent {
                id: r.get(0)?,
                display_name: format!("{}, {}", last, first),
                sort_order: r.get(3)?,
                active: r.get::<_, i64>(4)? != 0,
                mark_set_mask: mask,
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
            let term_ok = filters_applied
                .term
                .map(|t| a.term == Some(t))
                .unwrap_or(true);
            let cat_ok = filters_applied
                .category_name
                .as_ref()
                .map(|cat| {
                    a.category_name
                        .as_deref()
                        .map(|v| v.to_ascii_lowercase() == *cat)
                        .unwrap_or(false)
                })
                .unwrap_or(true);
            let type_ok = matches_types_mask(filters_applied.types_mask, a.legacy_type);
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
            if !is_valid_kid(s.active, &s.mark_set_mask, mark_set_sort_order) {
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

    // CalcMethod parity: if total category weight excluding BONUS is 0, force entry weighting.
    let non_bonus_cat_weight_sum: f64 = categories
        .iter()
        .filter(|c| !c.name.trim().eq_ignore_ascii_case("BONUS"))
        .map(|c| c.weight)
        .sum();

    let mode_cfg = load_mode_config(conn)?;

    let mut per_student: Vec<StudentFinal> = Vec::new();
    let mut per_student_categories: Vec<StudentCategoryBreakdown> = Vec::new();
    let mut per_category_totals: HashMap<String, (f64, usize, i64, f64)> = HashMap::new();
    let mut per_category_assessment_counts: HashMap<String, i64> = HashMap::new();

    for a in &selected_assessments {
        let key = a
            .category_name
            .clone()
            .unwrap_or_else(|| "Uncategorized".to_string());
        *per_category_assessment_counts.entry(key).or_insert(0) += 1;
    }

    let weight_method_setting = weight_method.clamp(0, 2);

    // VB6: if calc method is blended (3/4), force category weighting and ignore category filter.
    // We reflect that in calc computations. (Caller-provided filter value is still returned in
    // `settings`, but `filters` in the response reflects what was actually applied.)
    let ev_wt_meth_for_weights = if calc_method_applied > 2 { 1 } else { weight_method_setting };
    let weight_method_applied = if calc_method_applied > 2 {
        1
    } else if weight_method_setting == 1 && non_bonus_cat_weight_sum == 0.0 {
        0
    } else {
        weight_method_setting
    };
    let wrk_wt_meth = if ev_wt_meth_for_weights == 2 {
        0
    } else if weight_method_applied == 2 {
        0
    } else {
        weight_method_applied
    };

    let mut cat_idx_by_name: HashMap<String, usize> = HashMap::new();
    for (idx, c) in categories.iter().enumerate() {
        cat_idx_by_name.insert(c.name.to_ascii_lowercase(), idx);
    }
    let bonus_cat_idx: Option<usize> = categories
        .iter()
        .position(|c| c.name.trim().eq_ignore_ascii_case("BONUS"));
    let wrk_cat_wt: Vec<f64> = categories
        .iter()
        .map(|c| {
            if ev_wt_meth_for_weights == 2 {
                if c.weight > 0.0 { 1.0 } else { 0.0 }
            } else {
                c.weight
            }
        })
        .collect();

    let mut excluded_by_weight_count = 0usize;
    let mut excluded_by_category_weight_count = 0usize;

    let selected_assessments_for_stats: Vec<SummaryAssessment> = selected_assessments.clone();
    let selected_assessments_for_calc: Vec<SummaryAssessment> = selected_assessments_for_stats
        .iter()
        .filter(|a| {
            // VB6 Okay: exclude if entry weight is 0.
            if a.weight <= 0.0 {
                excluded_by_weight_count += 1;
                return false;
            }

            // If category weighting is applied, exclude entries in categories with weight 0.
            if wrk_wt_meth == 1 {
                let cat = a
                    .category_name
                    .as_deref()
                    .unwrap_or("Uncategorized")
                    .to_ascii_lowercase();
                let cat_weight = category_weight_map.get(&cat).copied().unwrap_or(0.0);
                if cat_weight <= 0.0 {
                    excluded_by_category_weight_count += 1;
                    return false;
                }
            }
            // Parity with VB6: unknown categories are excluded from calculations.
            let cat = a
                .category_name
                .as_deref()
                .unwrap_or("Uncategorized")
                .to_ascii_lowercase();
            if !cat_idx_by_name.contains_key(&cat) {
                return false;
            }
            true
        })
        .cloned()
        .collect();

    // Use calc-assessments for category counts in calculation model.
    per_category_assessment_counts.clear();
    for a in &selected_assessments_for_calc {
        let key = a
            .category_name
            .clone()
            .unwrap_or_else(|| "Uncategorized".to_string());
        *per_category_assessment_counts.entry(key).or_insert(0) += 1;
    }

    for s in &students {
        let valid_kid = is_valid_kid(s.active, &s.mark_set_mask, mark_set_sort_order);

        let mut no_mark_count = 0_i64;
        let mut zero_count = 0_i64;
        let mut scored_count = 0_i64;

        let cat_count = categories.len();
        let mut cat_sum: Vec<f64> = vec![0.0; cat_count];
        let mut cat_wsum: Vec<f64> = vec![0.0; cat_count];
        let mut cat_has_nonzero: Vec<bool> = vec![false; cat_count];
        let mut entries: Vec<StudentEntry> = Vec::new();

        if valid_kid {
            for a in &selected_assessments_for_calc {
                let cat_name = a
                    .category_name
                    .as_deref()
                    .unwrap_or("Uncategorized")
                    .to_ascii_lowercase();
                let Some(&cat_idx) = cat_idx_by_name.get(&cat_name) else {
                    continue;
                };
                let state = score_by_pair
                    .get(&(a.id.clone(), s.id.clone()))
                    .copied()
                    .unwrap_or(ScoreState::NoMark);
                let pct_opt = match state {
                    ScoreState::NoMark => {
                        no_mark_count += 1;
                        None
                    }
                    ScoreState::Zero => {
                        zero_count += 1;
                        Some(0.001)
                    }
                    ScoreState::Scored(v) => {
                        scored_count += 1;
                        if v > 0.0 {
                            cat_has_nonzero[cat_idx] = true;
                        }
                        if a.out_of > 0.0 {
                            Some(100.0 * v / a.out_of)
                        } else {
                            Some(0.0)
                        }
                    }
                };
                let Some(pct) = pct_opt else {
                    continue;
                };
                let entry_wt = if ev_wt_meth_for_weights == 2 { 1.0 } else { a.weight };
                cat_sum[cat_idx] += pct * entry_wt;
                cat_wsum[cat_idx] += entry_wt;
                entries.push(StudentEntry {
                    pct,
                    entry_wt,
                    cat_idx,
                });
            }
        }

        // VB6 EV_CatWT(k,0): overall denominator excludes BONUS.
        let mut total_wt0 = 0.0_f64;
        if valid_kid {
            for cat in 0..cat_count {
                if Some(cat) == bonus_cat_idx {
                    continue;
                }
                if cat_wsum[cat] <= 0.0 {
                    continue;
                }
                if wrk_wt_meth == 1 {
                    total_wt0 += wrk_cat_wt.get(cat).copied().unwrap_or(0.0);
                } else {
                    total_wt0 += cat_wsum[cat];
                }
            }
        }

        let mut cat_avg: Vec<Option<f64>> = vec![None; cat_count];
        if valid_kid {
            for cat in 0..cat_count {
                if cat_wsum[cat] > 0.0 {
                    let mut v = cat_sum[cat] / cat_wsum[cat];
                    if !cat_has_nonzero[cat] {
                        v = 0.001;
                    }
                    cat_avg[cat] = Some(v);
                }
            }
        }

        // Emit per-student category breakdown for UI/debugging.
        per_student_categories.push(StudentCategoryBreakdown {
            student_id: s.id.clone(),
            categories: categories
                .iter()
                .enumerate()
                .map(|(cat_idx, c)| {
                    let has_data = valid_kid && cat_wsum[cat_idx] > 0.0;
                    let value = if has_data {
                        cat_avg[cat_idx].map(|v| {
                            if (v - 0.001).abs() < 1e-9 {
                                0.0
                            } else {
                                round_off_1_decimal(v)
                            }
                        })
                    } else {
                        None
                    };
                    StudentCategoryValue {
                        name: c.name.clone(),
                        value,
                        weight: c.weight,
                        has_data,
                    }
                })
                .collect(),
        });

        let final_mark_raw = if !valid_kid {
            None
        } else if total_wt0 <= 0.0 {
            None
        } else if scored_count == 0 && zero_count == 0 {
            None
        } else if scored_count == 0 && zero_count > 0 {
            Some(0.0)
        } else {
            match calc_method_applied {
                1 => vb6_median_mark(
                    &entries,
                    ev_wt_meth_for_weights,
                    wrk_wt_meth,
                    &cat_wsum,
                    &wrk_cat_wt,
                    total_wt0,
                    None,
                ),
                2 => vb6_mode_mark(
                    &mode_cfg,
                    &entries,
                    wrk_wt_meth,
                    &cat_wsum,
                    &wrk_cat_wt,
                    total_wt0,
                    None,
                ),
                3 | 4 => {
                    // Blended methods: force category weighting and ignore category filter.
                    let mut total = 0.0_f64;
                    if total_wt0 <= 0.0 {
                        None
                    } else {
                        for cat in 0..cat_count {
                            if wrk_cat_wt.get(cat).copied().unwrap_or(0.0) <= 0.0 {
                                continue;
                            }
                            if cat_wsum[cat] <= 0.0 {
                                continue;
                            }
                            let cat_mark = if calc_method_applied == 3 {
                                // VB6 quirk: ModeCats ignores the types mask. We approximate that by
                                // using *all* entries for this term/category, but still using
                                // denominators derived from type-filtered EV_CatWT.
                                let mut entries_modecats: Vec<StudentEntry> = Vec::new();
                                for a in &all_assessments {
                                    // Term filter only.
                                    if let Some(t) = filters_applied.term {
                                        if a.term != Some(t) {
                                            continue;
                                        }
                                    }
                                    if a.weight <= 0.0 {
                                        continue;
                                    }
                                    let cat_name = a
                                        .category_name
                                        .as_deref()
                                        .unwrap_or("Uncategorized")
                                        .to_ascii_lowercase();
                                    let Some(&cat_idx2) = cat_idx_by_name.get(&cat_name) else {
                                        continue;
                                    };
                                    if cat_idx2 != cat {
                                        continue;
                                    }
                                    if wrk_wt_meth == 1 {
                                        let cat_weight = category_weight_map
                                            .get(&cat_name)
                                            .copied()
                                            .unwrap_or(0.0);
                                        if cat_weight <= 0.0 {
                                            continue;
                                        }
                                    }

                                    let state = score_by_pair
                                        .get(&(a.id.clone(), s.id.clone()))
                                        .copied()
                                        .unwrap_or(ScoreState::NoMark);
                                    let pct_opt = match state {
                                        ScoreState::NoMark => None,
                                        ScoreState::Zero => Some(0.001),
                                        ScoreState::Scored(v) => {
                                            if a.out_of > 0.0 {
                                                Some(100.0 * v / a.out_of)
                                            } else {
                                                Some(0.0)
                                            }
                                        }
                                    };
                                    let Some(pct) = pct_opt else { continue };
                                    let entry_wt =
                                        if ev_wt_meth_for_weights == 2 { 1.0 } else { a.weight };
                                    entries_modecats.push(StudentEntry {
                                        pct,
                                        entry_wt,
                                        cat_idx: cat,
                                    });
                                }
                                vb6_mode_mark(
                                    &mode_cfg,
                                    &entries_modecats,
                                    wrk_wt_meth,
                                    &cat_wsum,
                                    &wrk_cat_wt,
                                    total_wt0,
                                    Some(cat),
                                )
                            } else {
                                vb6_median_mark(
                                    &entries,
                                    ev_wt_meth_for_weights,
                                    wrk_wt_meth,
                                    &cat_wsum,
                                    &wrk_cat_wt,
                                    total_wt0,
                                    Some(cat),
                                )
                            };
                            let Some(cat_mark) = cat_mark else {
                                continue;
                            };
                            if cat_mark <= 0.0 {
                                continue;
                            }
                            total += cat_mark * (wrk_cat_wt.get(cat).copied().unwrap_or(0.0) / total_wt0);
                        }
                        Some(total)
                    }
                }
                _ => {
                    // Average with BONUS add-on outside denominator.
                    let mut base = 0.0_f64;
                    for cat in 0..cat_count {
                        if Some(cat) == bonus_cat_idx {
                            continue;
                        }
                        let Some(v) = cat_avg[cat] else { continue };
                        if cat_wsum[cat] <= 0.0 {
                            continue;
                        }
                        if wrk_wt_meth == 1 {
                            base += v * (wrk_cat_wt.get(cat).copied().unwrap_or(0.0) / total_wt0);
                        } else {
                            base += v * (cat_wsum[cat] / total_wt0);
                        }
                    }
                    if let Some(b) = bonus_cat_idx {
                        if let Some(bavg) = cat_avg[b] {
                            // VB6: bonus adds: kid_bonus_avg * bonus_weight / 100.
                            base += bavg * (wrk_cat_wt.get(b).copied().unwrap_or(0.0) / 100.0);
                        }
                    }
                    Some(base)
                }
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

        // Build class-level per-category averages across valid kids for this mark set.
        if valid_kid {
            for (cat_idx, c) in categories.iter().enumerate() {
                let Some(v) = cat_avg[cat_idx] else {
                    continue;
                };
                let weight = category_weight_map
                    .get(&c.name.to_ascii_lowercase())
                    .copied()
                    .unwrap_or(0.0);
                let entry = per_category_totals
                    .entry(c.name.clone())
                    .or_insert((0.0, 0, i64::MAX, weight));
                entry.0 += v;
                entry.1 += 1;
                entry.2 = entry.2.min(c.sort_order);
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

    let selected_category_count = categories_out.len();

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
        filters: filters_applied.clone(),
        categories: categories_out,
        assessments,
        per_assessment,
        per_category,
        per_student,
        settings_applied: Some(SettingsApplied {
            weight_method_applied,
            calc_method_applied,
            roff_applied: mode_cfg.roff,
            mode_active_levels: mode_cfg.active_levels as i64,
            mode_level_vals: mode_cfg.level_vals.clone(),
        }),
        per_student_categories: Some(per_student_categories),
        parity_diagnostics: if cfg!(debug_assertions) {
            Some(ParityDiagnostics {
                calc_method_applied,
                weight_method_applied,
                selected_assessment_count: selected_assessments.len(),
                selected_category_count,
                selected_assessments_for_stats: selected_assessments_for_stats.len(),
                selected_assessments_for_calc: selected_assessments_for_calc.len(),
                excluded_by_weight_count,
                excluded_by_category_weight_count,
            })
        } else {
            None
        },
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

    #[test]
    fn weighted_median_respects_weights() {
        let values = vec![(10.0, 1.0), (20.0, 1.0), (90.0, 8.0)];
        let m = weighted_median(&values).expect("weighted median");
        assert!((m - 90.0).abs() < 1e-9);
    }

    #[test]
    fn weighted_median_averages_when_exactly_half() {
        let values = vec![(50.0, 1.0), (60.0, 1.0)];
        let m = weighted_median(&values).expect("weighted median");
        assert!((m - 55.0).abs() < 1e-9);
    }

    #[test]
    fn weighted_mode_tie_prefers_higher_bucket() {
        let values = vec![(70.0, 1.0), (80.0, 1.0)];
        let m = weighted_mode(&values).expect("weighted mode");
        assert!((m - 80.0).abs() < 1e-9);
    }
}
