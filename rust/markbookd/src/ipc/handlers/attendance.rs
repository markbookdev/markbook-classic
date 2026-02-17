use crate::ipc::error::{err, ok};
use crate::ipc::types::{AppState, Request};
use rusqlite::{Connection, OptionalExtension};
use serde_json::json;
use std::collections::HashMap;

struct HandlerErr {
    code: &'static str,
    message: String,
    details: Option<serde_json::Value>,
}

impl HandlerErr {
    fn response(self, id: &str) -> serde_json::Value {
        err(id, self.code, self.message, self.details)
    }
}

#[derive(Debug, Clone)]
struct BasicStudent {
    id: String,
    display_name: String,
    sort_order: i64,
    active: bool,
}

fn get_required_str(params: &serde_json::Value, key: &str) -> Result<String, HandlerErr> {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| HandlerErr {
            code: "bad_params",
            message: format!("missing {}", key),
            details: None,
        })
}

fn class_exists(conn: &Connection, class_id: &str) -> Result<bool, HandlerErr> {
    conn.query_row("SELECT 1 FROM classes WHERE id = ?", [class_id], |r| {
        r.get::<_, i64>(0)
    })
    .optional()
    .map(|v| v.is_some())
    .map_err(|e| HandlerErr {
        code: "db_query_failed",
        message: e.to_string(),
        details: None,
    })
}

fn list_students_for_class(conn: &Connection, class_id: &str) -> Result<Vec<BasicStudent>, HandlerErr> {
    let mut stmt = conn
        .prepare(
            "SELECT id, last_name, first_name, sort_order, active
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
        let last: String = r.get(1)?;
        let first: String = r.get(2)?;
        Ok(BasicStudent {
            id: r.get(0)?,
            display_name: format!("{}, {}", last, first),
            sort_order: r.get(3)?,
            active: r.get::<_, i64>(4)? != 0,
        })
    })
    .and_then(|it| it.collect::<Result<Vec<_>, _>>())
    .map_err(|e| HandlerErr {
        code: "db_query_failed",
        message: e.to_string(),
        details: None,
    })
}

fn parse_month_key(month: &str) -> Result<(i32, u32), HandlerErr> {
    let t = month.trim();
    if let Ok(m) = t.parse::<u32>() {
        if (1..=12).contains(&m) {
            return Ok((2001, m));
        }
    }
    let Some((y, m)) = t.split_once('-') else {
        return Err(HandlerErr {
            code: "bad_params",
            message: "month must be MM or YYYY-MM".to_string(),
            details: None,
        });
    };
    let year = y.parse::<i32>().map_err(|_| HandlerErr {
        code: "bad_params",
        message: "month year must be numeric".to_string(),
        details: None,
    })?;
    let month_num = m.parse::<u32>().map_err(|_| HandlerErr {
        code: "bad_params",
        message: "month must be YYYY-MM".to_string(),
        details: None,
    })?;
    if !(1..=12).contains(&month_num) {
        return Err(HandlerErr {
            code: "bad_params",
            message: "month must be between 01 and 12".to_string(),
            details: None,
        });
    }
    Ok((year, month_num))
}

fn days_in_month(year: i32, month: u32) -> usize {
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => 30,
    }
}

fn normalize_day_codes(raw: &str, days: usize) -> String {
    let mut chars: Vec<char> = raw.chars().collect();
    if chars.len() < days {
        chars.extend(std::iter::repeat(' ').take(days - chars.len()));
    } else if chars.len() > days {
        chars.truncate(days);
    }
    chars.into_iter().collect()
}

fn patch_day_code(existing: &str, days: usize, day: usize, code: Option<char>) -> String {
    let mut chars: Vec<char> = normalize_day_codes(existing, days).chars().collect();
    let idx = day.saturating_sub(1);
    if idx < chars.len() {
        chars[idx] = code.unwrap_or(' ');
    }
    chars.into_iter().collect()
}

fn parse_optional_code_char(v: Option<&serde_json::Value>) -> Result<Option<char>, HandlerErr> {
    let Some(v) = v else { return Ok(None) };
    if v.is_null() {
        return Ok(None);
    }
    let Some(s) = v.as_str() else {
        return Err(HandlerErr {
            code: "bad_params",
            message: "code must be string or null".to_string(),
            details: None,
        });
    };
    let t = s.trim();
    if t.is_empty() {
        return Ok(None);
    }
    Ok(t.chars().next())
}

fn attendance_month_open(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    let month_key = get_required_str(params, "month")?;
    let (year, month_num) = parse_month_key(&month_key)?;
    let days = days_in_month(year, month_num);

    if !class_exists(conn, &class_id)? {
        return Err(HandlerErr {
            code: "not_found",
            message: "class not found".to_string(),
            details: None,
        });
    }
    let students = list_students_for_class(conn, &class_id)?;
    let school_year_start_month: i64 = conn
        .query_row(
            "SELECT school_year_start_month FROM attendance_settings WHERE class_id = ?",
            [&class_id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?
        .unwrap_or(9);

    let type_of_day_codes_raw: Option<String> = conn
        .query_row(
            "SELECT type_of_day_codes FROM attendance_months WHERE class_id = ? AND month = ?",
            (&class_id, &month_key),
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let type_of_day_codes =
        normalize_day_codes(type_of_day_codes_raw.as_deref().unwrap_or(""), days);

    let mut by_student: HashMap<String, String> = HashMap::new();
    let mut stmt = conn
        .prepare(
            "SELECT student_id, day_codes
             FROM attendance_student_months
             WHERE class_id = ? AND month = ?",
        )
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let rows = stmt
        .query_map((&class_id, &month_key), |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    for (student_id, day_codes) in rows {
        by_student.insert(student_id, normalize_day_codes(&day_codes, days));
    }

    let students_json: Vec<serde_json::Value> = students
        .iter()
        .map(|s| {
            json!({
                "id": s.id,
                "displayName": s.display_name,
                "sortOrder": s.sort_order,
                "active": s.active
            })
        })
        .collect();
    let rows_json: Vec<serde_json::Value> = students
        .iter()
        .map(|s| {
            let day_codes = by_student
                .get(&s.id)
                .cloned()
                .unwrap_or_else(|| normalize_day_codes("", days));
            json!({
                "studentId": s.id,
                "dayCodes": day_codes
            })
        })
        .collect();

    Ok(json!({
        "schoolYearStartMonth": school_year_start_month,
        "month": month_key,
        "daysInMonth": days,
        "typeOfDayCodes": type_of_day_codes,
        "students": students_json,
        "rows": rows_json
    }))
}

fn attendance_set_type_of_day(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    let month_key = get_required_str(params, "month")?;
    let day = params
        .get("day")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| HandlerErr {
            code: "bad_params",
            message: "missing day".to_string(),
            details: None,
        })? as usize;
    let code = parse_optional_code_char(params.get("code"))?;
    let (year, month_num) = parse_month_key(&month_key)?;
    let days = days_in_month(year, month_num);
    if day == 0 || day > days {
        return Err(HandlerErr {
            code: "bad_params",
            message: "day out of range for month".to_string(),
            details: None,
        });
    }
    let existing: Option<String> = conn
        .query_row(
            "SELECT type_of_day_codes FROM attendance_months WHERE class_id = ? AND month = ?",
            (&class_id, &month_key),
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let patched = patch_day_code(existing.as_deref().unwrap_or(""), days, day, code);
    conn.execute(
        "INSERT INTO attendance_months(class_id, month, type_of_day_codes)
         VALUES(?, ?, ?)
         ON CONFLICT(class_id, month) DO UPDATE SET
           type_of_day_codes = excluded.type_of_day_codes",
        (&class_id, &month_key, &patched),
    )
    .map_err(|e| HandlerErr {
        code: "db_update_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "attendance_months" })),
    })?;
    Ok(json!({ "ok": true }))
}

fn attendance_set_student_day(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    let month_key = get_required_str(params, "month")?;
    let student_id = get_required_str(params, "studentId")?;
    let day = params
        .get("day")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| HandlerErr {
            code: "bad_params",
            message: "missing day".to_string(),
            details: None,
        })? as usize;
    let code = parse_optional_code_char(params.get("code"))?;
    let (year, month_num) = parse_month_key(&month_key)?;
    let days = days_in_month(year, month_num);
    if day == 0 || day > days {
        return Err(HandlerErr {
            code: "bad_params",
            message: "day out of range for month".to_string(),
            details: None,
        });
    }
    let student_exists = conn
        .query_row(
            "SELECT 1 FROM students WHERE class_id = ? AND id = ?",
            (&class_id, &student_id),
            |r| r.get::<_, i64>(0),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?
        .is_some();
    if !student_exists {
        return Err(HandlerErr {
            code: "not_found",
            message: "student not found".to_string(),
            details: None,
        });
    }
    let existing: Option<String> = conn
        .query_row(
            "SELECT day_codes FROM attendance_student_months WHERE class_id = ? AND student_id = ? AND month = ?",
            (&class_id, &student_id, &month_key),
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let patched = patch_day_code(existing.as_deref().unwrap_or(""), days, day, code);
    conn.execute(
        "INSERT INTO attendance_student_months(class_id, student_id, month, day_codes)
         VALUES(?, ?, ?, ?)
         ON CONFLICT(class_id, student_id, month) DO UPDATE SET
           day_codes = excluded.day_codes",
        (&class_id, &student_id, &month_key, &patched),
    )
    .map_err(|e| HandlerErr {
        code: "db_update_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "attendance_student_months" })),
    })?;
    Ok(json!({ "ok": true }))
}

fn attendance_bulk_stamp_day(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    let month_key = get_required_str(params, "month")?;
    let day = params
        .get("day")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| HandlerErr {
            code: "bad_params",
            message: "missing day".to_string(),
            details: None,
        })? as usize;
    let code = parse_optional_code_char(params.get("code"))?;
    let Some(student_ids_json) = params.get("studentIds").and_then(|v| v.as_array()) else {
        return Err(HandlerErr {
            code: "bad_params",
            message: "missing studentIds".to_string(),
            details: None,
        });
    };
    let student_ids: Vec<String> = student_ids_json
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();
    let (year, month_num) = parse_month_key(&month_key)?;
    let days = days_in_month(year, month_num);
    if day == 0 || day > days {
        return Err(HandlerErr {
            code: "bad_params",
            message: "day out of range for month".to_string(),
            details: None,
        });
    }

    let tx = conn.unchecked_transaction().map_err(|e| HandlerErr {
        code: "db_tx_failed",
        message: e.to_string(),
        details: None,
    })?;
    for student_id in student_ids {
        let exists = tx
            .query_row(
                "SELECT 1 FROM students WHERE class_id = ? AND id = ?",
                (&class_id, &student_id),
                |r| r.get::<_, i64>(0),
            )
            .optional()
            .map_err(|e| HandlerErr {
                code: "db_query_failed",
                message: e.to_string(),
                details: None,
            })?
            .is_some();
        if !exists {
            continue;
        }
        let existing: Option<String> = tx
            .query_row(
                "SELECT day_codes FROM attendance_student_months WHERE class_id = ? AND student_id = ? AND month = ?",
                (&class_id, &student_id, &month_key),
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| HandlerErr {
                code: "db_query_failed",
                message: e.to_string(),
                details: None,
            })?;
        let patched = patch_day_code(existing.as_deref().unwrap_or(""), days, day, code);
        tx.execute(
            "INSERT INTO attendance_student_months(class_id, student_id, month, day_codes)
             VALUES(?, ?, ?, ?)
             ON CONFLICT(class_id, student_id, month) DO UPDATE SET
               day_codes = excluded.day_codes",
            (&class_id, &student_id, &month_key, &patched),
        )
        .map_err(|e| HandlerErr {
            code: "db_update_failed",
            message: e.to_string(),
            details: Some(json!({ "table": "attendance_student_months" })),
        })?;
    }
    tx.commit().map_err(|e| HandlerErr {
        code: "db_commit_failed",
        message: e.to_string(),
        details: None,
    })?;
    Ok(json!({ "ok": true }))
}

fn handle_attendance_month_open(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match attendance_month_open(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_attendance_set_type_of_day(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match attendance_set_type_of_day(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_attendance_set_student_day(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match attendance_set_student_day(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_attendance_bulk_stamp_day(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match attendance_bulk_stamp_day(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "attendance.monthOpen" => Some(handle_attendance_month_open(state, req)),
        "attendance.setTypeOfDay" => Some(handle_attendance_set_type_of_day(state, req)),
        "attendance.setStudentDay" => Some(handle_attendance_set_student_day(state, req)),
        "attendance.bulkStampDay" => Some(handle_attendance_bulk_stamp_day(state, req)),
        _ => None,
    }
}

