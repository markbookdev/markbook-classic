use crate::db;
use crate::ipc::error::{err, ok};
use crate::ipc::types::{AppState, Request};
use chrono::{Duration as ChronoDuration, NaiveDate};
use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension};
use serde_json::{json, Map, Value as JsonValue};
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const ARTIFACT_UNIT: &str = "unit";
const ARTIFACT_LESSON: &str = "lesson";
const ARTIFACT_COURSE_DESCRIPTION: &str = "course_description";
const ARTIFACT_TIME_MANAGEMENT: &str = "time_management";

const STATUS_DRAFT: &str = "draft";
const STATUS_PUBLISHED: &str = "published";
const STATUS_ARCHIVED: &str = "archived";

#[derive(Clone, Debug)]
struct PlannerSetupDefaults {
    default_lesson_duration_minutes: i64,
    default_publish_status: String,
    show_archived_by_default: bool,
    default_unit_title_prefix: String,
}

#[derive(Clone, Debug)]
struct CourseSetupDefaults {
    default_period_minutes: i64,
    default_periods_per_week: i64,
    default_total_weeks: i64,
    include_policy_by_default: bool,
}

fn load_setup_section(conn: &Connection, key: &str) -> Option<Map<String, JsonValue>> {
    db::settings_get_json(conn, key)
        .ok()
        .flatten()
        .and_then(|v| v.as_object().cloned())
}

fn load_planner_setup_defaults(conn: &Connection) -> PlannerSetupDefaults {
    let obj = load_setup_section(conn, "setup.planner").unwrap_or_default();
    let default_lesson_duration_minutes = obj
        .get("defaultLessonDurationMinutes")
        .and_then(|v| v.as_i64())
        .filter(|v| *v > 0)
        .unwrap_or(75);
    let default_publish_status = obj
        .get("defaultPublishStatus")
        .and_then(|v| v.as_str())
        .map(|s| s.to_ascii_lowercase())
        .filter(|s| validate_publish_status(s))
        .unwrap_or_else(|| STATUS_DRAFT.to_string());
    let show_archived_by_default = obj
        .get("showArchivedByDefault")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let default_unit_title_prefix = obj
        .get("defaultUnitTitlePrefix")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Unit".to_string());
    PlannerSetupDefaults {
        default_lesson_duration_minutes,
        default_publish_status,
        show_archived_by_default,
        default_unit_title_prefix,
    }
}

fn load_course_setup_defaults(conn: &Connection) -> CourseSetupDefaults {
    let obj = load_setup_section(conn, "setup.courseDescription").unwrap_or_default();
    let default_period_minutes = obj
        .get("defaultPeriodMinutes")
        .and_then(|v| v.as_i64())
        .filter(|v| *v > 0)
        .unwrap_or(75);
    let default_periods_per_week = obj
        .get("defaultPeriodsPerWeek")
        .and_then(|v| v.as_i64())
        .filter(|v| *v > 0)
        .unwrap_or(5);
    let default_total_weeks = obj
        .get("defaultTotalWeeks")
        .and_then(|v| v.as_i64())
        .filter(|v| *v > 0)
        .unwrap_or(36);
    let include_policy_by_default = obj
        .get("includePolicyByDefault")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    CourseSetupDefaults {
        default_period_minutes,
        default_periods_per_week,
        default_total_weeks,
        include_policy_by_default,
    }
}

fn db_conn<'a>(state: &'a AppState, req: &Request) -> Result<&'a Connection, serde_json::Value> {
    state
        .db
        .as_ref()
        .ok_or_else(|| err(&req.id, "no_workspace", "select a workspace first", None))
}

fn required_str(req: &Request, key: &str) -> Result<String, serde_json::Value> {
    req.params
        .get(key)
        .and_then(|v| v.as_str())
        .map(|v| v.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| err(&req.id, "bad_params", format!("missing {}", key), None))
}

fn now_ts() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().to_string())
        .unwrap_or_else(|_| "0".to_string())
}

fn parse_bool(v: Option<&JsonValue>, default: bool) -> Result<bool, &'static str> {
    match v {
        None => Ok(default),
        Some(v) if v.is_null() => Ok(default),
        Some(v) => v.as_bool().ok_or("must be boolean"),
    }
}

fn parse_opt_string(v: Option<&JsonValue>) -> Result<Option<String>, &'static str> {
    match v {
        None => Ok(None),
        Some(v) if v.is_null() => Ok(None),
        Some(v) => {
            let s = v.as_str().ok_or("must be string or null")?.trim().to_string();
            if s.is_empty() {
                Ok(None)
            } else {
                Ok(Some(s))
            }
        }
    }
}

fn parse_opt_i64(v: Option<&JsonValue>) -> Result<Option<i64>, &'static str> {
    match v {
        None => Ok(None),
        Some(v) if v.is_null() => Ok(None),
        Some(v) => v.as_i64().map(Some).ok_or("must be integer or null"),
    }
}

fn parse_string_array(v: Option<&JsonValue>) -> Result<Vec<String>, &'static str> {
    match v {
        None => Ok(Vec::new()),
        Some(v) if v.is_null() => Ok(Vec::new()),
        Some(v) => {
            let arr = v.as_array().ok_or("must be array of strings")?;
            let mut out = Vec::with_capacity(arr.len());
            for item in arr {
                let s = item
                    .as_str()
                    .ok_or("must be array of strings")?
                    .trim()
                    .to_string();
                if !s.is_empty() {
                    out.push(s);
                }
            }
            Ok(out)
        }
    }
}

fn parse_required_string_array(v: Option<&JsonValue>, key: &str) -> Result<Vec<String>, String> {
    let Some(raw) = v else {
        return Err(format!("missing {}", key));
    };
    let arr = raw
        .as_array()
        .ok_or_else(|| format!("{} must be array of strings", key))?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let s = item
            .as_str()
            .ok_or_else(|| format!("{} must be array of strings", key))?
            .trim()
            .to_string();
        if !s.is_empty() && !out.contains(&s) {
            out.push(s);
        }
    }
    if out.is_empty() {
        return Err(format!("{} must contain at least one lesson id", key));
    }
    Ok(out)
}

fn parse_title_mode(v: Option<&JsonValue>, default: &str) -> Result<String, String> {
    let mode = match v.and_then(|x| x.as_str()) {
        Some(raw) => raw.trim().to_ascii_lowercase(),
        None => default.to_string(),
    };
    if mode != "same" && mode != "appendcopy" {
        return Err("titleMode must be one of: same, appendCopy".to_string());
    }
    Ok(if mode == "appendcopy" {
        "appendCopy".to_string()
    } else {
        "same".to_string()
    })
}

fn shift_iso_date(value: Option<String>, day_offset: i64) -> Option<String> {
    let Some(raw) = value else {
        return None;
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() || day_offset == 0 {
        return if trimmed.is_empty() { None } else { Some(trimmed.to_string()) };
    }
    match NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        Ok(date) => Some(
            (date + ChronoDuration::days(day_offset))
                .format("%Y-%m-%d")
                .to_string(),
        ),
        Err(_) => Some(trimmed.to_string()),
    }
}

fn validate_artifact_kind(kind: &str) -> bool {
    matches!(
        kind,
        ARTIFACT_UNIT | ARTIFACT_LESSON | ARTIFACT_COURSE_DESCRIPTION | ARTIFACT_TIME_MANAGEMENT
    )
}

fn validate_publish_status(status: &str) -> bool {
    matches!(status, STATUS_DRAFT | STATUS_PUBLISHED | STATUS_ARCHIVED)
}

fn ensure_class_exists(conn: &Connection, class_id: &str) -> Result<(), &'static str> {
    let exists = conn
        .query_row(
            "SELECT 1 FROM classes WHERE id = ? LIMIT 1",
            [class_id],
            |_r| Ok(()),
        )
        .optional()
        .map_err(|_| "db_query_failed")?;
    if exists.is_some() {
        Ok(())
    } else {
        Err("not_found")
    }
}

fn json_array_string(values: &[String]) -> String {
    serde_json::to_string(values).unwrap_or_else(|_| "[]".to_string())
}

fn parse_json_array_string(raw: &str) -> Vec<String> {
    serde_json::from_str::<Vec<String>>(raw).unwrap_or_default()
}

fn handle_units_list(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let include_archived = match parse_bool(req.params.get("includeArchived"), false) {
        Ok(v) => v,
        Err(m) => return err(&req.id, "bad_params", format!("includeArchived {}", m), None),
    };
    if let Err(code) = ensure_class_exists(conn, &class_id) {
        return err(
            &req.id,
            code,
            if code == "not_found" {
                "class not found".to_string()
            } else {
                "failed to read class".to_string()
            },
            None,
        );
    }

    let sql = if include_archived {
        "SELECT id, sort_order, title, start_date, end_date, summary, expectations_json, resources_json, archived, created_at, updated_at
         FROM planner_units
         WHERE class_id = ?
         ORDER BY sort_order, id"
    } else {
        "SELECT id, sort_order, title, start_date, end_date, summary, expectations_json, resources_json, archived, created_at, updated_at
         FROM planner_units
         WHERE class_id = ? AND archived = 0
         ORDER BY sort_order, id"
    };
    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let units = match stmt.query_map([&class_id], |r| {
        let expectations_raw: String = r.get(6)?;
        let resources_raw: String = r.get(7)?;
        Ok(json!({
            "id": r.get::<_, String>(0)?,
            "sortOrder": r.get::<_, i64>(1)?,
            "title": r.get::<_, String>(2)?,
            "startDate": r.get::<_, Option<String>>(3)?,
            "endDate": r.get::<_, Option<String>>(4)?,
            "summary": r.get::<_, String>(5)?,
            "expectations": parse_json_array_string(&expectations_raw),
            "resources": parse_json_array_string(&resources_raw),
            "archived": r.get::<_, i64>(8)? != 0,
            "createdAt": r.get::<_, String>(9)?,
            "updatedAt": r.get::<_, String>(10)?,
        }))
    }) {
        Ok(rows) => match rows.collect::<Result<Vec<_>, _>>() {
            Ok(v) => v,
            Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
        },
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    ok(&req.id, json!({ "units": units }))
}

fn handle_units_open(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let unit_id = match required_str(req, "unitId") {
        Ok(v) => v,
        Err(e) => return e,
    };

    let row = conn
        .query_row(
            "SELECT id, sort_order, title, start_date, end_date, summary, expectations_json, resources_json, archived, created_at, updated_at
             FROM planner_units
             WHERE class_id = ? AND id = ?",
            params![class_id, unit_id],
            |r| {
                let expectations_raw: String = r.get(6)?;
                let resources_raw: String = r.get(7)?;
                Ok(json!({
                    "id": r.get::<_, String>(0)?,
                    "sortOrder": r.get::<_, i64>(1)?,
                    "title": r.get::<_, String>(2)?,
                    "startDate": r.get::<_, Option<String>>(3)?,
                    "endDate": r.get::<_, Option<String>>(4)?,
                    "summary": r.get::<_, String>(5)?,
                    "expectations": parse_json_array_string(&expectations_raw),
                    "resources": parse_json_array_string(&resources_raw),
                    "archived": r.get::<_, i64>(8)? != 0,
                    "createdAt": r.get::<_, String>(9)?,
                    "updatedAt": r.get::<_, String>(10)?,
                }))
            },
        )
        .optional();
    match row {
        Ok(Some(unit)) => ok(&req.id, json!({ "unit": unit })),
        Ok(None) => err(&req.id, "not_found", "unit not found", None),
        Err(e) => err(&req.id, "db_query_failed", e.to_string(), None),
    }
}

fn next_sort_order(conn: &Connection, table: &str, class_id: &str, unit_id: Option<&str>) -> Result<i64, String> {
    let sql = if table == "planner_lessons" && unit_id.is_some() {
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM planner_lessons WHERE class_id = ? AND COALESCE(unit_id,'') = COALESCE(?, '')"
    } else if table == "planner_lessons" {
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM planner_lessons WHERE class_id = ?"
    } else {
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM planner_units WHERE class_id = ?"
    };
    let result: i64 = if table == "planner_lessons" && unit_id.is_some() {
        conn.query_row(sql, params![class_id, unit_id], |r| r.get(0))
            .map_err(|e| e.to_string())?
    } else {
        conn.query_row(sql, params![class_id], |r| r.get(0))
            .map_err(|e| e.to_string())?
    };
    Ok(result.max(0))
}

fn handle_units_create(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    if let Err(code) = ensure_class_exists(conn, &class_id) {
        return err(
            &req.id,
            code,
            if code == "not_found" {
                "class not found".to_string()
            } else {
                "failed to read class".to_string()
            },
            None,
        );
    }

    let Some(input) = req.params.get("input").and_then(|v| v.as_object()) else {
        return err(&req.id, "bad_params", "missing input", None);
    };
    let title = match input.get("title").and_then(|v| v.as_str()) {
        Some(v) => v.trim().to_string(),
        None => return err(&req.id, "bad_params", "input.title is required", None),
    };
    if title.is_empty() {
        return err(&req.id, "bad_params", "input.title must not be empty", None);
    }
    let start_date = match parse_opt_string(input.get("startDate")) {
        Ok(v) => v,
        Err(m) => return err(&req.id, "bad_params", format!("input.startDate {}", m), None),
    };
    let end_date = match parse_opt_string(input.get("endDate")) {
        Ok(v) => v,
        Err(m) => return err(&req.id, "bad_params", format!("input.endDate {}", m), None),
    };
    let summary = match parse_opt_string(input.get("summary")) {
        Ok(v) => v.unwrap_or_default(),
        Err(m) => return err(&req.id, "bad_params", format!("input.summary {}", m), None),
    };
    let expectations = match parse_string_array(input.get("expectations")) {
        Ok(v) => v,
        Err(m) => return err(&req.id, "bad_params", format!("input.expectations {}", m), None),
    };
    let resources = match parse_string_array(input.get("resources")) {
        Ok(v) => v,
        Err(m) => return err(&req.id, "bad_params", format!("input.resources {}", m), None),
    };
    let archived = match parse_bool(input.get("archived"), false) {
        Ok(v) => v,
        Err(m) => return err(&req.id, "bad_params", format!("input.archived {}", m), None),
    };
    let sort_order = match parse_opt_i64(input.get("sortOrder")) {
        Ok(Some(v)) if v >= 0 => v,
        Ok(Some(_)) => return err(&req.id, "bad_params", "input.sortOrder must be >= 0", None),
        Ok(None) => match next_sort_order(conn, "planner_units", &class_id, None) {
            Ok(v) => v,
            Err(e) => return err(&req.id, "db_query_failed", e, None),
        },
        Err(m) => return err(&req.id, "bad_params", format!("input.sortOrder {}", m), None),
    };

    let unit_id = Uuid::new_v4().to_string();
    let ts = now_ts();
    if let Err(e) = conn.execute(
        "INSERT INTO planner_units(
            id, class_id, sort_order, title, start_date, end_date, summary, expectations_json, resources_json, archived, created_at, updated_at
         ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            unit_id,
            class_id,
            sort_order,
            title,
            start_date,
            end_date,
            summary,
            json_array_string(&expectations),
            json_array_string(&resources),
            if archived { 1 } else { 0 },
            ts,
            ts
        ],
    ) {
        return err(&req.id, "db_insert_failed", e.to_string(), None);
    }
    ok(&req.id, json!({ "unitId": unit_id }))
}

fn handle_units_update(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let unit_id = match required_str(req, "unitId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let Some(patch) = req.params.get("patch").and_then(|v| v.as_object()) else {
        return err(&req.id, "bad_params", "missing patch", None);
    };

    let exists = match conn
        .query_row(
            "SELECT 1 FROM planner_units WHERE class_id = ? AND id = ?",
            params![class_id, unit_id],
            |_r| Ok(()),
        )
        .optional()
    {
        Ok(v) => v.is_some(),
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    if !exists {
        return err(&req.id, "not_found", "unit not found", None);
    }

    let mut fields: Vec<String> = Vec::new();
    let mut values: Vec<Value> = Vec::new();
    for (k, v) in patch {
        match k.as_str() {
            "title" => {
                let Some(s) = v.as_str() else {
                    return err(&req.id, "bad_params", "patch.title must be string", None);
                };
                let s = s.trim();
                if s.is_empty() {
                    return err(&req.id, "bad_params", "patch.title must not be empty", None);
                }
                fields.push("title = ?".to_string());
                values.push(Value::Text(s.to_string()));
            }
            "startDate" => {
                fields.push("start_date = ?".to_string());
                if v.is_null() {
                    values.push(Value::Null);
                } else if let Some(s) = v.as_str() {
                    values.push(Value::Text(s.trim().to_string()));
                } else {
                    return err(&req.id, "bad_params", "patch.startDate must be string or null", None);
                }
            }
            "endDate" => {
                fields.push("end_date = ?".to_string());
                if v.is_null() {
                    values.push(Value::Null);
                } else if let Some(s) = v.as_str() {
                    values.push(Value::Text(s.trim().to_string()));
                } else {
                    return err(&req.id, "bad_params", "patch.endDate must be string or null", None);
                }
            }
            "summary" => {
                let Some(s) = v.as_str() else {
                    return err(&req.id, "bad_params", "patch.summary must be string", None);
                };
                fields.push("summary = ?".to_string());
                values.push(Value::Text(s.to_string()));
            }
            "expectations" => {
                let list = match parse_string_array(Some(v)) {
                    Ok(v) => v,
                    Err(m) => return err(&req.id, "bad_params", format!("patch.expectations {}", m), None),
                };
                fields.push("expectations_json = ?".to_string());
                values.push(Value::Text(json_array_string(&list)));
            }
            "resources" => {
                let list = match parse_string_array(Some(v)) {
                    Ok(v) => v,
                    Err(m) => return err(&req.id, "bad_params", format!("patch.resources {}", m), None),
                };
                fields.push("resources_json = ?".to_string());
                values.push(Value::Text(json_array_string(&list)));
            }
            "archived" => {
                let Some(b) = v.as_bool() else {
                    return err(&req.id, "bad_params", "patch.archived must be boolean", None);
                };
                fields.push("archived = ?".to_string());
                values.push(Value::Integer(if b { 1 } else { 0 }));
            }
            _ => return err(&req.id, "bad_params", format!("unknown patch field: {}", k), None),
        }
    }
    if fields.is_empty() {
        return ok(&req.id, json!({ "ok": true }));
    }
    fields.push("updated_at = ?".to_string());
    values.push(Value::Text(now_ts()));
    values.push(Value::Text(class_id));
    values.push(Value::Text(unit_id));
    let sql = format!(
        "UPDATE planner_units SET {} WHERE class_id = ? AND id = ?",
        fields.join(", ")
    );
    if let Err(e) = conn.execute(&sql, params_from_iter(values)) {
        return err(&req.id, "db_update_failed", e.to_string(), None);
    }
    ok(&req.id, json!({ "ok": true }))
}

fn handle_units_reorder(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let Some(ids) = req.params.get("unitIds").and_then(|v| v.as_array()) else {
        return err(&req.id, "bad_params", "missing unitIds", None);
    };
    let mut provided: Vec<String> = Vec::new();
    let mut seen = HashSet::new();
    for v in ids {
        let Some(s) = v.as_str() else {
            return err(&req.id, "bad_params", "unitIds must be strings", None);
        };
        let s = s.trim();
        if s.is_empty() {
            return err(&req.id, "bad_params", "unitIds must not contain empty values", None);
        }
        if seen.insert(s.to_string()) {
            provided.push(s.to_string());
        }
    }
    let mut stmt = match conn.prepare(
        "SELECT id FROM planner_units WHERE class_id = ? ORDER BY sort_order, id",
    ) {
        Ok(s) => s,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let existing = match stmt.query_map([&class_id], |r| r.get::<_, String>(0)) {
        Ok(rows) => match rows.collect::<Result<Vec<_>, _>>() {
            Ok(v) => v,
            Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
        },
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let existing_set: HashSet<String> = existing.iter().cloned().collect();
    for id in &provided {
        if !existing_set.contains(id) {
            return err(
                &req.id,
                "bad_params",
                format!("unit id not found for class: {}", id),
                None,
            );
        }
    }
    let mut final_order = provided;
    for id in existing {
        if !final_order.contains(&id) {
            final_order.push(id);
        }
    }
    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return err(&req.id, "db_tx_failed", e.to_string(), None),
    };
    let ts = now_ts();
    for (idx, id) in final_order.iter().enumerate() {
        if let Err(e) = tx.execute(
            "UPDATE planner_units SET sort_order = ?, updated_at = ? WHERE class_id = ? AND id = ?",
            params![idx as i64, ts, class_id, id],
        ) {
            let _ = tx.rollback();
            return err(&req.id, "db_update_failed", e.to_string(), None);
        }
    }
    if let Err(e) = tx.commit() {
        return err(&req.id, "db_commit_failed", e.to_string(), None);
    }
    ok(&req.id, json!({ "ok": true }))
}

fn handle_units_archive(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let unit_id = match required_str(req, "unitId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let archived = match parse_bool(req.params.get("archived"), true) {
        Ok(v) => v,
        Err(m) => return err(&req.id, "bad_params", format!("archived {}", m), None),
    };
    match conn.execute(
        "UPDATE planner_units SET archived = ?, updated_at = ? WHERE class_id = ? AND id = ?",
        params![if archived { 1 } else { 0 }, now_ts(), class_id, unit_id],
    ) {
        Ok(0) => err(&req.id, "not_found", "unit not found", None),
        Ok(_) => ok(&req.id, json!({ "ok": true })),
        Err(e) => err(&req.id, "db_update_failed", e.to_string(), None),
    }
}

fn handle_units_clone(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let unit_id = match required_str(req, "unitId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let title_mode = match parse_title_mode(req.params.get("titleMode"), "appendCopy") {
        Ok(v) => v,
        Err(m) => return err(&req.id, "bad_params", m, None),
    };

    let source = match conn
        .query_row(
            "SELECT title, start_date, end_date, summary, expectations_json, resources_json
             FROM planner_units
             WHERE class_id = ? AND id = ?",
            params![class_id, unit_id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, String>(4)?,
                    r.get::<_, String>(5)?,
                ))
            },
        )
        .optional()
    {
        Ok(Some(row)) => row,
        Ok(None) => return err(&req.id, "not_found", "unit not found", None),
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    let cloned_title = if title_mode == "appendCopy" {
        format!("{} (Copy)", source.0.trim())
    } else {
        source.0.clone()
    };
    let sort_order = match next_sort_order(conn, "planner_units", &class_id, None) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e, None),
    };
    let ts = now_ts();
    let cloned_unit_id = Uuid::new_v4().to_string();
    let mut lesson_stmt = match conn.prepare(
        "SELECT lesson_date, title, outline, detail, follow_up, homework, duration_minutes, archived
         FROM planner_lessons
         WHERE class_id = ? AND unit_id = ?
         ORDER BY sort_order, id",
    ) {
        Ok(s) => s,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let lesson_rows = match lesson_stmt.query_map(params![class_id, unit_id], |r| {
        Ok((
            r.get::<_, Option<String>>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
            r.get::<_, String>(4)?,
            r.get::<_, String>(5)?,
            r.get::<_, Option<i64>>(6)?,
            r.get::<_, i64>(7)?,
        ))
    }) {
        Ok(rows) => match rows.collect::<Result<Vec<_>, _>>() {
            Ok(v) => v,
            Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
        },
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    drop(lesson_stmt);

    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return err(&req.id, "db_tx_failed", e.to_string(), None),
    };

    if let Err(e) = tx.execute(
        "INSERT INTO planner_units(
            id, class_id, sort_order, title, start_date, end_date, summary, expectations_json, resources_json, archived, created_at, updated_at
         ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, ?)",
        params![
            cloned_unit_id,
            class_id,
            sort_order,
            cloned_title,
            source.1,
            source.2,
            source.3,
            source.4,
            source.5,
            ts,
            ts
        ],
    ) {
        let _ = tx.rollback();
        return err(&req.id, "db_insert_failed", e.to_string(), None);
    }

    for (idx, lesson) in lesson_rows.iter().enumerate() {
        let cloned_lesson_id = Uuid::new_v4().to_string();
        if let Err(e) = tx.execute(
            "INSERT INTO planner_lessons(
                id, class_id, unit_id, sort_order, lesson_date, title, outline, detail, follow_up, homework, duration_minutes, archived, created_at, updated_at
             ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                cloned_lesson_id,
                class_id,
                cloned_unit_id,
                idx as i64,
                lesson.0,
                lesson.1,
                lesson.2,
                lesson.3,
                lesson.4,
                lesson.5,
                lesson.6,
                lesson.7,
                ts,
                ts
            ],
        ) {
            let _ = tx.rollback();
            return err(&req.id, "db_insert_failed", e.to_string(), None);
        }
    }

    if let Err(e) = tx.commit() {
        return err(&req.id, "db_commit_failed", e.to_string(), None);
    }
    ok(&req.id, json!({ "ok": true, "unitId": cloned_unit_id }))
}

fn lesson_to_json(row: &rusqlite::Row<'_>) -> rusqlite::Result<JsonValue> {
    Ok(json!({
        "id": row.get::<_, String>(0)?,
        "unitId": row.get::<_, Option<String>>(1)?,
        "sortOrder": row.get::<_, i64>(2)?,
        "lessonDate": row.get::<_, Option<String>>(3)?,
        "title": row.get::<_, String>(4)?,
        "outline": row.get::<_, String>(5)?,
        "detail": row.get::<_, String>(6)?,
        "followUp": row.get::<_, String>(7)?,
        "homework": row.get::<_, String>(8)?,
        "durationMinutes": row.get::<_, Option<i64>>(9)?,
        "archived": row.get::<_, i64>(10)? != 0,
        "createdAt": row.get::<_, String>(11)?,
        "updatedAt": row.get::<_, String>(12)?,
    }))
}

fn handle_lessons_list(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let include_archived = match parse_bool(req.params.get("includeArchived"), false) {
        Ok(v) => v,
        Err(m) => return err(&req.id, "bad_params", format!("includeArchived {}", m), None),
    };
    let unit_id = match parse_opt_string(req.params.get("unitId")) {
        Ok(v) => v,
        Err(m) => return err(&req.id, "bad_params", format!("unitId {}", m), None),
    };

    let mut where_clause = String::from("class_id = ?");
    let mut values: Vec<Value> = vec![Value::Text(class_id)];
    if let Some(unit_id) = unit_id {
        where_clause.push_str(" AND unit_id = ?");
        values.push(Value::Text(unit_id));
    }
    if !include_archived {
        where_clause.push_str(" AND archived = 0");
    }
    let sql = format!(
        "SELECT id, unit_id, sort_order, lesson_date, title, outline, detail, follow_up, homework, duration_minutes, archived, created_at, updated_at
         FROM planner_lessons
         WHERE {}
         ORDER BY sort_order, id",
        where_clause
    );
    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let lessons = match stmt.query_map(params_from_iter(values), lesson_to_json) {
        Ok(rows) => match rows.collect::<Result<Vec<_>, _>>() {
            Ok(v) => v,
            Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
        },
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    ok(&req.id, json!({ "lessons": lessons }))
}

fn handle_lessons_open(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let lesson_id = match required_str(req, "lessonId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    match conn
        .query_row(
            "SELECT id, unit_id, sort_order, lesson_date, title, outline, detail, follow_up, homework, duration_minutes, archived, created_at, updated_at
             FROM planner_lessons
             WHERE class_id = ? AND id = ?",
            params![class_id, lesson_id],
            lesson_to_json,
        )
        .optional()
    {
        Ok(Some(lesson)) => ok(&req.id, json!({ "lesson": lesson })),
        Ok(None) => err(&req.id, "not_found", "lesson not found", None),
        Err(e) => err(&req.id, "db_query_failed", e.to_string(), None),
    }
}

fn handle_lessons_create(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let planner_defaults = load_planner_setup_defaults(conn);
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    if let Err(code) = ensure_class_exists(conn, &class_id) {
        return err(
            &req.id,
            code,
            if code == "not_found" {
                "class not found".to_string()
            } else {
                "failed to read class".to_string()
            },
            None,
        );
    }
    let Some(input) = req.params.get("input").and_then(|v| v.as_object()) else {
        return err(&req.id, "bad_params", "missing input", None);
    };
    let title = match input.get("title").and_then(|v| v.as_str()) {
        Some(v) => v.trim().to_string(),
        None => return err(&req.id, "bad_params", "input.title is required", None),
    };
    if title.is_empty() {
        return err(&req.id, "bad_params", "input.title must not be empty", None);
    }
    let unit_id = match parse_opt_string(input.get("unitId")) {
        Ok(v) => v,
        Err(m) => return err(&req.id, "bad_params", format!("input.unitId {}", m), None),
    };
    if let Some(ref uid) = unit_id {
        let exists = match conn
            .query_row(
                "SELECT 1 FROM planner_units WHERE class_id = ? AND id = ?",
                params![class_id, uid],
                |_r| Ok(()),
            )
            .optional()
        {
            Ok(v) => v.is_some(),
            Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
        };
        if !exists {
            return err(&req.id, "not_found", "unit not found", None);
        }
    }
    let lesson_date = match parse_opt_string(input.get("lessonDate")) {
        Ok(v) => v,
        Err(m) => return err(&req.id, "bad_params", format!("input.lessonDate {}", m), None),
    };
    let outline = match parse_opt_string(input.get("outline")) {
        Ok(v) => v.unwrap_or_default(),
        Err(m) => return err(&req.id, "bad_params", format!("input.outline {}", m), None),
    };
    let detail = match parse_opt_string(input.get("detail")) {
        Ok(v) => v.unwrap_or_default(),
        Err(m) => return err(&req.id, "bad_params", format!("input.detail {}", m), None),
    };
    let follow_up = match parse_opt_string(input.get("followUp")) {
        Ok(v) => v.unwrap_or_default(),
        Err(m) => return err(&req.id, "bad_params", format!("input.followUp {}", m), None),
    };
    let homework = match parse_opt_string(input.get("homework")) {
        Ok(v) => v.unwrap_or_default(),
        Err(m) => return err(&req.id, "bad_params", format!("input.homework {}", m), None),
    };
    let duration_minutes = match parse_opt_i64(input.get("durationMinutes")) {
        Ok(Some(v)) if v > 0 => Some(v),
        Ok(Some(_)) => return err(&req.id, "bad_params", "input.durationMinutes must be > 0", None),
        Ok(None) => Some(planner_defaults.default_lesson_duration_minutes),
        Err(m) => return err(&req.id, "bad_params", format!("input.durationMinutes {}", m), None),
    };
    let archived = match parse_bool(input.get("archived"), false) {
        Ok(v) => v,
        Err(m) => return err(&req.id, "bad_params", format!("input.archived {}", m), None),
    };
    let sort_order = match parse_opt_i64(input.get("sortOrder")) {
        Ok(Some(v)) if v >= 0 => v,
        Ok(Some(_)) => return err(&req.id, "bad_params", "input.sortOrder must be >= 0", None),
        Ok(None) => match next_sort_order(conn, "planner_lessons", &class_id, unit_id.as_deref()) {
            Ok(v) => v,
            Err(e) => return err(&req.id, "db_query_failed", e, None),
        },
        Err(m) => return err(&req.id, "bad_params", format!("input.sortOrder {}", m), None),
    };

    let lesson_id = Uuid::new_v4().to_string();
    let ts = now_ts();
    if let Err(e) = conn.execute(
        "INSERT INTO planner_lessons(
            id, class_id, unit_id, sort_order, lesson_date, title, outline, detail, follow_up, homework, duration_minutes, archived, created_at, updated_at
         ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            lesson_id,
            class_id,
            unit_id,
            sort_order,
            lesson_date,
            title,
            outline,
            detail,
            follow_up,
            homework,
            duration_minutes,
            if archived { 1 } else { 0 },
            ts,
            ts
        ],
    ) {
        return err(&req.id, "db_insert_failed", e.to_string(), None);
    }
    ok(&req.id, json!({ "lessonId": lesson_id }))
}

fn handle_lessons_update(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let lesson_id = match required_str(req, "lessonId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let Some(patch) = req.params.get("patch").and_then(|v| v.as_object()) else {
        return err(&req.id, "bad_params", "missing patch", None);
    };

    let exists = match conn
        .query_row(
            "SELECT 1 FROM planner_lessons WHERE class_id = ? AND id = ?",
            params![class_id, lesson_id],
            |_r| Ok(()),
        )
        .optional()
    {
        Ok(v) => v.is_some(),
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    if !exists {
        return err(&req.id, "not_found", "lesson not found", None);
    }

    let mut fields: Vec<String> = Vec::new();
    let mut values: Vec<Value> = Vec::new();
    for (k, v) in patch {
        match k.as_str() {
            "unitId" => {
                fields.push("unit_id = ?".to_string());
                if v.is_null() {
                    values.push(Value::Null);
                } else if let Some(uid) = v.as_str() {
                    let uid = uid.trim().to_string();
                    let unit_exists = match conn
                        .query_row(
                            "SELECT 1 FROM planner_units WHERE class_id = ? AND id = ?",
                            params![class_id, uid],
                            |_r| Ok(()),
                        )
                        .optional()
                    {
                        Ok(v) => v.is_some(),
                        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
                    };
                    if !unit_exists {
                        return err(&req.id, "not_found", "unit not found", None);
                    }
                    values.push(Value::Text(uid));
                } else {
                    return err(&req.id, "bad_params", "patch.unitId must be string or null", None);
                }
            }
            "lessonDate" => {
                fields.push("lesson_date = ?".to_string());
                if v.is_null() {
                    values.push(Value::Null);
                } else if let Some(s) = v.as_str() {
                    values.push(Value::Text(s.trim().to_string()));
                } else {
                    return err(&req.id, "bad_params", "patch.lessonDate must be string or null", None);
                }
            }
            "title" => {
                let Some(s) = v.as_str() else {
                    return err(&req.id, "bad_params", "patch.title must be string", None);
                };
                let s = s.trim();
                if s.is_empty() {
                    return err(&req.id, "bad_params", "patch.title must not be empty", None);
                }
                fields.push("title = ?".to_string());
                values.push(Value::Text(s.to_string()));
            }
            "outline" => {
                let Some(s) = v.as_str() else {
                    return err(&req.id, "bad_params", "patch.outline must be string", None);
                };
                fields.push("outline = ?".to_string());
                values.push(Value::Text(s.to_string()));
            }
            "detail" => {
                let Some(s) = v.as_str() else {
                    return err(&req.id, "bad_params", "patch.detail must be string", None);
                };
                fields.push("detail = ?".to_string());
                values.push(Value::Text(s.to_string()));
            }
            "followUp" => {
                let Some(s) = v.as_str() else {
                    return err(&req.id, "bad_params", "patch.followUp must be string", None);
                };
                fields.push("follow_up = ?".to_string());
                values.push(Value::Text(s.to_string()));
            }
            "homework" => {
                let Some(s) = v.as_str() else {
                    return err(&req.id, "bad_params", "patch.homework must be string", None);
                };
                fields.push("homework = ?".to_string());
                values.push(Value::Text(s.to_string()));
            }
            "durationMinutes" => {
                fields.push("duration_minutes = ?".to_string());
                if v.is_null() {
                    values.push(Value::Null);
                } else if let Some(n) = v.as_i64() {
                    if n <= 0 {
                        return err(&req.id, "bad_params", "patch.durationMinutes must be > 0", None);
                    }
                    values.push(Value::Integer(n));
                } else {
                    return err(
                        &req.id,
                        "bad_params",
                        "patch.durationMinutes must be integer or null",
                        None,
                    );
                }
            }
            "archived" => {
                let Some(b) = v.as_bool() else {
                    return err(&req.id, "bad_params", "patch.archived must be boolean", None);
                };
                fields.push("archived = ?".to_string());
                values.push(Value::Integer(if b { 1 } else { 0 }));
            }
            _ => return err(&req.id, "bad_params", format!("unknown patch field: {}", k), None),
        }
    }
    if fields.is_empty() {
        return ok(&req.id, json!({ "ok": true }));
    }
    fields.push("updated_at = ?".to_string());
    values.push(Value::Text(now_ts()));
    values.push(Value::Text(class_id));
    values.push(Value::Text(lesson_id));
    let sql = format!(
        "UPDATE planner_lessons SET {} WHERE class_id = ? AND id = ?",
        fields.join(", ")
    );
    if let Err(e) = conn.execute(&sql, params_from_iter(values)) {
        return err(&req.id, "db_update_failed", e.to_string(), None);
    }
    ok(&req.id, json!({ "ok": true }))
}

fn handle_lessons_reorder(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let unit_id = match parse_opt_string(req.params.get("unitId")) {
        Ok(v) => v,
        Err(m) => return err(&req.id, "bad_params", format!("unitId {}", m), None),
    };
    let Some(ids) = req.params.get("lessonIdOrder").and_then(|v| v.as_array()) else {
        return err(&req.id, "bad_params", "missing lessonIdOrder", None);
    };
    let mut provided: Vec<String> = Vec::new();
    let mut seen = HashSet::new();
    for v in ids {
        let Some(s) = v.as_str() else {
            return err(&req.id, "bad_params", "lessonIdOrder must be strings", None);
        };
        let s = s.trim();
        if s.is_empty() {
            return err(
                &req.id,
                "bad_params",
                "lessonIdOrder must not contain empty values",
                None,
            );
        }
        if seen.insert(s.to_string()) {
            provided.push(s.to_string());
        }
    }

    let mut where_clause = String::from("class_id = ?");
    let mut values: Vec<Value> = vec![Value::Text(class_id.clone())];
    if let Some(ref unit_id) = unit_id {
        where_clause.push_str(" AND unit_id = ?");
        values.push(Value::Text(unit_id.clone()));
    }
    let sql = format!(
        "SELECT id FROM planner_lessons WHERE {} ORDER BY sort_order, id",
        where_clause
    );
    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let existing = match stmt.query_map(params_from_iter(values), |r| r.get::<_, String>(0)) {
        Ok(rows) => match rows.collect::<Result<Vec<_>, _>>() {
            Ok(v) => v,
            Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
        },
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let existing_set: HashSet<String> = existing.iter().cloned().collect();
    for id in &provided {
        if !existing_set.contains(id) {
            return err(
                &req.id,
                "bad_params",
                format!("lesson id not found for scope: {}", id),
                None,
            );
        }
    }
    let mut final_order = provided;
    for id in existing {
        if !final_order.contains(&id) {
            final_order.push(id);
        }
    }
    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return err(&req.id, "db_tx_failed", e.to_string(), None),
    };
    let ts = now_ts();
    for (idx, id) in final_order.iter().enumerate() {
        if let Err(e) = tx.execute(
            "UPDATE planner_lessons SET sort_order = ?, updated_at = ? WHERE class_id = ? AND id = ?",
            params![idx as i64, ts, class_id, id],
        ) {
            let _ = tx.rollback();
            return err(&req.id, "db_update_failed", e.to_string(), None);
        }
    }
    if let Err(e) = tx.commit() {
        return err(&req.id, "db_commit_failed", e.to_string(), None);
    }
    ok(&req.id, json!({ "ok": true }))
}

fn handle_lessons_archive(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let lesson_id = match required_str(req, "lessonId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let archived = match parse_bool(req.params.get("archived"), true) {
        Ok(v) => v,
        Err(m) => return err(&req.id, "bad_params", format!("archived {}", m), None),
    };
    match conn.execute(
        "UPDATE planner_lessons SET archived = ?, updated_at = ? WHERE class_id = ? AND id = ?",
        params![if archived { 1 } else { 0 }, now_ts(), class_id, lesson_id],
    ) {
        Ok(0) => err(&req.id, "not_found", "lesson not found", None),
        Ok(_) => ok(&req.id, json!({ "ok": true })),
        Err(e) => err(&req.id, "db_update_failed", e.to_string(), None),
    }
}

fn handle_lessons_copy_forward(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let lesson_ids = match parse_required_string_array(req.params.get("lessonIds"), "lessonIds") {
        Ok(v) => v,
        Err(m) => return err(&req.id, "bad_params", m, None),
    };
    let day_offset = match req.params.get("dayOffset").and_then(|v| v.as_i64()) {
        Some(v) if (-3650..=3650).contains(&v) => v,
        Some(_) => {
            return err(
                &req.id,
                "bad_params",
                "dayOffset must be in -3650..=3650",
                None,
            )
        }
        None => return err(&req.id, "bad_params", "missing dayOffset", None),
    };
    let include_follow_up = match parse_bool(req.params.get("includeFollowUp"), true) {
        Ok(v) => v,
        Err(m) => return err(&req.id, "bad_params", format!("includeFollowUp {}", m), None),
    };
    let include_homework = match parse_bool(req.params.get("includeHomework"), true) {
        Ok(v) => v,
        Err(m) => return err(&req.id, "bad_params", format!("includeHomework {}", m), None),
    };

    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return err(&req.id, "db_tx_failed", e.to_string(), None),
    };
    let mut created_lesson_ids: Vec<String> = Vec::new();
    let ts = now_ts();
    for lesson_id in &lesson_ids {
        let source = match tx
            .query_row(
                "SELECT unit_id, sort_order, lesson_date, title, outline, detail, follow_up, homework, duration_minutes, archived
                 FROM planner_lessons
                 WHERE class_id = ? AND id = ?",
                params![class_id, lesson_id],
                |r| {
                    Ok((
                        r.get::<_, Option<String>>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, Option<String>>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, String>(5)?,
                        r.get::<_, String>(6)?,
                        r.get::<_, String>(7)?,
                        r.get::<_, Option<i64>>(8)?,
                        r.get::<_, i64>(9)?,
                    ))
                },
            )
            .optional()
        {
            Ok(Some(row)) => row,
            Ok(None) => continue,
            Err(e) => {
                let _ = tx.rollback();
                return err(&req.id, "db_query_failed", e.to_string(), None);
            }
        };

        let next_sort = match next_sort_order(&tx, "planner_lessons", &class_id, source.0.as_deref())
        {
            Ok(v) => v,
            Err(e) => {
                let _ = tx.rollback();
                return err(&req.id, "db_query_failed", e, None);
            }
        };
        let copied_id = Uuid::new_v4().to_string();
        let shifted_date = shift_iso_date(source.2.clone(), day_offset);
        if let Err(e) = tx.execute(
            "INSERT INTO planner_lessons(
                id, class_id, unit_id, sort_order, lesson_date, title, outline, detail, follow_up, homework, duration_minutes, archived, created_at, updated_at
             ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                copied_id,
                class_id,
                source.0,
                next_sort,
                shifted_date,
                source.3,
                source.4,
                source.5,
                if include_follow_up { source.6 } else { String::new() },
                if include_homework { source.7 } else { String::new() },
                source.8,
                source.9,
                ts,
                ts
            ],
        ) {
            let _ = tx.rollback();
            return err(&req.id, "db_insert_failed", e.to_string(), None);
        }
        created_lesson_ids.push(copied_id);
    }

    if let Err(e) = tx.commit() {
        return err(&req.id, "db_commit_failed", e.to_string(), None);
    }
    ok(
        &req.id,
        json!({ "ok": true, "createdLessonIds": created_lesson_ids }),
    )
}

fn handle_lessons_bulk_assign_unit(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let lesson_ids = match parse_required_string_array(req.params.get("lessonIds"), "lessonIds") {
        Ok(v) => v,
        Err(m) => return err(&req.id, "bad_params", m, None),
    };
    let unit_id = match parse_opt_string(req.params.get("unitId")) {
        Ok(v) => v,
        Err(m) => return err(&req.id, "bad_params", format!("unitId {}", m), None),
    };
    if let Some(ref uid) = unit_id {
        let exists = match conn
            .query_row(
                "SELECT 1 FROM planner_units WHERE class_id = ? AND id = ?",
                params![class_id, uid],
                |_r| Ok(()),
            )
            .optional()
        {
            Ok(v) => v.is_some(),
            Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
        };
        if !exists {
            return err(&req.id, "not_found", "unit not found", None);
        }
    }

    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return err(&req.id, "db_tx_failed", e.to_string(), None),
    };
    let mut updated = 0usize;
    let ts = now_ts();
    for lesson_id in lesson_ids {
        match tx.execute(
            "UPDATE planner_lessons SET unit_id = ?, updated_at = ? WHERE class_id = ? AND id = ?",
            params![unit_id.as_deref(), ts, class_id, lesson_id],
        ) {
            Ok(count) => updated += count as usize,
            Err(e) => {
                let _ = tx.rollback();
                return err(&req.id, "db_update_failed", e.to_string(), None);
            }
        }
    }
    if let Err(e) = tx.commit() {
        return err(&req.id, "db_commit_failed", e.to_string(), None);
    }
    ok(&req.id, json!({ "ok": true, "updated": updated }))
}

fn load_profile(
    conn: &Connection,
    class_id: &str,
    defaults: &CourseSetupDefaults,
) -> Result<JsonValue, String> {
    let row = conn
        .query_row(
            "SELECT course_title, grade_label, period_minutes, periods_per_week, total_weeks, strands_json, policy_text, updated_at
             FROM course_description_profiles
             WHERE class_id = ?",
            [class_id],
            |r| {
                let strands_json: String = r.get(5)?;
                Ok(json!({
                    "courseTitle": r.get::<_, String>(0)?,
                    "gradeLabel": r.get::<_, String>(1)?,
                    "periodMinutes": r.get::<_, i64>(2)?,
                    "periodsPerWeek": r.get::<_, i64>(3)?,
                    "totalWeeks": r.get::<_, i64>(4)?,
                    "strands": parse_json_array_string(&strands_json),
                    "policyText": r.get::<_, String>(6)?,
                    "updatedAt": r.get::<_, String>(7)?,
                }))
            },
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(row.unwrap_or_else(|| {
        json!({
            "courseTitle": "",
            "gradeLabel": "",
            "periodMinutes": defaults.default_period_minutes,
            "periodsPerWeek": defaults.default_periods_per_week,
            "totalWeeks": defaults.default_total_weeks,
            "strands": [],
            "policyText": "",
            "updatedAt": null,
        })
    }))
}

fn resolve_course_description_options(
    profile: &JsonValue,
    defaults: &CourseSetupDefaults,
    options: Option<&Map<String, JsonValue>>,
) -> Result<(i64, i64, i64, bool, bool, bool, bool, bool, JsonValue), String> {
    let mut period_minutes = profile
        .get("periodMinutes")
        .and_then(|v| v.as_i64())
        .filter(|v| *v > 0)
        .unwrap_or(defaults.default_period_minutes);
    let mut periods_per_week = profile
        .get("periodsPerWeek")
        .and_then(|v| v.as_i64())
        .filter(|v| *v > 0)
        .unwrap_or(defaults.default_periods_per_week);
    let mut total_weeks = profile
        .get("totalWeeks")
        .and_then(|v| v.as_i64())
        .filter(|v| *v > 0)
        .unwrap_or(defaults.default_total_weeks);
    let mut include_policy = defaults.include_policy_by_default;
    let mut include_strands = true;
    let mut include_assessment_plan = true;
    let mut include_resources = true;
    let mut include_archived = false;
    let mut period_minutes_source = "profile";
    let mut periods_per_week_source = "profile";
    let mut total_weeks_source = "profile";
    let mut include_policy_source = "setupDefault";
    if profile.get("periodMinutes").and_then(|v| v.as_i64()).unwrap_or(0) <= 0 {
        period_minutes_source = "setupDefault";
    }
    if profile.get("periodsPerWeek").and_then(|v| v.as_i64()).unwrap_or(0) <= 0 {
        periods_per_week_source = "setupDefault";
    }
    if profile.get("totalWeeks").and_then(|v| v.as_i64()).unwrap_or(0) <= 0 {
        total_weeks_source = "setupDefault";
    }
    if let Some(opts) = options {
        if let Some(v) = opts.get("periodMinutes") {
            period_minutes = v
                .as_i64()
                .ok_or_else(|| "options.periodMinutes must be integer".to_string())?;
            period_minutes_source = "options";
        }
        if let Some(v) = opts.get("periodsPerWeek") {
            periods_per_week = v
                .as_i64()
                .ok_or_else(|| "options.periodsPerWeek must be integer".to_string())?;
            periods_per_week_source = "options";
        }
        if let Some(v) = opts.get("totalWeeks") {
            total_weeks = v
                .as_i64()
                .ok_or_else(|| "options.totalWeeks must be integer".to_string())?;
            total_weeks_source = "options";
        }
        if let Some(v) = opts.get("includePolicy") {
            include_policy = v
                .as_bool()
                .ok_or_else(|| "options.includePolicy must be boolean".to_string())?;
            include_policy_source = "options";
        }
        if let Some(v) = opts.get("includeStrands") {
            include_strands = v
                .as_bool()
                .ok_or_else(|| "options.includeStrands must be boolean".to_string())?;
        }
        if let Some(v) = opts.get("includeAssessmentPlan") {
            include_assessment_plan = v
                .as_bool()
                .ok_or_else(|| "options.includeAssessmentPlan must be boolean".to_string())?;
        }
        if let Some(v) = opts.get("includeResources") {
            include_resources = v
                .as_bool()
                .ok_or_else(|| "options.includeResources must be boolean".to_string())?;
        }
        if let Some(v) = opts.get("includeArchived") {
            include_archived = v
                .as_bool()
                .ok_or_else(|| "options.includeArchived must be boolean".to_string())?;
        }
    }
    if period_minutes <= 0 || periods_per_week <= 0 || total_weeks <= 0 {
        return Err("periodMinutes, periodsPerWeek, totalWeeks must all be > 0".to_string());
    }
    let settings_applied = json!({
        "courseDescriptionDefaults": {
            "defaultPeriodMinutes": defaults.default_period_minutes,
            "defaultPeriodsPerWeek": defaults.default_periods_per_week,
            "defaultTotalWeeks": defaults.default_total_weeks,
            "includePolicyByDefault": defaults.include_policy_by_default
        },
        "sources": {
            "periodMinutes": period_minutes_source,
            "periodsPerWeek": periods_per_week_source,
            "totalWeeks": total_weeks_source,
            "includePolicy": include_policy_source
        },
        "resolved": {
            "periodMinutes": period_minutes,
            "periodsPerWeek": periods_per_week,
            "totalWeeks": total_weeks,
            "includePolicy": include_policy,
            "includeStrands": include_strands,
            "includeAssessmentPlan": include_assessment_plan,
            "includeResources": include_resources,
            "includeArchived": include_archived
        }
    });
    Ok((
        period_minutes,
        periods_per_week,
        total_weeks,
        include_policy,
        include_strands,
        include_assessment_plan,
        include_resources,
        include_archived,
        settings_applied,
    ))
}

fn generate_course_description_model(
    conn: &Connection,
    class_id: &str,
    options: Option<&Map<String, JsonValue>>,
) -> Result<JsonValue, String> {
    let class_name: String = conn
        .query_row("SELECT name FROM classes WHERE id = ?", [class_id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let setup_defaults = load_course_setup_defaults(conn);
    let profile = load_profile(conn, class_id, &setup_defaults)?;
    let (
        period_minutes,
        periods_per_week,
        total_weeks,
        include_policy,
        include_strands,
        include_assessment_plan,
        include_resources,
        include_archived,
        settings_applied,
    ) = resolve_course_description_options(&profile, &setup_defaults, options)?;

    let mut unit_stmt = conn
        .prepare(
            "SELECT id, title, start_date, end_date, summary, resources_json
             FROM planner_units
             WHERE class_id = ? AND (? OR archived = 0)
             ORDER BY sort_order, id",
        )
        .map_err(|e| e.to_string())?;
    let units = unit_stmt
        .query_map(params![class_id, include_archived], |r| {
            let resources_raw: String = r.get(5)?;
            Ok(json!({
                "unitId": r.get::<_, String>(0)?,
                "title": r.get::<_, String>(1)?,
                "startDate": r.get::<_, Option<String>>(2)?,
                "endDate": r.get::<_, Option<String>>(3)?,
                "summary": r.get::<_, String>(4)?,
                "resources": parse_json_array_string(&resources_raw),
            }))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| e.to_string())?;

    let mut lesson_stmt = conn
        .prepare(
            "SELECT id, unit_id, title, lesson_date, duration_minutes
             FROM planner_lessons
             WHERE class_id = ? AND (? OR archived = 0)
             ORDER BY sort_order, id",
        )
        .map_err(|e| e.to_string())?;
    let lessons = lesson_stmt
        .query_map(params![class_id, include_archived], |r| {
            Ok(json!({
                "lessonId": r.get::<_, String>(0)?,
                "unitId": r.get::<_, Option<String>>(1)?,
                "title": r.get::<_, String>(2)?,
                "lessonDate": r.get::<_, Option<String>>(3)?,
                "durationMinutes": r.get::<_, Option<i64>>(4)?,
            }))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| e.to_string())?;

    let total_hours = ((period_minutes * periods_per_week * total_weeks) as f64) / 60.0;
    let course_title = profile
        .get("courseTitle")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let grade_label = profile
        .get("gradeLabel")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let strands = if include_strands {
        profile
            .get("strands")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    } else {
        Vec::new()
    };
    let policy_text = if include_policy {
        profile
            .get("policyText")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    } else {
        String::new()
    };
    let resources = if include_resources {
        let mut out: Vec<String> = Vec::new();
        for unit in &units {
            if let Some(list) = unit.get("resources").and_then(|v| v.as_array()) {
                for item in list {
                    if let Some(s) = item.as_str() {
                        let trimmed = s.trim();
                        if !trimmed.is_empty() && !out.iter().any(|v| v == trimmed) {
                            out.push(trimmed.to_string());
                        }
                    }
                }
            }
        }
        out
    } else {
        Vec::new()
    };
    let assessment_plan = if include_assessment_plan {
        let mark_set_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM mark_sets WHERE class_id = ? AND deleted_at IS NULL",
                [class_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let assessment_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM assessments a JOIN mark_sets m ON m.id = a.mark_set_id WHERE m.class_id = ? AND m.deleted_at IS NULL",
                [class_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        Some(json!({
            "markSetCount": mark_set_count,
            "assessmentCount": assessment_count
        }))
    } else {
        None
    };

    let mut model = json!({
        "class": { "id": class_id, "name": class_name },
        "profile": {
            "courseTitle": if course_title.is_empty() { class_name.clone() } else { course_title.to_string() },
            "gradeLabel": grade_label,
            "periodMinutes": period_minutes,
            "periodsPerWeek": periods_per_week,
            "totalWeeks": total_weeks,
            "strands": strands,
            "policyText": policy_text,
        },
        "resources": resources,
        "schedule": {
            "periodMinutes": period_minutes,
            "periodsPerWeek": periods_per_week,
            "totalWeeks": total_weeks,
            "totalHours": total_hours
        },
        "units": units,
        "lessons": lessons,
        "generatedAt": now_ts(),
        "settingsApplied": settings_applied,
    });
    if let Some(plan) = assessment_plan {
        if let Some(obj) = model.as_object_mut() {
            obj.insert("assessmentPlan".to_string(), plan);
        }
    }
    Ok(model)
}

fn generate_time_management_model(
    conn: &Connection,
    class_id: &str,
    options: Option<&Map<String, JsonValue>>,
) -> Result<JsonValue, String> {
    let class_name: String = conn
        .query_row("SELECT name FROM classes WHERE id = ?", [class_id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let setup_defaults = load_course_setup_defaults(conn);
    let profile = load_profile(conn, class_id, &setup_defaults)?;
    let (
        period_minutes,
        periods_per_week,
        total_weeks,
        _include_policy,
        _include_strands,
        _include_assessment_plan,
        _include_resources,
        include_archived,
        settings_applied,
    ) = resolve_course_description_options(&profile, &setup_defaults, options)?;
    let mut stmt = conn
        .prepare(
            "SELECT COALESCE(duration_minutes, ?) FROM planner_lessons
             WHERE class_id = ? AND (? OR archived = 0)",
        )
        .map_err(|e| e.to_string())?;
    let durations = stmt
        .query_map(params![period_minutes, class_id, include_archived], |r| r.get::<_, i64>(0))
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| e.to_string())?;
    let planned_minutes: i64 = durations.iter().sum();
    let available_minutes = period_minutes * periods_per_week * total_weeks;
    let utilization = if available_minutes > 0 {
        (planned_minutes as f64) * 100.0 / (available_minutes as f64)
    } else {
        0.0
    };
    Ok(json!({
        "class": { "id": class_id, "name": class_name },
        "inputs": {
            "periodMinutes": period_minutes,
            "periodsPerWeek": periods_per_week,
            "totalWeeks": total_weeks,
            "includeArchived": include_archived
        },
        "totals": {
            "plannedMinutes": planned_minutes,
            "availableMinutes": available_minutes,
            "remainingMinutes": available_minutes - planned_minutes,
            "utilizationPercent": utilization
        },
        "generatedAt": now_ts(),
        "settingsApplied": settings_applied,
    }))
}

fn preview_artifact_model(
    conn: &Connection,
    class_id: &str,
    artifact_kind: &str,
    source_id: Option<&str>,
    options: Option<&Map<String, JsonValue>>,
) -> Result<(String, JsonValue), String> {
    match artifact_kind {
        ARTIFACT_UNIT => {
            let source_id = source_id.ok_or_else(|| "sourceId is required for unit preview".to_string())?;
            let unit = conn
                .query_row(
                    "SELECT id, title, start_date, end_date, summary, expectations_json, resources_json
                     FROM planner_units
                     WHERE class_id = ? AND id = ?",
                    params![class_id, source_id],
                    |r| {
                        let expectations_raw: String = r.get(5)?;
                        let resources_raw: String = r.get(6)?;
                        Ok(json!({
                            "unitId": r.get::<_, String>(0)?,
                            "title": r.get::<_, String>(1)?,
                            "startDate": r.get::<_, Option<String>>(2)?,
                            "endDate": r.get::<_, Option<String>>(3)?,
                            "summary": r.get::<_, String>(4)?,
                            "expectations": parse_json_array_string(&expectations_raw),
                            "resources": parse_json_array_string(&resources_raw),
                        }))
                    },
                )
                .optional()
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "unit not found".to_string())?;
            let mut lessons_stmt = conn
                .prepare(
                    "SELECT id, title, lesson_date, outline, detail, follow_up, homework, duration_minutes
                     FROM planner_lessons
                     WHERE class_id = ? AND unit_id = ? AND archived = 0
                     ORDER BY sort_order, id",
                )
                .map_err(|e| e.to_string())?;
            let lessons = lessons_stmt
                .query_map(params![class_id, source_id], |r| {
                    Ok(json!({
                        "lessonId": r.get::<_, String>(0)?,
                        "title": r.get::<_, String>(1)?,
                        "lessonDate": r.get::<_, Option<String>>(2)?,
                        "outline": r.get::<_, String>(3)?,
                        "detail": r.get::<_, String>(4)?,
                        "followUp": r.get::<_, String>(5)?,
                        "homework": r.get::<_, String>(6)?,
                        "durationMinutes": r.get::<_, Option<i64>>(7)?
                    }))
                })
                .and_then(|it| it.collect::<Result<Vec<_>, _>>())
                .map_err(|e| e.to_string())?;
            let title = unit
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Unit")
                .to_string();
            Ok((title.clone(), json!({ "artifactKind": ARTIFACT_UNIT, "title": title, "unit": unit, "lessons": lessons })))
        }
        ARTIFACT_LESSON => {
            let source_id =
                source_id.ok_or_else(|| "sourceId is required for lesson preview".to_string())?;
            let lesson = conn
                .query_row(
                    "SELECT l.id, l.unit_id, l.title, l.lesson_date, l.outline, l.detail, l.follow_up, l.homework, l.duration_minutes, u.title
                     FROM planner_lessons l
                     LEFT JOIN planner_units u ON u.id = l.unit_id
                     WHERE l.class_id = ? AND l.id = ?",
                    params![class_id, source_id],
                    |r| {
                        Ok(json!({
                            "lessonId": r.get::<_, String>(0)?,
                            "unitId": r.get::<_, Option<String>>(1)?,
                            "title": r.get::<_, String>(2)?,
                            "lessonDate": r.get::<_, Option<String>>(3)?,
                            "outline": r.get::<_, String>(4)?,
                            "detail": r.get::<_, String>(5)?,
                            "followUp": r.get::<_, String>(6)?,
                            "homework": r.get::<_, String>(7)?,
                            "durationMinutes": r.get::<_, Option<i64>>(8)?,
                            "unitTitle": r.get::<_, Option<String>>(9)?,
                        }))
                    },
                )
                .optional()
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "lesson not found".to_string())?;
            let title = lesson
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Lesson")
                .to_string();
            Ok((title.clone(), json!({ "artifactKind": ARTIFACT_LESSON, "title": title, "lesson": lesson })))
        }
        ARTIFACT_COURSE_DESCRIPTION => {
            let model = generate_course_description_model(conn, class_id, options)?;
            let title = model
                .pointer("/profile/courseTitle")
                .and_then(|v| v.as_str())
                .unwrap_or("Course Description")
                .to_string();
            Ok((title.clone(), json!({ "artifactKind": ARTIFACT_COURSE_DESCRIPTION, "title": title, "model": model })))
        }
        ARTIFACT_TIME_MANAGEMENT => {
            let model = generate_time_management_model(conn, class_id, options)?;
            Ok((
                "Time Management".to_string(),
                json!({ "artifactKind": ARTIFACT_TIME_MANAGEMENT, "title": "Time Management", "model": model }),
            ))
        }
        _ => Err("artifactKind must be one of: unit, lesson, course_description, time_management".to_string()),
    }
}

fn handle_publish_list(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let artifact_kind = match parse_opt_string(req.params.get("artifactKind")) {
        Ok(v) => v,
        Err(m) => return err(&req.id, "bad_params", format!("artifactKind {}", m), None),
    };
    if let Some(ref kind) = artifact_kind {
        if !validate_artifact_kind(kind) {
            return err(
                &req.id,
                "bad_params",
                "artifactKind must be one of: unit, lesson, course_description, time_management",
                None,
            );
        }
    }
    let status = match parse_opt_string(req.params.get("status")) {
        Ok(v) => v,
        Err(m) => return err(&req.id, "bad_params", format!("status {}", m), None),
    };
    if let Some(ref s) = status {
        if !validate_publish_status(s) {
            return err(
                &req.id,
                "bad_params",
                "status must be one of: draft, published, archived",
                None,
            );
        }
    }

    let mut where_clause = String::from("class_id = ?");
    let mut values: Vec<Value> = vec![Value::Text(class_id)];
    if let Some(kind) = artifact_kind {
        where_clause.push_str(" AND artifact_kind = ?");
        values.push(Value::Text(kind));
    }
    if let Some(status) = status {
        where_clause.push_str(" AND status = ?");
        values.push(Value::Text(status));
    }
    let sql = format!(
        "SELECT id, artifact_kind, source_id, title, status, version, model_json, created_at, updated_at
         FROM planner_publish
         WHERE {}
         ORDER BY updated_at DESC, id DESC",
        where_clause
    );
    let mut stmt = match conn.prepare(&sql) {
        Ok(s) => s,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let publishes = match stmt.query_map(params_from_iter(values), |r| {
        let model_json: String = r.get(6)?;
        let model = serde_json::from_str::<JsonValue>(&model_json).unwrap_or_else(|_| json!({}));
        Ok(json!({
            "id": r.get::<_, String>(0)?,
            "artifactKind": r.get::<_, String>(1)?,
            "sourceId": r.get::<_, Option<String>>(2)?,
            "title": r.get::<_, String>(3)?,
            "status": r.get::<_, String>(4)?,
            "version": r.get::<_, i64>(5)?,
            "model": model,
            "createdAt": r.get::<_, String>(7)?,
            "updatedAt": r.get::<_, String>(8)?,
        }))
    }) {
        Ok(rows) => match rows.collect::<Result<Vec<_>, _>>() {
            Ok(v) => v,
            Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
        },
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    ok(&req.id, json!({ "published": publishes }))
}

fn handle_publish_preview(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let artifact_kind = match required_str(req, "artifactKind") {
        Ok(v) => v.to_ascii_lowercase(),
        Err(e) => return e,
    };
    if !validate_artifact_kind(&artifact_kind) {
        return err(
            &req.id,
            "bad_params",
            "artifactKind must be one of: unit, lesson, course_description, time_management",
            None,
        );
    }
    let source_id = req.params.get("sourceId").and_then(|v| v.as_str());
    let options = req.params.get("options").and_then(|v| v.as_object());
    match preview_artifact_model(conn, &class_id, &artifact_kind, source_id, options) {
        Ok((title, model)) => ok(
            &req.id,
            json!({
                "artifactKind": artifact_kind,
                "sourceId": source_id,
                "title": title,
                "model": model,
            }),
        ),
        Err(msg) => {
            if msg.contains("not found") {
                err(&req.id, "not_found", msg, None)
            } else if msg.contains("must be") || msg.contains("required") {
                err(&req.id, "bad_params", msg, None)
            } else {
                err(&req.id, "db_query_failed", msg, None)
            }
        }
    }
}

fn handle_publish_commit(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let planner_defaults = load_planner_setup_defaults(conn);
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let artifact_kind = match required_str(req, "artifactKind") {
        Ok(v) => v.to_ascii_lowercase(),
        Err(e) => return e,
    };
    if !validate_artifact_kind(&artifact_kind) {
        return err(
            &req.id,
            "bad_params",
            "artifactKind must be one of: unit, lesson, course_description, time_management",
            None,
        );
    }
    let source_id = match parse_opt_string(req.params.get("sourceId")) {
        Ok(v) => v,
        Err(m) => return err(&req.id, "bad_params", format!("sourceId {}", m), None),
    };
    let title = match req.params.get("title").and_then(|v| v.as_str()) {
        Some(v) => v.trim().to_string(),
        None => return err(&req.id, "bad_params", "missing title", None),
    };
    if title.is_empty() {
        return err(&req.id, "bad_params", "title must not be empty", None);
    }
    let model = req.params.get("model").cloned().unwrap_or_else(|| json!({}));
    let status = match parse_opt_string(req.params.get("status")) {
        Ok(v) => v.unwrap_or_else(|| planner_defaults.default_publish_status.clone()),
        Err(m) => return err(&req.id, "bad_params", format!("status {}", m), None),
    };
    if !validate_publish_status(&status) {
        return err(
            &req.id,
            "bad_params",
            "status must be one of: draft, published, archived",
            None,
        );
    }

    let version: i64 = match conn.query_row(
        "SELECT COALESCE(MAX(version), 0) + 1
         FROM planner_publish
         WHERE class_id = ? AND artifact_kind = ? AND COALESCE(source_id,'') = COALESCE(?, '')",
        params![class_id, artifact_kind, source_id],
        |r| r.get(0),
    ) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let publish_id = Uuid::new_v4().to_string();
    let ts = now_ts();
    if let Err(e) = conn.execute(
        "INSERT INTO planner_publish(
            id, class_id, artifact_kind, source_id, title, status, version, model_json, created_at, updated_at
         ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            publish_id,
            class_id,
            artifact_kind,
            source_id,
            title,
            status,
            version,
            model.to_string(),
            ts,
            ts
        ],
    ) {
        return err(&req.id, "db_insert_failed", e.to_string(), None);
    }
    ok(
        &req.id,
        json!({
            "ok": true,
            "publishId": publish_id,
            "status": status,
            "version": version,
            "settingsApplied": {
                "plannerDefaults": {
                    "defaultLessonDurationMinutes": planner_defaults.default_lesson_duration_minutes,
                    "defaultPublishStatus": planner_defaults.default_publish_status,
                    "showArchivedByDefault": planner_defaults.show_archived_by_default,
                    "defaultUnitTitlePrefix": planner_defaults.default_unit_title_prefix
                }
            }
        }),
    )
}

fn handle_publish_update_status(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let publish_id = match required_str(req, "publishId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let status = match required_str(req, "status") {
        Ok(v) => v.to_ascii_lowercase(),
        Err(e) => return e,
    };
    if !validate_publish_status(&status) {
        return err(
            &req.id,
            "bad_params",
            "status must be one of: draft, published, archived",
            None,
        );
    }
    match conn.execute(
        "UPDATE planner_publish SET status = ?, updated_at = ? WHERE class_id = ? AND id = ?",
        params![status, now_ts(), class_id, publish_id],
    ) {
        Ok(0) => err(&req.id, "not_found", "published artifact not found", None),
        Ok(_) => ok(&req.id, json!({ "ok": true })),
        Err(e) => err(&req.id, "db_update_failed", e.to_string(), None),
    }
}

fn handle_course_profile_get(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    if let Err(code) = ensure_class_exists(conn, &class_id) {
        return err(
            &req.id,
            code,
            if code == "not_found" {
                "class not found".to_string()
            } else {
                "failed to read class".to_string()
            },
            None,
        );
    }
    let setup_defaults = load_course_setup_defaults(conn);
    match load_profile(conn, &class_id, &setup_defaults) {
        Ok(profile) => ok(&req.id, json!({ "classId": class_id, "profile": profile })),
        Err(msg) => err(&req.id, "db_query_failed", msg, None),
    }
}

fn handle_course_profile_update(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let Some(patch) = req.params.get("patch").and_then(|v| v.as_object()) else {
        return err(&req.id, "bad_params", "missing patch", None);
    };
    let setup_defaults = load_course_setup_defaults(conn);
    let mut profile = match load_profile(conn, &class_id, &setup_defaults) {
        Ok(v) => v,
        Err(msg) => return err(&req.id, "db_query_failed", msg, None),
    };
    let p = profile.as_object_mut().expect("profile object");
    for (k, v) in patch {
        match k.as_str() {
            "courseTitle" | "gradeLabel" | "policyText" => {
                let Some(s) = v.as_str() else {
                    return err(&req.id, "bad_params", format!("patch.{} must be string", k), None);
                };
                p.insert(k.clone(), JsonValue::String(s.to_string()));
            }
            "periodMinutes" | "periodsPerWeek" | "totalWeeks" => {
                let Some(n) = v.as_i64() else {
                    return err(&req.id, "bad_params", format!("patch.{} must be integer", k), None);
                };
                if n <= 0 {
                    return err(&req.id, "bad_params", format!("patch.{} must be > 0", k), None);
                }
                p.insert(k.clone(), JsonValue::Number(n.into()));
            }
            "strands" => {
                let strands = match parse_string_array(Some(v)) {
                    Ok(v) => v,
                    Err(m) => return err(&req.id, "bad_params", format!("patch.strands {}", m), None),
                };
                p.insert(
                    "strands".to_string(),
                    JsonValue::Array(strands.into_iter().map(JsonValue::String).collect()),
                );
            }
            _ => return err(&req.id, "bad_params", format!("unknown patch field: {}", k), None),
        }
    }
    p.insert("updatedAt".to_string(), JsonValue::String(now_ts()));
    let strands_json = serde_json::to_string(
        &profile
            .get("strands")
            .cloned()
            .unwrap_or_else(|| JsonValue::Array(Vec::new())),
    )
    .unwrap_or_else(|_| "[]".to_string());
    let course_title = profile
        .get("courseTitle")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let grade_label = profile
        .get("gradeLabel")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let period_minutes = profile
        .get("periodMinutes")
        .and_then(|v| v.as_i64())
        .unwrap_or(75);
    let periods_per_week = profile
        .get("periodsPerWeek")
        .and_then(|v| v.as_i64())
        .unwrap_or(5);
    let total_weeks = profile
        .get("totalWeeks")
        .and_then(|v| v.as_i64())
        .unwrap_or(36);
    let policy_text = profile
        .get("policyText")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let updated_at = profile
        .get("updatedAt")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if let Err(e) = conn.execute(
        "INSERT INTO course_description_profiles(
            class_id, course_title, grade_label, period_minutes, periods_per_week, total_weeks, strands_json, policy_text, updated_at
         ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(class_id) DO UPDATE SET
            course_title = excluded.course_title,
            grade_label = excluded.grade_label,
            period_minutes = excluded.period_minutes,
            periods_per_week = excluded.periods_per_week,
            total_weeks = excluded.total_weeks,
            strands_json = excluded.strands_json,
            policy_text = excluded.policy_text,
            updated_at = excluded.updated_at",
        params![
            class_id,
            course_title,
            grade_label,
            period_minutes,
            periods_per_week,
            total_weeks,
            strands_json,
            policy_text,
            updated_at
        ],
    ) {
        return err(&req.id, "db_update_failed", e.to_string(), None);
    }
    ok(&req.id, json!({ "ok": true }))
}

fn handle_course_generate_model(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let options = req.params.get("options").and_then(|v| v.as_object());
    match generate_course_description_model(conn, &class_id, options) {
        Ok(model) => ok(&req.id, model),
        Err(msg) => {
            if msg.contains("must be") {
                err(&req.id, "bad_params", msg, None)
            } else {
                err(&req.id, "db_query_failed", msg, None)
            }
        }
    }
}

fn handle_course_time_management_model(state: &mut AppState, req: &Request) -> serde_json::Value {
    let conn = match db_conn(state, req) {
        Ok(c) => c,
        Err(e) => return e,
    };
    let class_id = match required_str(req, "classId") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let options = req.params.get("options").and_then(|v| v.as_object());
    match generate_time_management_model(conn, &class_id, options) {
        Ok(model) => ok(&req.id, model),
        Err(msg) => {
            if msg.contains("must be") {
                err(&req.id, "bad_params", msg, None)
            } else {
                err(&req.id, "db_query_failed", msg, None)
            }
        }
    }
}

pub fn reports_planner_unit_model(
    conn: &Connection,
    class_id: &str,
    unit_id: &str,
) -> Result<JsonValue, String> {
    let (_title, model) = preview_artifact_model(conn, class_id, ARTIFACT_UNIT, Some(unit_id), None)?;
    Ok(model)
}

pub fn reports_planner_lesson_model(
    conn: &Connection,
    class_id: &str,
    lesson_id: &str,
) -> Result<JsonValue, String> {
    let (_title, model) =
        preview_artifact_model(conn, class_id, ARTIFACT_LESSON, Some(lesson_id), None)?;
    Ok(model)
}

pub fn reports_course_description_model(
    conn: &Connection,
    class_id: &str,
    options: Option<&Map<String, JsonValue>>,
) -> Result<JsonValue, String> {
    generate_course_description_model(conn, class_id, options)
}

pub fn reports_time_management_model(
    conn: &Connection,
    class_id: &str,
    options: Option<&Map<String, JsonValue>>,
) -> Result<JsonValue, String> {
    generate_time_management_model(conn, class_id, options)
}

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "planner.units.list" => Some(handle_units_list(state, req)),
        "planner.units.open" => Some(handle_units_open(state, req)),
        "planner.units.create" => Some(handle_units_create(state, req)),
        "planner.units.update" => Some(handle_units_update(state, req)),
        "planner.units.reorder" => Some(handle_units_reorder(state, req)),
        "planner.units.archive" => Some(handle_units_archive(state, req)),
        "planner.units.clone" => Some(handle_units_clone(state, req)),
        "planner.lessons.list" => Some(handle_lessons_list(state, req)),
        "planner.lessons.open" => Some(handle_lessons_open(state, req)),
        "planner.lessons.create" => Some(handle_lessons_create(state, req)),
        "planner.lessons.update" => Some(handle_lessons_update(state, req)),
        "planner.lessons.reorder" => Some(handle_lessons_reorder(state, req)),
        "planner.lessons.archive" => Some(handle_lessons_archive(state, req)),
        "planner.lessons.copyForward" => Some(handle_lessons_copy_forward(state, req)),
        "planner.lessons.bulkAssignUnit" => Some(handle_lessons_bulk_assign_unit(state, req)),
        "planner.publish.list" => Some(handle_publish_list(state, req)),
        "planner.publish.preview" => Some(handle_publish_preview(state, req)),
        "planner.publish.commit" => Some(handle_publish_commit(state, req)),
        "planner.publish.updateStatus" => Some(handle_publish_update_status(state, req)),
        "courseDescription.getProfile" => Some(handle_course_profile_get(state, req)),
        "courseDescription.updateProfile" => Some(handle_course_profile_update(state, req)),
        "courseDescription.generateModel" => Some(handle_course_generate_model(state, req)),
        "courseDescription.timeManagementModel" => Some(handle_course_time_management_model(state, req)),
        _ => None,
    }
}
