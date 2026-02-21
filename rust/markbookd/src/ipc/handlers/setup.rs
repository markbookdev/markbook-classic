use crate::db;
use crate::ipc::error::{err, ok};
use crate::ipc::types::{AppState, Request};
use serde_json::{json, Map, Value};

#[derive(Clone, Copy)]
enum SetupSection {
    Analysis,
    Attendance,
    Comments,
    Printer,
    Planner,
    CourseDescription,
    Reports,
    Security,
    Email,
}

impl SetupSection {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "analysis" => Some(Self::Analysis),
            "attendance" => Some(Self::Attendance),
            "comments" => Some(Self::Comments),
            "printer" => Some(Self::Printer),
            "planner" => Some(Self::Planner),
            "courseDescription" => Some(Self::CourseDescription),
            "reports" => Some(Self::Reports),
            "security" => Some(Self::Security),
            "email" => Some(Self::Email),
            _ => None,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Self::Analysis => "setup.analysis",
            Self::Attendance => "setup.attendance",
            Self::Comments => "setup.comments",
            Self::Printer => "setup.printer",
            Self::Planner => "setup.planner",
            Self::CourseDescription => "setup.courseDescription",
            Self::Reports => "setup.reports",
            Self::Security => "setup.security",
            Self::Email => "setup.email",
        }
    }
}

fn default_section(section: SetupSection) -> Value {
    match section {
        SetupSection::Analysis => json!({
            "defaultStudentScope": "valid",
            "showInactiveStudents": false,
            "showDeletedEntries": false,
            "histogramBins": 10,
            "defaultSortBy": "sortOrder",
            "defaultTopBottomCount": 5
        }),
        SetupSection::Attendance => json!({
            "schoolYearStartMonth": 9,
            "presentCode": "P",
            "absentCode": "A",
            "lateCode": "L",
            "excusedCode": "E",
            "tardyThresholdMinutes": 10
        }),
        SetupSection::Comments => json!({
            "defaultTransferPolicy": "fill_blank",
            "appendSeparator": " ",
            "enforceFit": true,
            "enforceMaxChars": true,
            "defaultMaxChars": 600
        }),
        SetupSection::Printer => json!({
            "fontScale": 100,
            "landscapeWideTables": true,
            "repeatHeaders": true,
            "showGeneratedAt": true,
            "defaultMarginMm": 12
        }),
        SetupSection::Planner => json!({
            "defaultLessonDurationMinutes": 75,
            "defaultPublishStatus": "draft",
            "showArchivedByDefault": false,
            "defaultUnitTitlePrefix": "Unit"
        }),
        SetupSection::CourseDescription => json!({
            "defaultPeriodMinutes": 75,
            "defaultPeriodsPerWeek": 5,
            "defaultTotalWeeks": 36,
            "includePolicyByDefault": true
        }),
        SetupSection::Reports => json!({
            "plannerHeaderStyle": "classic",
            "showGeneratedAt": true,
            "defaultStudentScope": "valid"
        }),
        SetupSection::Security => json!({
            "passwordEnabled": false,
            "passwordHint": null,
            "confirmDeletes": true,
            "autoLockMinutes": 0
        }),
        SetupSection::Email => json!({
            "enabled": false,
            "fromName": "",
            "replyTo": "",
            "subjectPrefix": "MarkBook",
            "defaultCc": ""
        }),
    }
}

fn as_object_mut(value: &mut Value) -> Result<&mut Map<String, Value>, String> {
    value
        .as_object_mut()
        .ok_or_else(|| "internal setup object must be a JSON object".to_string())
}

fn parse_bool(v: &Value, key: &str) -> Result<bool, String> {
    v.as_bool()
        .ok_or_else(|| format!("{} must be boolean", key))
}

fn parse_i64_range(v: &Value, key: &str, min: i64, max: i64) -> Result<i64, String> {
    let n = v
        .as_i64()
        .ok_or_else(|| format!("{} must be integer", key))?;
    if !(min..=max).contains(&n) {
        return Err(format!("{} must be in {}..={}", key, min, max));
    }
    Ok(n)
}

fn parse_string_max(v: &Value, key: &str, max_len: usize) -> Result<String, String> {
    let s = v.as_str().ok_or_else(|| format!("{} must be string", key))?;
    let s = s.trim();
    if s.len() > max_len {
        return Err(format!("{} length must be <= {}", key, max_len));
    }
    Ok(s.to_string())
}

fn parse_nullable_string_max(v: &Value, key: &str, max_len: usize) -> Result<Value, String> {
    if v.is_null() {
        return Ok(Value::Null);
    }
    let s = parse_string_max(v, key, max_len)?;
    Ok(Value::String(s))
}

fn merge_section_patch(
    section: SetupSection,
    current: &mut Value,
    patch: &Map<String, Value>,
) -> Result<(), String> {
    let obj = as_object_mut(current)?;
    for (k, v) in patch {
        match section {
            SetupSection::Analysis => match k.as_str() {
                "defaultStudentScope" => {
                    let s = parse_string_max(v, k, 16)?.to_ascii_lowercase();
                    if s != "all" && s != "active" && s != "valid" {
                        return Err("defaultStudentScope must be one of: all, active, valid".into());
                    }
                    obj.insert(k.clone(), Value::String(s));
                }
                "showInactiveStudents" | "showDeletedEntries" => {
                    obj.insert(k.clone(), Value::Bool(parse_bool(v, k)?));
                }
                "histogramBins" => {
                    obj.insert(k.clone(), Value::from(parse_i64_range(v, k, 4, 20)?));
                }
                "defaultSortBy" => {
                    let s = parse_string_max(v, k, 24)?.to_ascii_lowercase();
                    if s != "sortorder" && s != "displayname" && s != "finalmark" {
                        return Err(
                            "defaultSortBy must be one of: sortOrder, displayName, finalMark"
                                .into(),
                        );
                    }
                    let canonical = if s == "sortorder" {
                        "sortOrder"
                    } else if s == "displayname" {
                        "displayName"
                    } else {
                        "finalMark"
                    };
                    obj.insert(k.clone(), Value::String(canonical.to_string()));
                }
                "defaultTopBottomCount" => {
                    obj.insert(k.clone(), Value::from(parse_i64_range(v, k, 3, 20)?));
                }
                _ => return Err(format!("unknown analysis field: {}", k)),
            },
            SetupSection::Attendance => match k.as_str() {
                "schoolYearStartMonth" => {
                    obj.insert(k.clone(), Value::from(parse_i64_range(v, k, 1, 12)?));
                }
                "presentCode" | "absentCode" | "lateCode" | "excusedCode" => {
                    let s = parse_string_max(v, k, 8)?;
                    if s.is_empty() {
                        return Err(format!("{} must not be empty", k));
                    }
                    obj.insert(k.clone(), Value::String(s.to_ascii_uppercase()));
                }
                "tardyThresholdMinutes" => {
                    obj.insert(k.clone(), Value::from(parse_i64_range(v, k, 0, 120)?));
                }
                _ => return Err(format!("unknown attendance field: {}", k)),
            },
            SetupSection::Comments => match k.as_str() {
                "defaultTransferPolicy" => {
                    let p = parse_string_max(v, k, 24)?.to_ascii_lowercase();
                    if p != "replace"
                        && p != "append"
                        && p != "fill_blank"
                        && p != "source_if_longer"
                    {
                        return Err(
                            "defaultTransferPolicy must be one of: replace, append, fill_blank, source_if_longer"
                                .into(),
                        );
                    }
                    obj.insert(k.clone(), Value::String(p));
                }
                "appendSeparator" => {
                    let s = parse_string_max(v, k, 8)?;
                    obj.insert(k.clone(), Value::String(s));
                }
                "enforceFit" | "enforceMaxChars" => {
                    obj.insert(k.clone(), Value::Bool(parse_bool(v, k)?));
                }
                "defaultMaxChars" => {
                    obj.insert(k.clone(), Value::from(parse_i64_range(v, k, 80, 5000)?));
                }
                _ => return Err(format!("unknown comments field: {}", k)),
            },
            SetupSection::Printer => match k.as_str() {
                "fontScale" => {
                    obj.insert(k.clone(), Value::from(parse_i64_range(v, k, 60, 160)?));
                }
                "landscapeWideTables" | "repeatHeaders" | "showGeneratedAt" => {
                    obj.insert(k.clone(), Value::Bool(parse_bool(v, k)?));
                }
                "defaultMarginMm" => {
                    obj.insert(k.clone(), Value::from(parse_i64_range(v, k, 5, 30)?));
                }
                _ => return Err(format!("unknown printer field: {}", k)),
            },
            SetupSection::Planner => match k.as_str() {
                "defaultLessonDurationMinutes" => {
                    obj.insert(k.clone(), Value::from(parse_i64_range(v, k, 15, 240)?));
                }
                "defaultPublishStatus" => {
                    let p = parse_string_max(v, k, 16)?.to_ascii_lowercase();
                    if p != "draft" && p != "published" && p != "archived" {
                        return Err(
                            "defaultPublishStatus must be one of: draft, published, archived"
                                .into(),
                        );
                    }
                    obj.insert(k.clone(), Value::String(p));
                }
                "showArchivedByDefault" => {
                    obj.insert(k.clone(), Value::Bool(parse_bool(v, k)?));
                }
                "defaultUnitTitlePrefix" => {
                    obj.insert(k.clone(), Value::String(parse_string_max(v, k, 32)?));
                }
                _ => return Err(format!("unknown planner field: {}", k)),
            },
            SetupSection::CourseDescription => match k.as_str() {
                "defaultPeriodMinutes" => {
                    obj.insert(k.clone(), Value::from(parse_i64_range(v, k, 1, 300)?));
                }
                "defaultPeriodsPerWeek" => {
                    obj.insert(k.clone(), Value::from(parse_i64_range(v, k, 1, 14)?));
                }
                "defaultTotalWeeks" => {
                    obj.insert(k.clone(), Value::from(parse_i64_range(v, k, 1, 60)?));
                }
                "includePolicyByDefault" => {
                    obj.insert(k.clone(), Value::Bool(parse_bool(v, k)?));
                }
                _ => return Err(format!("unknown courseDescription field: {}", k)),
            },
            SetupSection::Reports => match k.as_str() {
                "plannerHeaderStyle" => {
                    let style = parse_string_max(v, k, 16)?.to_ascii_lowercase();
                    if style != "compact" && style != "classic" && style != "minimal" {
                        return Err(
                            "plannerHeaderStyle must be one of: compact, classic, minimal".into(),
                        );
                    }
                    obj.insert(k.clone(), Value::String(style));
                }
                "showGeneratedAt" => {
                    obj.insert(k.clone(), Value::Bool(parse_bool(v, k)?));
                }
                "defaultStudentScope" => {
                    let s = parse_string_max(v, k, 16)?.to_ascii_lowercase();
                    if s != "all" && s != "active" && s != "valid" {
                        return Err(
                            "defaultStudentScope must be one of: all, active, valid".into(),
                        );
                    }
                    obj.insert(k.clone(), Value::String(s));
                }
                _ => return Err(format!("unknown reports field: {}", k)),
            },
            SetupSection::Security => match k.as_str() {
                "passwordEnabled" | "confirmDeletes" => {
                    obj.insert(k.clone(), Value::Bool(parse_bool(v, k)?));
                }
                "passwordHint" => {
                    obj.insert(k.clone(), parse_nullable_string_max(v, k, 120)?);
                }
                "autoLockMinutes" => {
                    obj.insert(k.clone(), Value::from(parse_i64_range(v, k, 0, 240)?));
                }
                _ => return Err(format!("unknown security field: {}", k)),
            },
            SetupSection::Email => match k.as_str() {
                "enabled" => {
                    obj.insert(k.clone(), Value::Bool(parse_bool(v, k)?));
                }
                "fromName" => {
                    obj.insert(k.clone(), Value::String(parse_string_max(v, k, 120)?));
                }
                "replyTo" => {
                    obj.insert(k.clone(), Value::String(parse_string_max(v, k, 200)?));
                }
                "subjectPrefix" => {
                    obj.insert(k.clone(), Value::String(parse_string_max(v, k, 80)?));
                }
                "defaultCc" => {
                    obj.insert(k.clone(), Value::String(parse_string_max(v, k, 200)?));
                }
                _ => return Err(format!("unknown email field: {}", k)),
            },
        }
    }
    Ok(())
}

fn load_section(
    conn: &rusqlite::Connection,
    section: SetupSection,
) -> anyhow::Result<Value> {
    let mut current = default_section(section);
    if let Some(saved) = db::settings_get_json(conn, section.key())? {
        if let Some(saved_obj) = saved.as_object() {
            // Best-effort apply: malformed historical values should not block setup UI.
            let _ = merge_section_patch(section, &mut current, saved_obj);
        }
    }
    Ok(current)
}

fn handle_setup_get(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    let analysis = match load_section(conn, SetupSection::Analysis) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let attendance = match load_section(conn, SetupSection::Attendance) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let comments = match load_section(conn, SetupSection::Comments) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let printer = match load_section(conn, SetupSection::Printer) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let planner = match load_section(conn, SetupSection::Planner) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let course_description = match load_section(conn, SetupSection::CourseDescription) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let reports = match load_section(conn, SetupSection::Reports) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let security = match load_section(conn, SetupSection::Security) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let email = match load_section(conn, SetupSection::Email) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    ok(
        &req.id,
        json!({
            "analysis": analysis,
            "attendance": attendance,
            "comments": comments,
            "printer": printer,
            "planner": planner,
            "courseDescription": course_description,
            "reports": reports,
            "security": security,
            "email": email
        }),
    )
}

fn handle_setup_update(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    let Some(section_raw) = req.params.get("section").and_then(|v| v.as_str()) else {
        return err(&req.id, "bad_params", "missing section", None);
    };
    let Some(section) = SetupSection::parse(section_raw) else {
        return err(&req.id, "bad_params", "unknown section", None);
    };
    let Some(patch_obj) = req.params.get("patch").and_then(|v| v.as_object()) else {
        return err(&req.id, "bad_params", "patch must be an object", None);
    };

    let mut current = match load_section(conn, section) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    if let Err(msg) = merge_section_patch(section, &mut current, patch_obj) {
        return err(&req.id, "bad_params", msg, None);
    }
    if let Err(e) = db::settings_set_json(conn, section.key(), &current) {
        return err(&req.id, "db_update_failed", e.to_string(), None);
    }
    ok(&req.id, json!({ "ok": true }))
}

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "setup.get" => Some(handle_setup_get(state, req)),
        "setup.update" => Some(handle_setup_update(state, req)),
        _ => None,
    }
}
