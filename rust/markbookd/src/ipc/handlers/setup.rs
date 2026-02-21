use crate::db;
use crate::ipc::error::{err, ok};
use crate::ipc::types::{AppState, Request};
use serde_json::{json, Map, Value};

#[derive(Clone, Copy)]
enum SetupSection {
    Analysis,
    Marks,
    Exchange,
    Analytics,
    Attendance,
    Comments,
    Printer,
    Integrations,
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
            "marks" => Some(Self::Marks),
            "exchange" => Some(Self::Exchange),
            "analytics" => Some(Self::Analytics),
            "attendance" => Some(Self::Attendance),
            "comments" => Some(Self::Comments),
            "printer" => Some(Self::Printer),
            "integrations" => Some(Self::Integrations),
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
            Self::Marks => "setup.marks",
            Self::Exchange => "setup.exchange",
            Self::Analytics => "setup.analytics",
            Self::Attendance => "setup.attendance",
            Self::Comments => "setup.comments",
            Self::Printer => "setup.printer",
            Self::Integrations => "setup.integrations",
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
        SetupSection::Marks => json!({
            "defaultHideDeletedEntries": true,
            "defaultAutoPreviewBeforeBulkApply": false
        }),
        SetupSection::Exchange => json!({
            "defaultExportStudentScope": "valid",
            "includeStateColumnsByDefault": true
        }),
        SetupSection::Analytics => json!({
            "defaultPageSize": 25,
            "defaultCohortMode": "none"
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
            "defaultSetNumber": 1,
            "defaultAppendSeparator": " ",
            "enforceMaxCharsByDefault": true,
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
            "defaultMarginMm": 12,
            "defaultPaperSize": "letter",
            "defaultOrientation": "portrait"
        }),
        SetupSection::Integrations => json!({
            "defaultSisProfile": "sis_roster_v1",
            "defaultMatchMode": "student_no_then_name",
            "defaultCollisionPolicy": "merge_existing",
            "autoPreviewBeforeApply": true,
            "adminTransferDefaultPolicy": "fill_blank"
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
            "defaultStudentScope": "valid",
            "defaultAnalyticsScope": "valid",
            "showFiltersInHeaderByDefault": true,
            "repeatHeadersByDefault": true,
            "defaultPageMargins": {
                "topMm": 12,
                "rightMm": 12,
                "bottomMm": 12,
                "leftMm": 12
            }
        }),
        SetupSection::Security => json!({
            "passwordEnabled": false,
            "requireWorkspacePassword": false,
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
            SetupSection::Marks => match k.as_str() {
                "defaultHideDeletedEntries" | "defaultAutoPreviewBeforeBulkApply" => {
                    obj.insert(k.clone(), Value::Bool(parse_bool(v, k)?));
                }
                _ => return Err(format!("unknown marks field: {}", k)),
            },
            SetupSection::Exchange => match k.as_str() {
                "defaultExportStudentScope" => {
                    let s = parse_string_max(v, k, 16)?.to_ascii_lowercase();
                    if s != "all" && s != "active" && s != "valid" {
                        return Err(
                            "defaultExportStudentScope must be one of: all, active, valid".into(),
                        );
                    }
                    obj.insert(k.clone(), Value::String(s));
                }
                "includeStateColumnsByDefault" => {
                    obj.insert(k.clone(), Value::Bool(parse_bool(v, k)?));
                }
                _ => return Err(format!("unknown exchange field: {}", k)),
            },
            SetupSection::Analytics => match k.as_str() {
                "defaultPageSize" => {
                    obj.insert(k.clone(), Value::from(parse_i64_range(v, k, 10, 200)?));
                }
                "defaultCohortMode" => {
                    let s = parse_string_max(v, k, 16)?.to_ascii_lowercase();
                    if s != "none" && s != "bin" && s != "threshold" {
                        return Err(
                            "defaultCohortMode must be one of: none, bin, threshold".into(),
                        );
                    }
                    obj.insert(k.clone(), Value::String(s));
                }
                _ => return Err(format!("unknown analytics field: {}", k)),
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
                "defaultSetNumber" => {
                    obj.insert(k.clone(), Value::from(parse_i64_range(v, k, 1, 20)?));
                }
                "appendSeparator" | "defaultAppendSeparator" => {
                    let s = parse_string_max(v, k, 8)?;
                    obj.insert("appendSeparator".to_string(), Value::String(s.clone()));
                    obj.insert("defaultAppendSeparator".to_string(), Value::String(s));
                }
                "enforceFit" => {
                    obj.insert(k.clone(), Value::Bool(parse_bool(v, k)?));
                }
                "enforceMaxChars" | "enforceMaxCharsByDefault" => {
                    let b = parse_bool(v, k)?;
                    obj.insert("enforceMaxChars".to_string(), Value::Bool(b));
                    obj.insert("enforceMaxCharsByDefault".to_string(), Value::Bool(b));
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
                "defaultPaperSize" => {
                    let s = parse_string_max(v, k, 16)?.to_ascii_lowercase();
                    if s != "letter" && s != "legal" && s != "a4" {
                        return Err(
                            "defaultPaperSize must be one of: letter, legal, a4".into(),
                        );
                    }
                    obj.insert(k.clone(), Value::String(s));
                }
                "defaultOrientation" => {
                    let s = parse_string_max(v, k, 16)?.to_ascii_lowercase();
                    if s != "portrait" && s != "landscape" {
                        return Err(
                            "defaultOrientation must be one of: portrait, landscape".into(),
                        );
                    }
                    obj.insert(k.clone(), Value::String(s));
                }
                _ => return Err(format!("unknown printer field: {}", k)),
            },
            SetupSection::Integrations => match k.as_str() {
                "defaultSisProfile" => {
                    let p = parse_string_max(v, k, 32)?.to_ascii_lowercase();
                    if p != "mb_exchange_v1" && p != "sis_roster_v1" && p != "sis_marks_v1" {
                        return Err(
                            "defaultSisProfile must be one of: mb_exchange_v1, sis_roster_v1, sis_marks_v1"
                                .into(),
                        );
                    }
                    obj.insert(k.clone(), Value::String(p));
                }
                "defaultMatchMode" => {
                    let m = parse_string_max(v, k, 32)?.to_ascii_lowercase();
                    if m != "student_no_then_name" && m != "name_only" && m != "sort_order" {
                        return Err(
                            "defaultMatchMode must be one of: student_no_then_name, name_only, sort_order"
                                .into(),
                        );
                    }
                    obj.insert(k.clone(), Value::String(m));
                }
                "defaultCollisionPolicy" => {
                    let p = parse_string_max(v, k, 32)?.to_ascii_lowercase();
                    if p != "merge_existing" && p != "append_new" && p != "stop_on_collision" {
                        return Err(
                            "defaultCollisionPolicy must be one of: merge_existing, append_new, stop_on_collision"
                                .into(),
                        );
                    }
                    obj.insert(k.clone(), Value::String(p));
                }
                "autoPreviewBeforeApply" => {
                    obj.insert(k.clone(), Value::Bool(parse_bool(v, k)?));
                }
                "adminTransferDefaultPolicy" => {
                    let p = parse_string_max(v, k, 24)?.to_ascii_lowercase();
                    if p != "replace"
                        && p != "append"
                        && p != "fill_blank"
                        && p != "source_if_longer"
                    {
                        return Err(
                            "adminTransferDefaultPolicy must be one of: replace, append, fill_blank, source_if_longer"
                                .into(),
                        );
                    }
                    obj.insert(k.clone(), Value::String(p));
                }
                _ => return Err(format!("unknown integrations field: {}", k)),
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
                "defaultAnalyticsScope" => {
                    let s = parse_string_max(v, k, 16)?.to_ascii_lowercase();
                    if s != "all" && s != "active" && s != "valid" {
                        return Err(
                            "defaultAnalyticsScope must be one of: all, active, valid".into(),
                        );
                    }
                    obj.insert(k.clone(), Value::String(s));
                }
                "showFiltersInHeaderByDefault" => {
                    obj.insert(k.clone(), Value::Bool(parse_bool(v, k)?));
                }
                "repeatHeadersByDefault" => {
                    obj.insert(k.clone(), Value::Bool(parse_bool(v, k)?));
                }
                "defaultPageMargins" => {
                    let Some(margins) = v.as_object() else {
                        return Err("defaultPageMargins must be an object".into());
                    };
                    let top = parse_i64_range(
                        margins
                            .get("topMm")
                            .ok_or_else(|| "defaultPageMargins.topMm is required".to_string())?,
                        "defaultPageMargins.topMm",
                        0,
                        40,
                    )?;
                    let right = parse_i64_range(
                        margins
                            .get("rightMm")
                            .ok_or_else(|| "defaultPageMargins.rightMm is required".to_string())?,
                        "defaultPageMargins.rightMm",
                        0,
                        40,
                    )?;
                    let bottom = parse_i64_range(
                        margins
                            .get("bottomMm")
                            .ok_or_else(|| "defaultPageMargins.bottomMm is required".to_string())?,
                        "defaultPageMargins.bottomMm",
                        0,
                        40,
                    )?;
                    let left = parse_i64_range(
                        margins
                            .get("leftMm")
                            .ok_or_else(|| "defaultPageMargins.leftMm is required".to_string())?,
                        "defaultPageMargins.leftMm",
                        0,
                        40,
                    )?;
                    obj.insert(
                        "defaultPageMargins".to_string(),
                        json!({
                            "topMm": top,
                            "rightMm": right,
                            "bottomMm": bottom,
                            "leftMm": left
                        }),
                    );
                }
                _ => return Err(format!("unknown reports field: {}", k)),
            },
            SetupSection::Security => match k.as_str() {
                "passwordEnabled" | "requireWorkspacePassword" => {
                    let enabled = parse_bool(v, k)?;
                    obj.insert("passwordEnabled".to_string(), Value::Bool(enabled));
                    obj.insert("requireWorkspacePassword".to_string(), Value::Bool(enabled));
                }
                "confirmDeletes" => {
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
    let marks = match load_section(conn, SetupSection::Marks) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let exchange = match load_section(conn, SetupSection::Exchange) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let analytics = match load_section(conn, SetupSection::Analytics) {
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
    let integrations = match load_section(conn, SetupSection::Integrations) {
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
            "marks": marks,
            "exchange": exchange,
            "analytics": analytics,
            "attendance": attendance,
            "comments": comments,
            "printer": printer,
            "integrations": integrations,
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
