use crate::calc;
use crate::db;
use crate::ipc::error::{err, ok};
use crate::ipc::types::{AppState, Request};
use rusqlite::{params_from_iter, Connection, OptionalExtension};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

const ADMIN_TRANSFER_FORMAT: &str = "mb-admin-transfer-v1";

struct HandlerErr {
    code: &'static str,
    message: String,
    details: Option<Value>,
}

impl HandlerErr {
    fn response(self, id: &str) -> Value {
        err(id, self.code, self.message, self.details)
    }
}

#[derive(Clone, Debug)]
struct StudentRow {
    id: String,
    student_no: Option<String>,
    last_name: String,
    first_name: String,
    birth_date: Option<String>,
    active: bool,
    sort_order: i64,
    mark_set_mask: String,
}

#[derive(Clone, Debug)]
struct SisRosterRow {
    line_no: usize,
    student_no: Option<String>,
    last_name: String,
    first_name: String,
    birth_date: Option<String>,
    active: Option<bool>,
}

#[derive(Clone, Debug)]
struct SourceAssessmentRow {
    idx: i64,
    date: Option<String>,
    category_name: Option<String>,
    title: String,
    term: Option<i64>,
    legacy_type: Option<i64>,
    weight: Option<f64>,
    out_of: Option<f64>,
}

#[derive(Clone, Debug)]
struct SourceScoreRow {
    assessment_idx: i64,
    student_id: String,
    status: String,
    raw_value: Option<f64>,
    remark: Option<String>,
}

#[derive(Clone, Debug)]
struct SourceCommentSetRow {
    set_number: i64,
    title: String,
    max_chars: i64,
    fit_width: i64,
    fit_lines: i64,
    bank_short: Option<String>,
    remarks: Vec<(String, String)>,
}

#[derive(Clone, Debug)]
struct SourceMarkSetPackage {
    code: String,
    description: String,
    assessments: Vec<SourceAssessmentRow>,
    scores: Vec<SourceScoreRow>,
    comment_sets: Vec<SourceCommentSetRow>,
}

#[derive(Clone, Debug)]
struct SourceStudentPackage {
    source_id: String,
    student_no: Option<String>,
    last_name: String,
    first_name: String,
    birth_date: Option<String>,
    active: bool,
    mark_set_mask: String,
}

#[derive(Clone, Debug)]
struct SourceLearningSkillsRow {
    source_student_id: String,
    term: i64,
    skill_code: String,
    value: String,
}

#[derive(Clone, Debug)]
struct AdminTransferPackage {
    manifest: Value,
    students: Vec<SourceStudentPackage>,
    mark_sets: Vec<SourceMarkSetPackage>,
    learning_skills: Vec<SourceLearningSkillsRow>,
}

fn get_required_str(params: &Value, key: &str) -> Result<String, HandlerErr> {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| HandlerErr {
            code: "bad_params",
            message: format!("missing {}", key),
            details: None,
        })
}

fn normalize_key(s: &str) -> String {
    s.trim().to_ascii_lowercase()
}

fn normalized_name_key(last: &str, first: &str) -> String {
    format!("{}|{}", normalize_key(last), normalize_key(first))
}

fn non_empty_trimmed(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
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

fn csv_quote(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn parse_boolish(s: &str) -> Option<bool> {
    match s.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "y" => Some(true),
        "0" | "false" | "no" | "n" => Some(false),
        "" => None,
        _ => None,
    }
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

fn load_section(conn: &Connection, key: &str) -> Value {
    db::settings_get_json(conn, key)
        .ok()
        .flatten()
        .and_then(|v| v.as_object().cloned())
        .map(Value::Object)
        .unwrap_or_else(|| json!({}))
}

fn get_setup_string(conn: &Connection, section_key: &str, field: &str, default: &str) -> String {
    load_section(conn, section_key)
        .get(field)
        .and_then(|v| v.as_str())
        .map(|s| s.to_ascii_lowercase())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn parse_student_match_mode(params: &Value, conn: &Connection) -> Result<String, HandlerErr> {
    let default_mode =
        get_setup_string(conn, "setup.integrations", "defaultMatchMode", "student_no_then_name");
    let mode = params
        .get("matchMode")
        .and_then(|v| v.as_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or(default_mode);
    if mode == "student_no_then_name" || mode == "name_only" || mode == "sort_order" {
        Ok(mode)
    } else {
        Err(HandlerErr {
            code: "bad_params",
            message: "invalid matchMode".to_string(),
            details: None,
        })
    }
}

fn parse_collision_policy(params: &Value, conn: &Connection) -> Result<String, HandlerErr> {
    let default_policy =
        get_setup_string(conn, "setup.integrations", "defaultCollisionPolicy", "merge_existing");
    let policy = params
        .get("collisionPolicy")
        .and_then(|v| v.as_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or(default_policy);
    if policy == "merge_existing" || policy == "append_new" || policy == "stop_on_collision" {
        Ok(policy)
    } else {
        Err(HandlerErr {
            code: "bad_params",
            message: "invalid collisionPolicy".to_string(),
            details: None,
        })
    }
}

fn parse_comment_policy(params: &Value, conn: &Connection) -> Result<String, HandlerErr> {
    let default_policy =
        get_setup_string(conn, "setup.comments", "defaultTransferPolicy", "fill_blank");
    let policy = params
        .get("commentPolicy")
        .and_then(|v| v.as_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or(default_policy);
    if policy == "replace" || policy == "append" || policy == "fill_blank" || policy == "source_if_longer" {
        Ok(policy)
    } else {
        Err(HandlerErr {
            code: "bad_params",
            message: "invalid commentPolicy".to_string(),
            details: None,
        })
    }
}

fn parse_scope(params: &Value, conn: &Connection) -> Result<String, HandlerErr> {
    let default_scope =
        get_setup_string(conn, "setup.reports", "defaultAnalyticsScope", "valid");
    let scope = params
        .get("studentScope")
        .and_then(|v| v.as_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or(default_scope);
    if scope == "all" || scope == "active" || scope == "valid" {
        Ok(scope)
    } else {
        Err(HandlerErr {
            code: "bad_params",
            message: "studentScope must be one of: all, active, valid".to_string(),
            details: None,
        })
    }
}

fn parse_filters(params: &Value) -> Result<(Option<i64>, Option<String>, Option<i64>), HandlerErr> {
    let Some(filters) = params.get("filters") else {
        return Ok((None, None, None));
    };
    let Some(obj) = filters.as_object() else {
        return Err(HandlerErr {
            code: "bad_params",
            message: "filters must be object".to_string(),
            details: None,
        });
    };
    let term = match obj.get("term") {
        None => None,
        Some(v) if v.is_null() => None,
        Some(v) => {
            let Some(n) = v.as_i64() else {
                return Err(HandlerErr {
                    code: "bad_params",
                    message: "filters.term must be integer or null".to_string(),
                    details: None,
                });
            };
            Some(n)
        }
    };
    let category_name = match obj.get("categoryName") {
        None => None,
        Some(v) if v.is_null() => None,
        Some(v) => {
            let Some(s) = v.as_str() else {
                return Err(HandlerErr {
                    code: "bad_params",
                    message: "filters.categoryName must be string or null".to_string(),
                    details: None,
                });
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
        Some(v) => v.as_i64(),
    };
    Ok((term, category_name, types_mask))
}

fn list_students(conn: &Connection, class_id: &str) -> Result<Vec<StudentRow>, HandlerErr> {
    let mut stmt = conn
        .prepare(
            "SELECT id, student_no, last_name, first_name, birth_date, active, sort_order, COALESCE(mark_set_mask, 'TBA')
             FROM students
             WHERE class_id = ?
             ORDER BY sort_order",
        )
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    stmt.query_map([class_id], |r| {
        Ok(StudentRow {
            id: r.get(0)?,
            student_no: r.get(1)?,
            last_name: r.get(2)?,
            first_name: r.get(3)?,
            birth_date: r.get(4)?,
            active: r.get::<_, i64>(5)? != 0,
            sort_order: r.get(6)?,
            mark_set_mask: r.get(7)?,
        })
    })
    .and_then(|it| it.collect::<Result<Vec<_>, _>>())
    .map_err(|e| HandlerErr {
        code: "db_query_failed",
        message: e.to_string(),
        details: None,
    })
}

fn parse_sis_roster_rows(text: &str) -> (Vec<SisRosterRow>, Vec<Value>, usize) {
    let mut warnings = Vec::new();
    let mut rows = Vec::new();
    let lines = text.lines().collect::<Vec<_>>();
    if lines.is_empty() {
        return (rows, warnings, 0);
    }

    let header_fields = parse_csv_record(lines[0])
        .into_iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .collect::<Vec<_>>();
    let mut idx = HashMap::<String, usize>::new();
    for (i, f) in header_fields.iter().enumerate() {
        idx.insert(f.clone(), i);
    }

    let student_no_col = idx.get("student_no").copied().unwrap_or(0);
    let last_col = idx
        .get("last_name")
        .or_else(|| idx.get("last"))
        .copied()
        .unwrap_or(1);
    let first_col = idx
        .get("first_name")
        .or_else(|| idx.get("first"))
        .copied()
        .unwrap_or(2);
    let active_col = idx.get("active").copied().unwrap_or(3);
    let birth_col = idx.get("birth_date").copied().unwrap_or(4);

    let mut total = 0usize;
    for (line_no, raw_line) in lines.iter().enumerate().skip(1) {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        total += 1;
        let fields = parse_csv_record(line);

        let last_name = fields
            .get(last_col)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        let first_name = fields
            .get(first_col)
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        if last_name.is_empty() || first_name.is_empty() {
            warnings.push(json!({
                "line": line_no + 1,
                "code": "missing_name",
                "message": "first_name and last_name are required"
            }));
            continue;
        }
        let student_no = fields.get(student_no_col).and_then(|s| non_empty_trimmed(s));
        let birth_date = fields.get(birth_col).and_then(|s| non_empty_trimmed(s));
        let active = fields
            .get(active_col)
            .and_then(|s| parse_boolish(s));

        rows.push(SisRosterRow {
            line_no: line_no + 1,
            student_no,
            last_name,
            first_name,
            birth_date,
            active,
        });
    }

    (rows, warnings, total)
}

fn pick_unique_non_used(
    ids: Option<&Vec<String>>,
    used: &HashSet<String>,
) -> (Option<String>, bool) {
    let Some(ids) = ids else {
        return (None, false);
    };
    let mut available = ids
        .iter()
        .filter(|id| !used.contains(id.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    available.sort();
    available.dedup();
    if available.len() == 1 {
        (available.first().cloned(), false)
    } else if available.is_empty() {
        (None, false)
    } else {
        (None, true)
    }
}

fn find_student_match(
    row: &SisRosterRow,
    mode: &str,
    row_index: usize,
    by_student_no: &HashMap<String, Vec<String>>,
    by_name: &HashMap<String, Vec<String>>,
    by_sort: &[String],
    used: &HashSet<String>,
) -> (Option<String>, bool) {
    if mode == "sort_order" {
        let target = by_sort.get(row_index).cloned();
        return (target, false);
    }
    if mode == "student_no_then_name" {
        if let Some(student_no) = row.student_no.as_deref().map(normalize_key) {
            let (id, ambiguous) = pick_unique_non_used(by_student_no.get(&student_no), used);
            if id.is_some() || ambiguous {
                return (id, ambiguous);
            }
        }
    }

    let key = normalized_name_key(&row.last_name, &row.first_name);
    pick_unique_non_used(by_name.get(&key), used)
}

fn now_unix_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

fn transfer_text_by_policy(source: &str, target: &str, policy: &str, separator: &str) -> Option<String> {
    let s = source.trim();
    let t = target.trim();
    match policy {
        "replace" => Some(s.to_string()),
        "append" => {
            if s.is_empty() {
                None
            } else if t.is_empty() {
                Some(s.to_string())
            } else {
                Some(format!("{t}{separator}{s}"))
            }
        }
        "fill_blank" => {
            if t.is_empty() && !s.is_empty() {
                Some(s.to_string())
            } else {
                None
            }
        }
        "source_if_longer" => {
            if s.chars().count() > t.chars().count() {
                Some(s.to_string())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn handle_sis_preview_import(state: &mut AppState, req: &Request) -> Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    let class_id = match get_required_str(&req.params, "classId") {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let in_path = match get_required_str(&req.params, "inPath") {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let profile_default =
        get_setup_string(conn, "setup.integrations", "defaultSisProfile", "sis_roster_v1");
    let profile = req
        .params
        .get("profile")
        .and_then(|v| v.as_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or(profile_default);
    let mode = req
        .params
        .get("mode")
        .and_then(|v| v.as_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_else(|| "upsert_preserve".to_string());
    let match_mode = match parse_student_match_mode(&req.params, conn) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };

    let text = match std::fs::read_to_string(&in_path) {
        Ok(t) => t,
        Err(e) => {
            return err(
                &req.id,
                "parse_failed",
                e.to_string(),
                Some(json!({ "path": in_path })),
            )
        }
    };
    let (rows, mut warnings, rows_total) = parse_sis_roster_rows(&text);
    let students = match list_students(conn, &class_id) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let mut by_student_no: HashMap<String, Vec<String>> = HashMap::new();
    let mut by_name: HashMap<String, Vec<String>> = HashMap::new();
    let mut by_sort: Vec<String> = Vec::new();
    for s in &students {
        if let Some(student_no) = s.student_no.as_deref().map(normalize_key) {
            by_student_no.entry(student_no).or_default().push(s.id.clone());
        }
        by_name
            .entry(normalized_name_key(&s.last_name, &s.first_name))
            .or_default()
            .push(s.id.clone());
        by_sort.push(s.id.clone());
    }

    let mut used = HashSet::<String>::new();
    let mut matched = 0usize;
    let mut new_count = 0usize;
    let mut ambiguous = 0usize;
    let mut invalid = warnings.len();
    let mut preview_rows = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let (matched_id, is_ambiguous) = find_student_match(
            row,
            &match_mode,
            i,
            &by_student_no,
            &by_name,
            &by_sort,
            &used,
        );
        if is_ambiguous {
            ambiguous += 1;
            warnings.push(json!({
                "line": row.line_no,
                "code": "ambiguous_match",
                "message": "multiple target students matched incoming row"
            }));
            preview_rows.push(json!({
                "line": row.line_no,
                "studentNo": row.student_no,
                "displayName": format!("{}, {}", row.last_name, row.first_name),
                "status": "ambiguous"
            }));
            continue;
        }
        if let Some(student_id) = matched_id {
            matched += 1;
            used.insert(student_id.clone());
            preview_rows.push(json!({
                "line": row.line_no,
                "studentNo": row.student_no,
                "displayName": format!("{}, {}", row.last_name, row.first_name),
                "status": "matched",
                "matchedStudentId": student_id
            }));
        } else {
            new_count += 1;
            preview_rows.push(json!({
                "line": row.line_no,
                "studentNo": row.student_no,
                "displayName": format!("{}, {}", row.last_name, row.first_name),
                "status": "new"
            }));
        }
    }
    invalid += warnings
        .iter()
        .filter(|w| w.get("code").and_then(|c| c.as_str()) == Some("missing_name"))
        .count();

    ok(
        &req.id,
        json!({
            "ok": true,
            "classId": class_id,
            "path": in_path,
            "profile": profile,
            "matchMode": match_mode,
            "mode": mode,
            "rowsTotal": rows_total,
            "rowsParsed": rows.len(),
            "matched": matched,
            "new": new_count,
            "ambiguous": ambiguous,
            "invalid": invalid,
            "warnings": warnings,
            "previewRows": preview_rows
        }),
    )
}

fn handle_sis_apply_import(state: &mut AppState, req: &Request) -> Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    let class_id = match get_required_str(&req.params, "classId") {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let in_path = match get_required_str(&req.params, "inPath") {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let profile_default =
        get_setup_string(conn, "setup.integrations", "defaultSisProfile", "sis_roster_v1");
    let profile = req
        .params
        .get("profile")
        .and_then(|v| v.as_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or(profile_default);
    let mode = req
        .params
        .get("mode")
        .and_then(|v| v.as_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or_else(|| "upsert_preserve".to_string());
    if mode != "upsert_preserve" && mode != "replace_snapshot" {
        return err(
            &req.id,
            "bad_params",
            "mode must be one of: upsert_preserve, replace_snapshot",
            None,
        );
    }
    let match_mode = match parse_student_match_mode(&req.params, conn) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let preserve_local_validity = req
        .params
        .get("preserveLocalValidity")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let collision_policy = match parse_collision_policy(&req.params, conn) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };

    let text = match std::fs::read_to_string(&in_path) {
        Ok(t) => t,
        Err(e) => {
            return err(
                &req.id,
                "parse_failed",
                e.to_string(),
                Some(json!({ "path": in_path })),
            )
        }
    };
    let (rows, mut warnings, _rows_total) = parse_sis_roster_rows(&text);
    let existing_students = match list_students(conn, &class_id) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let mut by_student_no: HashMap<String, Vec<String>> = HashMap::new();
    let mut by_name: HashMap<String, Vec<String>> = HashMap::new();
    let mut by_sort: Vec<String> = Vec::new();
    let mut existing_ids = HashSet::<String>::new();
    for s in &existing_students {
        existing_ids.insert(s.id.clone());
        if let Some(student_no) = s.student_no.as_deref().map(normalize_key) {
            by_student_no.entry(student_no).or_default().push(s.id.clone());
        }
        by_name
            .entry(normalized_name_key(&s.last_name, &s.first_name))
            .or_default()
            .push(s.id.clone());
        by_sort.push(s.id.clone());
    }

    let tx = match conn.unchecked_transaction() {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let now = now_unix_string();
    let mut used = HashSet::<String>::new();
    let mut ordered = Vec::<String>::new();
    let mut created = 0usize;
    let mut updated = 0usize;
    let mut ambiguous_skipped = 0usize;

    for (row_index, row) in rows.iter().enumerate() {
        let (matched_id, ambiguous) = find_student_match(
            row,
            &match_mode,
            row_index,
            &by_student_no,
            &by_name,
            &by_sort,
            &used,
        );
        if ambiguous {
            ambiguous_skipped += 1;
            warnings.push(json!({
                "line": row.line_no,
                "code": "ambiguous_match",
                "message": "multiple target students matched incoming row"
            }));
            continue;
        }
        if let Some(student_id) = matched_id {
            let preserve_fields = tx
                .query_row(
                    "SELECT active, COALESCE(mark_set_mask, 'TBA') FROM students WHERE id = ?",
                    [&student_id],
                    |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
                )
                .optional()
                .ok()
                .flatten()
                .unwrap_or((1, "TBA".to_string()));
            let active_incoming = row.active.unwrap_or(true);
            let next_active = if preserve_local_validity {
                preserve_fields.0 != 0
            } else {
                active_incoming
            };
            let next_mask = if preserve_local_validity {
                preserve_fields.1
            } else {
                "TBA".to_string()
            };
            if let Err(e) = tx.execute(
                "UPDATE students
                 SET last_name = ?, first_name = ?, student_no = ?, birth_date = ?, active = ?, mark_set_mask = ?, raw_line = ?, updated_at = ?
                 WHERE id = ? AND class_id = ?",
                (
                    &row.last_name,
                    &row.first_name,
                    &row.student_no,
                    &row.birth_date,
                    if next_active { 1 } else { 0 },
                    &next_mask,
                    format!(
                        "{},{},{},{},{}",
                        if next_active { 1 } else { 0 },
                        row.last_name,
                        row.first_name,
                        row.student_no.clone().unwrap_or_default(),
                        row.birth_date.clone().unwrap_or_default()
                    ),
                    &now,
                    &student_id,
                    &class_id,
                ),
            ) {
                let _ = tx.rollback();
                return err(&req.id, "db_update_failed", e.to_string(), None);
            }
            used.insert(student_id.clone());
            ordered.push(student_id);
            updated += 1;
            continue;
        }

        let student_id = Uuid::new_v4().to_string();
        let active = row.active.unwrap_or(true);
        if let Err(e) = tx.execute(
            "INSERT INTO students(id, class_id, last_name, first_name, student_no, birth_date, active, sort_order, raw_line, mark_set_mask, updated_at)
             VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                &student_id,
                &class_id,
                &row.last_name,
                &row.first_name,
                &row.student_no,
                &row.birth_date,
                if active { 1 } else { 0 },
                row_index as i64,
                format!(
                    "{},{},{},{},{}",
                    if active { 1 } else { 0 },
                    row.last_name,
                    row.first_name,
                    row.student_no.clone().unwrap_or_default(),
                    row.birth_date.clone().unwrap_or_default()
                ),
                "TBA",
                &now,
            ),
        ) {
            let _ = tx.rollback();
            return err(&req.id, "db_insert_failed", e.to_string(), None);
        }
        ordered.push(student_id);
        created += 1;
    }

    let mut local_only = existing_students
        .iter()
        .filter(|s| !ordered.contains(&s.id))
        .cloned()
        .collect::<Vec<_>>();
    local_only.sort_by_key(|s| s.sort_order);

    if mode == "replace_snapshot" {
        for s in &local_only {
            let _ = tx.execute(
                "UPDATE students SET active = 0, updated_at = ? WHERE id = ?",
                (&now, &s.id),
            );
        }
    }

    for s in &local_only {
        ordered.push(s.id.clone());
    }
    for (idx, student_id) in ordered.iter().enumerate() {
        if let Err(e) = tx.execute(
            "UPDATE students SET sort_order = ? WHERE id = ? AND class_id = ?",
            (idx as i64, student_id, &class_id),
        ) {
            let _ = tx.rollback();
            return err(&req.id, "db_update_failed", e.to_string(), None);
        }
    }

    if let Err(e) = tx.commit() {
        return err(&req.id, "db_update_failed", e.to_string(), None);
    }

    ok(
        &req.id,
        json!({
            "ok": true,
            "classId": class_id,
            "path": in_path,
            "profile": profile,
            "matchMode": match_mode,
            "mode": mode,
            "collisionPolicy": collision_policy,
            "created": created,
            "updated": updated,
            "ambiguousSkipped": ambiguous_skipped,
            "warnings": warnings
        }),
    )
}

fn resolve_student_scope(
    scope: &str,
    mark_set_sort_order: i64,
    student: &StudentRow,
) -> bool {
    if scope == "all" {
        return true;
    }
    if scope == "active" {
        return student.active;
    }
    calc::is_valid_kid(student.active, &student.mark_set_mask, mark_set_sort_order)
}

fn write_text_file(path: &str, contents: &str) -> Result<(), HandlerErr> {
    let out = PathBuf::from(path);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| HandlerErr {
            code: "export_failed",
            message: e.to_string(),
            details: Some(json!({ "path": path })),
        })?;
    }
    std::fs::write(&out, contents).map_err(|e| HandlerErr {
        code: "export_failed",
        message: e.to_string(),
        details: Some(json!({ "path": path })),
    })?;
    Ok(())
}

fn handle_sis_export_roster(state: &mut AppState, req: &Request) -> Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    let class_id = match get_required_str(&req.params, "classId") {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let out_path = match get_required_str(&req.params, "outPath") {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let profile_default =
        get_setup_string(conn, "setup.integrations", "defaultSisProfile", "sis_roster_v1");
    let profile = req
        .params
        .get("profile")
        .and_then(|v| v.as_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or(profile_default);
    let scope = match parse_scope(&req.params, conn) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let students = match list_students(conn, &class_id) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };

    let mut csv = String::from("student_id,student_no,last_name,first_name,birth_date,active,sort_order,mark_set_mask\n");
    let mut rows_exported = 0usize;
    for s in students {
        if scope != "all" && !s.active {
            continue;
        }
        rows_exported += 1;
        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            csv_quote(&s.id),
            csv_quote(s.student_no.as_deref().unwrap_or("")),
            csv_quote(&s.last_name),
            csv_quote(&s.first_name),
            csv_quote(s.birth_date.as_deref().unwrap_or("")),
            if s.active { "1" } else { "0" },
            s.sort_order,
            csv_quote(&s.mark_set_mask)
        ));
    }
    if let Err(e) = write_text_file(&out_path, &csv) {
        return e.response(&req.id);
    }
    ok(
        &req.id,
        json!({
            "ok": true,
            "rowsExported": rows_exported,
            "profile": profile,
            "path": out_path
        }),
    )
}

fn handle_sis_export_marks(state: &mut AppState, req: &Request) -> Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    let class_id = match get_required_str(&req.params, "classId") {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let mark_set_id = match get_required_str(&req.params, "markSetId") {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let out_path = match get_required_str(&req.params, "outPath") {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let profile_default =
        get_setup_string(conn, "setup.integrations", "defaultSisProfile", "sis_marks_v1");
    let profile = req
        .params
        .get("profile")
        .and_then(|v| v.as_str())
        .map(|s| s.to_ascii_lowercase())
        .unwrap_or(profile_default);
    let include_state_columns = req
        .params
        .get("includeStateColumns")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let scope = match parse_scope(&req.params, conn) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let (term_filter, category_filter, types_mask) = match parse_filters(&req.params) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };

    let mark_set_row: Option<(String, i64)> = conn
        .query_row(
            "SELECT code, sort_order FROM mark_sets WHERE id = ? AND class_id = ?",
            (&mark_set_id, &class_id),
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .ok()
        .flatten();
    let Some((mark_set_code, mark_set_sort_order)) = mark_set_row else {
        return err(&req.id, "not_found", "mark set not found", None);
    };

    let students = match list_students(conn, &class_id) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let scoped_students = students
        .into_iter()
        .filter(|s| resolve_student_scope(&scope, mark_set_sort_order, s))
        .collect::<Vec<_>>();

    let mut stmt = match conn.prepare(
        "SELECT id, idx, title, term, legacy_type, category_name
         FROM assessments
         WHERE mark_set_id = ?
         ORDER BY idx",
    ) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let assessments = match stmt.query_map([&mark_set_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, Option<i64>>(3)?,
            r.get::<_, Option<i64>>(4)?,
            r.get::<_, Option<String>>(5)?,
        ))
    }) {
        Ok(rows) => rows.filter_map(Result::ok).collect::<Vec<_>>(),
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let filtered_assessments = assessments
        .into_iter()
        .filter(|(_, _, _, term, legacy_type, category_name)| {
            let term_ok = term_filter.is_none() || term_filter == *term;
            let category_ok = match category_filter.as_ref() {
                None => true,
                Some(cat) => category_name
                    .as_ref()
                    .map(|n| normalize_key(n) == *cat)
                    .unwrap_or(false),
            };
            let types_ok = matches_types_mask(types_mask, *legacy_type);
            term_ok && category_ok && types_ok
        })
        .collect::<Vec<_>>();

    let mut score_by_key: HashMap<(String, String), (String, Option<f64>)> = HashMap::new();
    if !filtered_assessments.is_empty() {
        let placeholders = std::iter::repeat_n("?", filtered_assessments.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT assessment_id, student_id, status, raw_value FROM scores WHERE assessment_id IN ({})",
            placeholders
        );
        let ids = filtered_assessments
            .iter()
            .map(|(id, ..)| id.clone())
            .collect::<Vec<_>>();
        let params = ids.iter().map(|s| s as &dyn rusqlite::ToSql).collect::<Vec<_>>();
        if let Ok(mut score_stmt) = conn.prepare(&sql) {
            if let Ok(rows) = score_stmt.query_map(params_from_iter(params), |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<f64>>(3)?,
                ))
            }) {
                for row in rows.filter_map(Result::ok) {
                    score_by_key.insert((row.0, row.1), (row.2, row.3));
                }
            }
        }
    }

    let mut csv = if include_state_columns {
        String::from("student_id,student_no,last_name,first_name,mark_set_code,assessment_idx,assessment_title,status,raw_value\n")
    } else {
        String::from("student_id,student_no,last_name,first_name,mark_set_code,assessment_idx,assessment_title,raw_value\n")
    };
    let mut rows_exported = 0usize;
    for (assessment_id, assessment_idx, assessment_title, _, _, _) in &filtered_assessments {
        for student in &scoped_students {
            let (status, raw_value) = score_by_key
                .get(&(assessment_id.clone(), student.id.clone()))
                .cloned()
                .unwrap_or_else(|| ("no_mark".to_string(), Some(0.0)));
            rows_exported += 1;
            if include_state_columns {
                csv.push_str(&format!(
                    "{},{},{},{},{},{},{},{},{}\n",
                    csv_quote(&student.id),
                    csv_quote(student.student_no.as_deref().unwrap_or("")),
                    csv_quote(&student.last_name),
                    csv_quote(&student.first_name),
                    csv_quote(&mark_set_code),
                    assessment_idx,
                    csv_quote(assessment_title),
                    csv_quote(&status),
                    raw_value.map(|v| v.to_string()).unwrap_or_default()
                ));
            } else {
                csv.push_str(&format!(
                    "{},{},{},{},{},{},{},{}\n",
                    csv_quote(&student.id),
                    csv_quote(student.student_no.as_deref().unwrap_or("")),
                    csv_quote(&student.last_name),
                    csv_quote(&student.first_name),
                    csv_quote(&mark_set_code),
                    assessment_idx,
                    csv_quote(assessment_title),
                    raw_value.map(|v| v.to_string()).unwrap_or_default()
                ));
            }
        }
    }
    if let Err(e) = write_text_file(&out_path, &csv) {
        return e.response(&req.id);
    }

    ok(
        &req.id,
        json!({
            "ok": true,
            "rowsExported": rows_exported,
            "assessmentsExported": filtered_assessments.len(),
            "profile": profile,
            "path": out_path
        }),
    )
}

fn read_zip_text_entry<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Option<String> {
    let mut f = archive.by_name(name).ok()?;
    let mut text = String::new();
    if f.read_to_string(&mut text).is_ok() {
        Some(text)
    } else {
        None
    }
}

fn parse_admin_students_csv(text: &str) -> Vec<SourceStudentPackage> {
    let mut out = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if idx == 0 || line.trim().is_empty() {
            continue;
        }
        let fields = parse_csv_record(line);
        if fields.len() < 8 {
            continue;
        }
        out.push(SourceStudentPackage {
            source_id: fields[0].trim().to_string(),
            student_no: non_empty_trimmed(fields[1].as_str()),
            last_name: fields[2].trim().to_string(),
            first_name: fields[3].trim().to_string(),
            birth_date: non_empty_trimmed(fields[4].as_str()),
            active: parse_boolish(fields[5].as_str()).unwrap_or(true),
            mark_set_mask: non_empty_trimmed(fields[7].as_str()).unwrap_or_else(|| "TBA".to_string()),
        });
    }
    out
}

fn parse_admin_assessments_csv(text: &str) -> Vec<SourceAssessmentRow> {
    let mut out = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if idx == 0 || line.trim().is_empty() {
            continue;
        }
        let fields = parse_csv_record(line);
        if fields.len() < 8 {
            continue;
        }
        let assessment_idx = fields[0].trim().parse::<i64>().ok();
        let Some(assessment_idx) = assessment_idx else {
            continue;
        };
        out.push(SourceAssessmentRow {
            idx: assessment_idx,
            date: non_empty_trimmed(fields[1].as_str()),
            category_name: non_empty_trimmed(fields[2].as_str()),
            title: fields[3].trim().to_string(),
            term: fields[4].trim().parse::<i64>().ok(),
            legacy_type: fields[5].trim().parse::<i64>().ok(),
            weight: fields[6].trim().parse::<f64>().ok(),
            out_of: fields[7].trim().parse::<f64>().ok(),
        });
    }
    out
}

fn parse_admin_scores_csv(text: &str) -> Vec<SourceScoreRow> {
    let mut out = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if idx == 0 || line.trim().is_empty() {
            continue;
        }
        let fields = parse_csv_record(line);
        if fields.len() < 5 {
            continue;
        }
        let assessment_idx = fields[0].trim().parse::<i64>().ok();
        let Some(assessment_idx) = assessment_idx else {
            continue;
        };
        out.push(SourceScoreRow {
            assessment_idx,
            student_id: fields[1].trim().to_string(),
            status: fields[2].trim().to_ascii_lowercase(),
            raw_value: fields[3].trim().parse::<f64>().ok(),
            remark: non_empty_trimmed(fields[4].as_str()),
        });
    }
    out
}

fn parse_admin_learning_skills_csv(text: &str) -> Vec<SourceLearningSkillsRow> {
    let mut out = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        if idx == 0 || line.trim().is_empty() {
            continue;
        }
        let fields = parse_csv_record(line);
        if fields.len() < 4 {
            continue;
        }
        let term = fields[1].trim().parse::<i64>().ok();
        let Some(term) = term else {
            continue;
        };
        out.push(SourceLearningSkillsRow {
            source_student_id: fields[0].trim().to_string(),
            term,
            skill_code: fields[2].trim().to_string(),
            value: fields[3].trim().to_string(),
        });
    }
    out
}

fn parse_admin_package(path: &str) -> Result<AdminTransferPackage, HandlerErr> {
    let file = File::open(path).map_err(|e| HandlerErr {
        code: "parse_failed",
        message: e.to_string(),
        details: Some(json!({ "path": path })),
    })?;
    let mut archive = ZipArchive::new(file).map_err(|e| HandlerErr {
        code: "parse_failed",
        message: e.to_string(),
        details: Some(json!({ "path": path })),
    })?;
    let manifest_text = read_zip_text_entry(&mut archive, "manifest.json").ok_or_else(|| HandlerErr {
        code: "parse_failed",
        message: "missing manifest.json".to_string(),
        details: Some(json!({ "path": path })),
    })?;
    let manifest: Value = serde_json::from_str(&manifest_text).map_err(|e| HandlerErr {
        code: "parse_failed",
        message: e.to_string(),
        details: Some(json!({ "path": path, "entry": "manifest.json" })),
    })?;
    let format = manifest
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if format != ADMIN_TRANSFER_FORMAT {
        return Err(HandlerErr {
            code: "parse_failed",
            message: format!("unsupported package format: {}", format),
            details: Some(json!({ "path": path })),
        });
    }
    let students = read_zip_text_entry(&mut archive, "students.csv")
        .map(|t| parse_admin_students_csv(&t))
        .unwrap_or_default();

    let mut mark_sets = Vec::<SourceMarkSetPackage>::new();
    let mut markset_codes = HashSet::<String>::new();
    for i in 0..archive.len() {
        let Ok(file) = archive.by_index(i) else {
            continue;
        };
        let name = file.name().to_string();
        if name.starts_with("marksets/") && name.ends_with("/assessments.csv") {
            let parts = name.split('/').collect::<Vec<_>>();
            if parts.len() >= 3 {
                markset_codes.insert(parts[1].to_string());
            }
        }
    }

    for code in markset_codes {
        let assessments = read_zip_text_entry(
            &mut archive,
            format!("marksets/{}/assessments.csv", code).as_str(),
        )
        .map(|t| parse_admin_assessments_csv(&t))
        .unwrap_or_default();
        let scores = read_zip_text_entry(
            &mut archive,
            format!("marksets/{}/scores.csv", code).as_str(),
        )
        .map(|t| parse_admin_scores_csv(&t))
        .unwrap_or_default();
        let comment_sets = read_zip_text_entry(
            &mut archive,
            format!("comments/{}/sets.json", code).as_str(),
        )
        .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        .and_then(|v| v.get("sets").and_then(|s| s.as_array()).cloned())
        .unwrap_or_default()
        .into_iter()
        .filter_map(|set| {
            let set_number = set.get("setNumber").and_then(|v| v.as_i64())?;
            let title = set.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let max_chars = set.get("maxChars").and_then(|v| v.as_i64()).unwrap_or(600);
            let fit_width = set.get("fitWidth").and_then(|v| v.as_i64()).unwrap_or(50);
            let fit_lines = set.get("fitLines").and_then(|v| v.as_i64()).unwrap_or(1);
            let bank_short = set
                .get("bankShort")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let remarks = set
                .get("remarks")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .filter_map(|r| {
                    Some((
                        r.get("studentId")?.as_str()?.to_string(),
                        r.get("remark")?.as_str()?.to_string(),
                    ))
                })
                .collect::<Vec<_>>();
            Some(SourceCommentSetRow {
                set_number,
                title,
                max_chars,
                fit_width,
                fit_lines,
                bank_short,
                remarks,
            })
        })
        .collect::<Vec<_>>();

        mark_sets.push(SourceMarkSetPackage {
            code: code.clone(),
            description: code.clone(),
            assessments,
            scores,
            comment_sets,
        });
    }

    let learning_skills = read_zip_text_entry(&mut archive, "learning-skills/grid.csv")
        .map(|t| parse_admin_learning_skills_csv(&t))
        .unwrap_or_default();

    Ok(AdminTransferPackage {
        manifest,
        students,
        mark_sets,
        learning_skills,
    })
}

fn handle_admin_transfer_export_package(state: &mut AppState, req: &Request) -> Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    let class_id = match get_required_str(&req.params, "classId") {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let out_path = match get_required_str(&req.params, "outPath") {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let include_comments = req
        .params
        .get("includeComments")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let include_learning_skills = req
        .params
        .get("includeLearningSkills")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let class_name: Option<String> = conn
        .query_row(
            "SELECT name FROM classes WHERE id = ?",
            [&class_id],
            |r| r.get(0),
        )
        .optional()
        .ok()
        .flatten();
    let Some(class_name) = class_name else {
        return err(&req.id, "not_found", "class not found", None);
    };

    let selected_mark_set_ids = req
        .params
        .get("markSetIds")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let mark_set_sql = if selected_mark_set_ids.is_empty() {
        "SELECT id, code, description FROM mark_sets WHERE class_id = ? AND deleted_at IS NULL ORDER BY sort_order".to_string()
    } else {
        let ph = std::iter::repeat_n("?", selected_mark_set_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "SELECT id, code, description FROM mark_sets WHERE class_id = ? AND id IN ({}) ORDER BY sort_order",
            ph
        )
    };
    let mut params: Vec<&dyn rusqlite::ToSql> = vec![&class_id];
    for id in &selected_mark_set_ids {
        params.push(id as &dyn rusqlite::ToSql);
    }
    let mut mark_sets = Vec::<(String, String, String)>::new();
    if let Ok(mut stmt) = conn.prepare(&mark_set_sql) {
        if let Ok(rows) = stmt.query_map(params_from_iter(params), |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
        }) {
            mark_sets = rows.filter_map(Result::ok).collect::<Vec<_>>();
        }
    }

    let out = PathBuf::from(&out_path);
    if let Some(parent) = out.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return err(
                &req.id,
                "export_failed",
                e.to_string(),
                Some(json!({ "path": out_path })),
            );
        }
    }
    let out_file = match File::create(&out) {
        Ok(v) => v,
        Err(e) => {
            return err(
                &req.id,
                "export_failed",
                e.to_string(),
                Some(json!({ "path": out_path })),
            )
        }
    };
    let mut zip = ZipWriter::new(out_file);
    let opts = FileOptions::default().compression_method(CompressionMethod::Deflated);
    let exported_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let manifest = json!({
        "format": ADMIN_TRANSFER_FORMAT,
        "version": 1,
        "exportedAt": exported_at,
        "class": { "id": class_id, "name": class_name },
        "markSets": mark_sets.iter().map(|(_, code, description)| json!({ "code": code, "description": description })).collect::<Vec<_>>()
    });
    if zip.start_file("manifest.json", opts).is_err() {
        return err(&req.id, "export_failed", "failed to start manifest entry", None);
    }
    if zip
        .write_all(
            serde_json::to_string_pretty(&manifest)
                .unwrap_or_else(|_| "{}".to_string())
                .as_bytes(),
        )
        .is_err()
    {
        return err(&req.id, "export_failed", "failed to write manifest", None);
    }

    let mut students_csv = String::from(
        "student_id,student_no,last_name,first_name,birth_date,active,sort_order,mark_set_mask\n",
    );
    if let Ok(mut stmt) = conn.prepare(
        "SELECT id, student_no, last_name, first_name, birth_date, active, sort_order, COALESCE(mark_set_mask,'TBA')
         FROM students
         WHERE class_id = ?
         ORDER BY sort_order",
    ) {
        if let Ok(rows) = stmt.query_map([&class_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, i64>(6)?,
                r.get::<_, String>(7)?,
            ))
        }) {
            for row in rows.filter_map(Result::ok) {
                students_csv.push_str(&format!(
                    "{},{},{},{},{},{},{},{}\n",
                    csv_quote(&row.0),
                    csv_quote(row.1.as_deref().unwrap_or("")),
                    csv_quote(&row.2),
                    csv_quote(&row.3),
                    csv_quote(row.4.as_deref().unwrap_or("")),
                    row.5,
                    row.6,
                    csv_quote(&row.7)
                ));
            }
        }
    }
    if zip.start_file("students.csv", opts).is_err() || zip.write_all(students_csv.as_bytes()).is_err() {
        return err(&req.id, "export_failed", "failed to write students.csv", None);
    }

    let mut entries_written = 2usize;
    for (mark_set_id, code, description) in &mark_sets {
        let _ = description;
        let mut assessments_csv =
            String::from("idx,date,category_name,title,term,legacy_type,weight,out_of\n");
        if let Ok(mut stmt) = conn.prepare(
            "SELECT idx, date, category_name, title, term, legacy_type, weight, out_of
             FROM assessments
             WHERE mark_set_id = ?
             ORDER BY idx",
        ) {
            if let Ok(rows) = stmt.query_map([mark_set_id], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, Option<i64>>(4)?,
                    r.get::<_, Option<i64>>(5)?,
                    r.get::<_, Option<f64>>(6)?,
                    r.get::<_, Option<f64>>(7)?,
                ))
            }) {
                for row in rows.filter_map(Result::ok) {
                    assessments_csv.push_str(&format!(
                        "{},{},{},{},{},{},{},{}\n",
                        row.0,
                        csv_quote(row.1.as_deref().unwrap_or("")),
                        csv_quote(row.2.as_deref().unwrap_or("")),
                        csv_quote(&row.3),
                        row.4.map(|v| v.to_string()).unwrap_or_default(),
                        row.5.map(|v| v.to_string()).unwrap_or_default(),
                        row.6.map(|v| v.to_string()).unwrap_or_default(),
                        row.7.map(|v| v.to_string()).unwrap_or_default()
                    ));
                }
            }
        }
        let assessments_entry = format!("marksets/{}/assessments.csv", code);
        if zip.start_file(&assessments_entry, opts).is_err()
            || zip.write_all(assessments_csv.as_bytes()).is_err()
        {
            return err(&req.id, "export_failed", "failed to write assessments.csv", None);
        }
        entries_written += 1;

        let mut scores_csv =
            String::from("assessment_idx,student_id,status,raw_value,remark\n");
        if let Ok(mut stmt) = conn.prepare(
            "SELECT a.idx, sc.student_id, sc.status, sc.raw_value, sc.remark
             FROM scores sc
             JOIN assessments a ON a.id = sc.assessment_id
             WHERE a.mark_set_id = ?
             ORDER BY a.idx",
        ) {
            if let Ok(rows) = stmt.query_map([mark_set_id], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<f64>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                ))
            }) {
                for row in rows.filter_map(Result::ok) {
                    scores_csv.push_str(&format!(
                        "{},{},{},{},{}\n",
                        row.0,
                        csv_quote(&row.1),
                        csv_quote(&row.2),
                        row.3.map(|v| v.to_string()).unwrap_or_default(),
                        csv_quote(row.4.as_deref().unwrap_or(""))
                    ));
                }
            }
        }
        let scores_entry = format!("marksets/{}/scores.csv", code);
        if zip.start_file(&scores_entry, opts).is_err() || zip.write_all(scores_csv.as_bytes()).is_err() {
            return err(&req.id, "export_failed", "failed to write scores.csv", None);
        }
        entries_written += 1;

        if include_comments {
            let mut sets = Vec::<Value>::new();
            if let Ok(mut stmt) = conn.prepare(
                "SELECT id, set_number, title, max_chars, fit_width, fit_lines, bank_short
                 FROM comment_set_indexes
                 WHERE mark_set_id = ?
                 ORDER BY set_number",
            ) {
                if let Ok(rows) = stmt.query_map([mark_set_id], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, i64>(3)?,
                        r.get::<_, i64>(4)?,
                        r.get::<_, i64>(5)?,
                        r.get::<_, Option<String>>(6)?,
                    ))
                }) {
                    for row in rows.filter_map(Result::ok) {
                        let mut remarks = Vec::<Value>::new();
                        if let Ok(mut rstmt) = conn.prepare(
                            "SELECT student_id, remark FROM comment_set_remarks WHERE comment_set_index_id = ?",
                        ) {
                            if let Ok(rrows) = rstmt.query_map([&row.0], |rr| {
                                Ok((rr.get::<_, String>(0)?, rr.get::<_, String>(1)?))
                            }) {
                                for rr in rrows.filter_map(Result::ok) {
                                    remarks.push(json!({ "studentId": rr.0, "remark": rr.1 }));
                                }
                            }
                        }
                        sets.push(json!({
                            "setNumber": row.1,
                            "title": row.2,
                            "maxChars": row.3,
                            "fitWidth": row.4,
                            "fitLines": row.5,
                            "bankShort": row.6,
                            "remarks": remarks
                        }));
                    }
                }
            }
            let entry = format!("comments/{}/sets.json", code);
            let payload = serde_json::to_string_pretty(&json!({ "sets": sets }))
                .unwrap_or_else(|_| "{\"sets\":[]}".to_string());
            if zip.start_file(&entry, opts).is_err() || zip.write_all(payload.as_bytes()).is_err() {
                return err(&req.id, "export_failed", "failed to write comments set json", None);
            }
            entries_written += 1;
        }
    }

    if include_learning_skills {
        let mut csv = String::from("student_id,term,skill_code,value\n");
        if let Ok(mut stmt) = conn.prepare(
            "SELECT student_id, term, skill_code, value
             FROM learning_skills_cells
             WHERE class_id = ?
             ORDER BY student_id, term, skill_code",
        ) {
            if let Ok(rows) = stmt.query_map([&class_id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            }) {
                for row in rows.filter_map(Result::ok) {
                    csv.push_str(&format!(
                        "{},{},{},{}\n",
                        csv_quote(&row.0),
                        row.1,
                        csv_quote(&row.2),
                        csv_quote(&row.3)
                    ));
                }
            }
        }
        if zip.start_file("learning-skills/grid.csv", opts).is_err() || zip.write_all(csv.as_bytes()).is_err() {
            return err(&req.id, "export_failed", "failed to write learning skills grid", None);
        }
        entries_written += 1;
    }

    if zip.finish().is_err() {
        return err(&req.id, "export_failed", "failed to finalize transfer package", None);
    }

    ok(
        &req.id,
        json!({
            "ok": true,
            "entriesWritten": entries_written,
            "path": out_path,
            "format": ADMIN_TRANSFER_FORMAT
        }),
    )
}

fn handle_admin_transfer_preview_package(state: &mut AppState, req: &Request) -> Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    let target_class_id = match get_required_str(&req.params, "targetClassId") {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let in_path = match get_required_str(&req.params, "inPath") {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let match_mode = match parse_student_match_mode(&req.params, conn) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };

    let package = match parse_admin_package(&in_path) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let target_students = match list_students(conn, &target_class_id) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let mut by_student_no: HashMap<String, Vec<String>> = HashMap::new();
    let mut by_name: HashMap<String, Vec<String>> = HashMap::new();
    let by_sort = target_students
        .iter()
        .map(|s| s.id.clone())
        .collect::<Vec<_>>();
    for s in &target_students {
        if let Some(student_no) = s.student_no.as_deref().map(normalize_key) {
            by_student_no.entry(student_no).or_default().push(s.id.clone());
        }
        by_name
            .entry(normalized_name_key(&s.last_name, &s.first_name))
            .or_default()
            .push(s.id.clone());
    }
    let mut used = HashSet::<String>::new();
    let mut matched = 0usize;
    let mut unmatched = 0usize;
    let mut ambiguous = 0usize;
    let mut warnings = Vec::<Value>::new();
    for (i, src) in package.students.iter().enumerate() {
        let row = SisRosterRow {
            line_no: i + 2,
            student_no: src.student_no.clone(),
            last_name: src.last_name.clone(),
            first_name: src.first_name.clone(),
            birth_date: src.birth_date.clone(),
            active: Some(src.active),
        };
        let (matched_id, is_ambiguous) = find_student_match(
            &row,
            &match_mode,
            i,
            &by_student_no,
            &by_name,
            &by_sort,
            &used,
        );
        if is_ambiguous {
            ambiguous += 1;
            warnings.push(json!({
                "sourceStudentId": src.source_id,
                "code": "ambiguous_match",
                "message": "multiple target students matched source row"
            }));
            continue;
        }
        if let Some(target_id) = matched_id {
            used.insert(target_id);
            matched += 1;
        } else {
            unmatched += 1;
        }
    }

    let mut collisions = Vec::<Value>::new();
    for ms in &package.mark_sets {
        let target: Option<String> = conn
            .query_row(
                "SELECT id FROM mark_sets WHERE class_id = ? AND UPPER(code) = UPPER(?)",
                (&target_class_id, &ms.code),
                |r| r.get(0),
            )
            .optional()
            .ok()
            .flatten();
        if let Some(target_mark_set_id) = target {
            collisions.push(json!({
                "markSetCode": ms.code,
                "targetMarkSetId": target_mark_set_id,
                "assessmentCount": ms.assessments.len()
            }));
        }
    }

    ok(
        &req.id,
        json!({
            "metadata": package.manifest.get("class").cloned().unwrap_or_else(|| json!({})),
            "markSetCount": package.mark_sets.len(),
            "studentAlignment": {
                "sourceRows": package.students.len(),
                "targetRows": target_students.len(),
                "matched": matched,
                "unmatchedSource": unmatched,
                "ambiguous": ambiguous
            },
            "collisions": collisions,
            "warnings": warnings
        }),
    )
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
        None => match value {
            Some(v) if v > 0.0 => Ok((Some(v), "scored")),
            _ => Ok((Some(0.0), "no_mark")),
        },
    }
}

fn upsert_score(
    conn: &Connection,
    assessment_id: &str,
    student_id: &str,
    raw_value: Option<f64>,
    status: &str,
    remark: Option<&str>,
) -> Result<(), HandlerErr> {
    let score_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO scores(id, assessment_id, student_id, raw_value, status, remark)
         VALUES(?, ?, ?, ?, ?, ?)
         ON CONFLICT(assessment_id, student_id) DO UPDATE SET
           raw_value = excluded.raw_value,
           status = excluded.status,
           remark = excluded.remark",
        (&score_id, assessment_id, student_id, raw_value, status, remark),
    )
    .map_err(|e| HandlerErr {
        code: "db_update_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "scores" })),
    })?;
    Ok(())
}

fn ensure_target_mark_set(
    tx: &Connection,
    class_id: &str,
    code: &str,
    description: &str,
) -> Result<String, HandlerErr> {
    let existing: Option<(String, Option<String>)> = tx
        .query_row(
            "SELECT id, deleted_at FROM mark_sets WHERE class_id = ? AND UPPER(code) = UPPER(?) LIMIT 1",
            (class_id, code),
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    if let Some((id, deleted_at)) = existing {
        if deleted_at.is_some() {
            let _ = tx.execute("UPDATE mark_sets SET deleted_at = NULL WHERE id = ?", [&id]);
        }
        return Ok(id);
    }
    let sort_order: i64 = tx
        .query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM mark_sets WHERE class_id = ?",
            [class_id],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let mark_set_id = Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO mark_sets(
           id, class_id, code, file_prefix, description, weight, source_filename, sort_order,
           weight_method, calc_method, is_default
         ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        (
            &mark_set_id,
            class_id,
            code,
            code,
            description,
            100.0_f64,
            code,
            sort_order,
            1_i64,
            0_i64,
            0_i64,
        ),
    )
    .map_err(|e| HandlerErr {
        code: "db_insert_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "mark_sets" })),
    })?;
    Ok(mark_set_id)
}

fn assessment_collision_key(a: &SourceAssessmentRow) -> String {
    format!(
        "{}|{}|{}|{}",
        normalize_key(a.title.as_str()),
        normalize_key(a.date.as_deref().unwrap_or("")),
        normalize_key(a.category_name.as_deref().unwrap_or("")),
        a.term.unwrap_or(0)
    )
}

fn handle_admin_transfer_apply_package(state: &mut AppState, req: &Request) -> Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    let target_class_id = match get_required_str(&req.params, "targetClassId") {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let in_path = match get_required_str(&req.params, "inPath") {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let match_mode = match parse_student_match_mode(&req.params, conn) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let collision_policy = match parse_collision_policy(&req.params, conn) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let comment_policy = match parse_comment_policy(&req.params, conn) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let package = match parse_admin_package(&in_path) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };

    let target_students = match list_students(conn, &target_class_id) {
        Ok(v) => v,
        Err(e) => return e.response(&req.id),
    };
    let mut by_student_no: HashMap<String, Vec<String>> = HashMap::new();
    let mut by_name: HashMap<String, Vec<String>> = HashMap::new();
    let by_sort = target_students
        .iter()
        .map(|s| s.id.clone())
        .collect::<Vec<_>>();
    for s in &target_students {
        if let Some(student_no) = s.student_no.as_deref().map(normalize_key) {
            by_student_no.entry(student_no).or_default().push(s.id.clone());
        }
        by_name
            .entry(normalized_name_key(&s.last_name, &s.first_name))
            .or_default()
            .push(s.id.clone());
    }

    let tx = match conn.unchecked_transaction() {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let now = now_unix_string();
    let mut warnings = Vec::<Value>::new();
    let mut source_to_target_student = HashMap::<String, String>::new();
    let mut used_target_ids = HashSet::<String>::new();
    let mut created_students = 0usize;

    for (i, source) in package.students.iter().enumerate() {
        let row = SisRosterRow {
            line_no: i + 2,
            student_no: source.student_no.clone(),
            last_name: source.last_name.clone(),
            first_name: source.first_name.clone(),
            birth_date: source.birth_date.clone(),
            active: Some(source.active),
        };
        let (matched_id, ambiguous) = find_student_match(
            &row,
            &match_mode,
            i,
            &by_student_no,
            &by_name,
            &by_sort,
            &used_target_ids,
        );
        if ambiguous {
            warnings.push(json!({
                "sourceStudentId": source.source_id,
                "code": "ambiguous_match",
                "message": "ambiguous target student match"
            }));
            continue;
        }
        if let Some(target_student_id) = matched_id {
            source_to_target_student.insert(source.source_id.clone(), target_student_id.clone());
            used_target_ids.insert(target_student_id);
            continue;
        }
        let new_student_id = Uuid::new_v4().to_string();
        let next_sort: i64 = tx
            .query_row(
                "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM students WHERE class_id = ?",
                [&target_class_id],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if let Err(e) = tx.execute(
            "INSERT INTO students(id, class_id, last_name, first_name, student_no, birth_date, active, sort_order, raw_line, mark_set_mask, updated_at)
             VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                &new_student_id,
                &target_class_id,
                &source.last_name,
                &source.first_name,
                &source.student_no,
                &source.birth_date,
                if source.active { 1 } else { 0 },
                next_sort,
                format!(
                    "{},{},{},{},{}",
                    if source.active { 1 } else { 0 },
                    source.last_name,
                    source.first_name,
                    source.student_no.clone().unwrap_or_default(),
                    source.birth_date.clone().unwrap_or_default()
                ),
                &source.mark_set_mask,
                &now,
            ),
        ) {
            let _ = tx.rollback();
            return err(&req.id, "db_insert_failed", e.to_string(), None);
        }
        source_to_target_student.insert(source.source_id.clone(), new_student_id);
        created_students += 1;
    }

    let mut assessments_created = 0usize;
    let mut assessments_merged = 0usize;
    let mut scores_upserted = 0usize;
    let mut remarks_updated = 0usize;
    for source_mark_set in &package.mark_sets {
        let target_mark_set_id = match ensure_target_mark_set(
            &tx,
            &target_class_id,
            &source_mark_set.code,
            &source_mark_set.description,
        ) {
            Ok(v) => v,
            Err(e) => {
                let _ = tx.rollback();
                return e.response(&req.id);
            }
        };
        let existing_assessments = {
            let mut map = HashMap::<String, String>::new();
            if let Ok(mut stmt) = tx.prepare(
                "SELECT id, date, category_name, title, term
                 FROM assessments
                 WHERE mark_set_id = ?",
            ) {
                if let Ok(rows) = stmt.query_map([&target_mark_set_id], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, Option<String>>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, Option<i64>>(4)?,
                    ))
                }) {
                    for row in rows.filter_map(Result::ok) {
                        let key = format!(
                            "{}|{}|{}|{}",
                            normalize_key(&row.3),
                            normalize_key(row.1.as_deref().unwrap_or("")),
                            normalize_key(row.2.as_deref().unwrap_or("")),
                            row.4.unwrap_or(0)
                        );
                        map.insert(key, row.0);
                    }
                }
            }
            map
        };

        let mut assessment_id_by_source_idx = HashMap::<i64, String>::new();
        let mut stop_collision = false;
        for source_assessment in &source_mark_set.assessments {
            let key = assessment_collision_key(source_assessment);
            if collision_policy == "stop_on_collision" && existing_assessments.contains_key(&key) {
                stop_collision = true;
                warnings.push(json!({
                    "code": "collision_conflict",
                    "markSetCode": source_mark_set.code,
                    "assessmentTitle": source_assessment.title
                }));
                break;
            }
        }
        if stop_collision {
            let _ = tx.rollback();
            return err(
                &req.id,
                "collision_conflict",
                "assessment collision encountered with stop_on_collision policy",
                Some(json!({ "markSetCode": source_mark_set.code })),
            );
        }

        for source_assessment in &source_mark_set.assessments {
            let key = assessment_collision_key(source_assessment);
            if collision_policy == "merge_existing" {
                if let Some(existing_id) = existing_assessments.get(&key) {
                    assessment_id_by_source_idx.insert(source_assessment.idx, existing_id.clone());
                    assessments_merged += 1;
                    continue;
                }
            }

            let next_idx: i64 = tx
                .query_row(
                    "SELECT COALESCE(MAX(idx), -1) + 1 FROM assessments WHERE mark_set_id = ?",
                    [&target_mark_set_id],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            let assessment_id = Uuid::new_v4().to_string();
            if let Err(e) = tx.execute(
                "INSERT INTO assessments(id, mark_set_id, idx, date, category_name, title, term, legacy_type, weight, out_of, avg_percent, avg_raw)
                 VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    &assessment_id,
                    &target_mark_set_id,
                    next_idx,
                    &source_assessment.date,
                    &source_assessment.category_name,
                    &source_assessment.title,
                    &source_assessment.term,
                    &source_assessment.legacy_type,
                    &source_assessment.weight,
                    &source_assessment.out_of,
                    Option::<f64>::None,
                    Option::<f64>::None,
                ),
            ) {
                let _ = tx.rollback();
                return err(&req.id, "db_insert_failed", e.to_string(), None);
            }
            assessments_created += 1;
            assessment_id_by_source_idx.insert(source_assessment.idx, assessment_id);
        }

        for source_score in &source_mark_set.scores {
            let Some(target_assessment_id) =
                assessment_id_by_source_idx.get(&source_score.assessment_idx).cloned()
            else {
                warnings.push(json!({
                    "code": "missing_target_assessment",
                    "markSetCode": source_mark_set.code,
                    "assessmentIdx": source_score.assessment_idx
                }));
                continue;
            };
            let Some(target_student_id) =
                source_to_target_student.get(&source_score.student_id).cloned()
            else {
                warnings.push(json!({
                    "code": "missing_target_student",
                    "sourceStudentId": source_score.student_id
                }));
                continue;
            };
            let (resolved_raw, resolved_status) =
                match resolve_score_state(Some(source_score.status.as_str()), source_score.raw_value) {
                    Ok(v) => v,
                    Err(e) => {
                        warnings.push(json!({
                            "code": e.code,
                            "message": e.message,
                            "sourceStudentId": source_score.student_id,
                            "assessmentIdx": source_score.assessment_idx
                        }));
                        continue;
                    }
                };
            if let Err(e) = upsert_score(
                &tx,
                &target_assessment_id,
                &target_student_id,
                resolved_raw,
                resolved_status,
                source_score.remark.as_deref(),
            ) {
                let _ = tx.rollback();
                return e.response(&req.id);
            }
            scores_upserted += 1;
        }

        for source_set in &source_mark_set.comment_sets {
            let target_set_id: String = tx
                .query_row(
                    "SELECT id FROM comment_set_indexes WHERE class_id = ? AND mark_set_id = ? AND set_number = ?",
                    (&target_class_id, &target_mark_set_id, source_set.set_number),
                    |r| r.get(0),
                )
                .optional()
                .ok()
                .flatten()
                .unwrap_or_else(|| {
                    let new_id = Uuid::new_v4().to_string();
                    let _ = tx.execute(
                        "INSERT INTO comment_set_indexes(
                           id, class_id, mark_set_id, set_number, title, fit_mode, fit_font_size, fit_width, fit_lines, fit_subj, max_chars, is_default, bank_short
                         ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                        (
                            &new_id,
                            &target_class_id,
                            &target_mark_set_id,
                            source_set.set_number,
                            &source_set.title,
                            0_i64,
                            8_i64,
                            source_set.fit_width,
                            source_set.fit_lines,
                            "",
                            source_set.max_chars,
                            0_i64,
                            &source_set.bank_short,
                        ),
                    );
                    new_id
                });
            let mut current_remarks = HashMap::<String, String>::new();
            if let Ok(mut stmt) = tx.prepare(
                "SELECT student_id, remark FROM comment_set_remarks WHERE comment_set_index_id = ?",
            ) {
                if let Ok(rows) = stmt.query_map([&target_set_id], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                }) {
                    for row in rows.filter_map(Result::ok) {
                        current_remarks.insert(row.0, row.1);
                    }
                }
            }
            for (source_student_id, source_remark) in &source_set.remarks {
                let Some(target_student_id) = source_to_target_student.get(source_student_id).cloned() else {
                    continue;
                };
                let current = current_remarks
                    .get(&target_student_id)
                    .cloned()
                    .unwrap_or_default();
                let Some(next_text) = transfer_text_by_policy(source_remark, &current, &comment_policy, " ") else {
                    continue;
                };
                if next_text.trim() == current.trim() {
                    continue;
                }
                if next_text.trim().is_empty() {
                    let _ = tx.execute(
                        "DELETE FROM comment_set_remarks WHERE comment_set_index_id = ? AND student_id = ?",
                        (&target_set_id, &target_student_id),
                    );
                } else {
                    let row_id = Uuid::new_v4().to_string();
                    let _ = tx.execute(
                        "INSERT INTO comment_set_remarks(id, comment_set_index_id, student_id, remark)
                         VALUES(?, ?, ?, ?)
                         ON CONFLICT(comment_set_index_id, student_id) DO UPDATE SET remark = excluded.remark",
                        (&row_id, &target_set_id, &target_student_id, &next_text),
                    );
                }
                remarks_updated += 1;
            }
        }
    }

    for ls in &package.learning_skills {
        let Some(target_student_id) = source_to_target_student.get(&ls.source_student_id).cloned() else {
            continue;
        };
        let _ = tx.execute(
            "INSERT INTO learning_skills_cells(class_id, student_id, term, skill_code, value, updated_at)
             VALUES(?, ?, ?, ?, ?, ?)
             ON CONFLICT(class_id, student_id, term, skill_code) DO UPDATE SET
               value = excluded.value,
               updated_at = excluded.updated_at",
            (&target_class_id, &target_student_id, ls.term, &ls.skill_code, &ls.value, &now),
        );
    }

    if let Err(e) = tx.commit() {
        return err(&req.id, "db_update_failed", e.to_string(), None);
    }

    ok(
        &req.id,
        json!({
            "ok": true,
            "students": {
                "created": created_students
            },
            "assessments": {
                "created": assessments_created,
                "merged": assessments_merged
            },
            "scores": {
                "upserted": scores_upserted
            },
            "remarks": {
                "updated": remarks_updated
            },
            "warnings": warnings
        }),
    )
}

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<Value> {
    match req.method.as_str() {
        "integrations.sis.previewImport" => Some(handle_sis_preview_import(state, req)),
        "integrations.sis.applyImport" => Some(handle_sis_apply_import(state, req)),
        "integrations.sis.exportRoster" => Some(handle_sis_export_roster(state, req)),
        "integrations.sis.exportMarks" => Some(handle_sis_export_marks(state, req)),
        "integrations.adminTransfer.previewPackage" => {
            Some(handle_admin_transfer_preview_package(state, req))
        }
        "integrations.adminTransfer.applyPackage" => {
            Some(handle_admin_transfer_apply_package(state, req))
        }
        "integrations.adminTransfer.exportPackage" => {
            Some(handle_admin_transfer_export_package(state, req))
        }
        _ => None,
    }
}
