use crate::ipc::error::{err, ok};
use crate::ipc::types::{AppState, Request};
use rusqlite::{Connection, OptionalExtension};
use serde_json::json;
use std::collections::{HashMap, HashSet};

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

fn normalize_day_codes(raw: &str, days: usize) -> String {
    let mut chars: Vec<char> = raw.chars().collect();
    if chars.len() < days {
        chars.extend(std::iter::repeat(' ').take(days - chars.len()));
    } else if chars.len() > days {
        chars.truncate(days);
    }
    chars.into_iter().collect()
}

fn seat_index_to_code(index: usize, seats_per_row: i64) -> i64 {
    let row = (index as i64) / seats_per_row.max(1);
    let col = (index as i64) % seats_per_row.max(1) + 1;
    row * 10 + col
}

fn seat_code_to_index(seat_code: i64, rows: i64, seats_per_row: i64) -> Option<usize> {
    if seat_code <= 0 {
        return None;
    }
    let row = seat_code / 10;
    let col = seat_code % 10;
    if row < 0 || row >= rows || col < 1 || col > seats_per_row {
        return None;
    }
    Some((row * seats_per_row + (col - 1)) as usize)
}

fn seating_get(conn: &Connection, params: &serde_json::Value) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    if !class_exists(conn, &class_id)? {
        return Err(HandlerErr {
            code: "not_found",
            message: "class not found".to_string(),
            details: None,
        });
    }
    let default_rows = 6_i64;
    let default_seats = 5_i64;
    let plan_row: Option<(i64, i64, String)> = conn
        .query_row(
            "SELECT rows, seats_per_row, blocked_mask FROM seating_plans WHERE class_id = ?",
            [&class_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let (rows, seats_per_row, blocked_mask) =
        plan_row.unwrap_or((default_rows, default_seats, "0".repeat(100)));
    let seat_count = ((rows.max(1) * seats_per_row.max(1)) as usize).max(1);
    let blocked = normalize_day_codes(&blocked_mask, 100);
    let blocked_codes: Vec<usize> = blocked
        .chars()
        .enumerate()
        .filter_map(|(i, ch)| if ch == '1' { Some(i + 1) } else { None })
        .collect();

    let students = list_students_for_class(conn, &class_id)?;
    let sort_by_student: HashMap<String, i64> = students
        .iter()
        .map(|s| (s.id.clone(), s.sort_order))
        .collect();
    let mut assignments: Vec<Option<i64>> = vec![None; seat_count];
    let mut stmt = conn
        .prepare(
            "SELECT student_id, seat_code
             FROM seating_assignments
             WHERE class_id = ?",
        )
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let rows_iter = stmt
        .query_map([&class_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    for (student_id, seat_code) in rows_iter {
        let Some(idx) = seat_code_to_index(seat_code, rows, seats_per_row) else {
            continue;
        };
        if idx >= assignments.len() {
            continue;
        }
        let Some(sort_order) = sort_by_student.get(&student_id).copied() else {
            continue;
        };
        assignments[idx] = Some(sort_order);
    }

    Ok(json!({
        "rows": rows,
        "seatsPerRow": seats_per_row,
        "blockedSeatCodes": blocked_codes,
        "assignments": assignments
    }))
}

fn seating_save(conn: &Connection, params: &serde_json::Value) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    let rows = params
        .get("rows")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| HandlerErr {
            code: "bad_params",
            message: "missing rows".to_string(),
            details: None,
        })?
        .max(1);
    let seats_per_row = params
        .get("seatsPerRow")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| HandlerErr {
            code: "bad_params",
            message: "missing seatsPerRow".to_string(),
            details: None,
        })?
        .max(1);
    let seat_count = (rows * seats_per_row) as usize;
    let assignments_json = params
        .get("assignments")
        .and_then(|v| v.as_array())
        .ok_or_else(|| HandlerErr {
            code: "bad_params",
            message: "missing assignments".to_string(),
            details: None,
        })?;

    let blocked_codes: Vec<usize> = match params.get("blockedSeatCodes") {
        Some(v) if v.is_string() => {
            let s = v.as_str().unwrap_or_default();
            s.chars()
                .enumerate()
                .filter_map(|(i, ch)| if ch == '1' { Some(i + 1) } else { None })
                .collect()
        }
        Some(v) => v
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_u64().map(|n| n as usize))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
        None => Vec::new(),
    };
    let mut blocked_mask_chars = vec!['0'; 100];
    for code in blocked_codes {
        if (1..=100).contains(&code) {
            blocked_mask_chars[code - 1] = '1';
        }
    }
    let blocked_mask: String = blocked_mask_chars.into_iter().collect();

    let students = list_students_for_class(conn, &class_id)?;
    let by_sort_order: HashMap<i64, String> = students
        .iter()
        .map(|s| (s.sort_order, s.id.clone()))
        .collect();

    let tx = conn.unchecked_transaction().map_err(|e| HandlerErr {
        code: "db_tx_failed",
        message: e.to_string(),
        details: None,
    })?;

    tx.execute(
        "INSERT INTO seating_plans(class_id, rows, seats_per_row, blocked_mask)
         VALUES(?, ?, ?, ?)
         ON CONFLICT(class_id) DO UPDATE SET
           rows = excluded.rows,
           seats_per_row = excluded.seats_per_row,
           blocked_mask = excluded.blocked_mask",
        (&class_id, rows, seats_per_row, &blocked_mask),
    )
    .map_err(|e| HandlerErr {
        code: "db_update_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "seating_plans" })),
    })?;
    tx.execute("DELETE FROM seating_assignments WHERE class_id = ?", [&class_id])
        .map_err(|e| HandlerErr {
            code: "db_delete_failed",
            message: e.to_string(),
            details: Some(json!({ "table": "seating_assignments" })),
        })?;

    let mut seen_students: HashSet<String> = HashSet::new();
    for (idx, v) in assignments_json.iter().enumerate() {
        if idx >= seat_count {
            break;
        }
        let Some(sort_order) = v.as_i64() else {
            continue;
        };
        let Some(student_id) = by_sort_order.get(&sort_order).cloned() else {
            continue;
        };
        if seen_students.contains(&student_id) {
            continue;
        }
        seen_students.insert(student_id.clone());
        tx.execute(
            "INSERT INTO seating_assignments(class_id, student_id, seat_code) VALUES(?, ?, ?)",
            (
                &class_id,
                &student_id,
                seat_index_to_code(idx, seats_per_row),
            ),
        )
        .map_err(|e| HandlerErr {
            code: "db_insert_failed",
            message: e.to_string(),
            details: Some(json!({ "table": "seating_assignments" })),
        })?;
    }
    tx.commit().map_err(|e| HandlerErr {
        code: "db_commit_failed",
        message: e.to_string(),
        details: None,
    })?;
    Ok(json!({ "ok": true }))
}

fn handle_seating_get(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match seating_get(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_seating_save(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match seating_save(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "seating.get" => Some(handle_seating_get(state, req)),
        "seating.save" => Some(handle_seating_save(state, req)),
        _ => None,
    }
}

