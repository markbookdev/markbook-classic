use crate::ipc::error::{err, ok};
use crate::ipc::types::{AppState, Request};
use crate::legacy;
use rusqlite::types::Value;
use rusqlite::{params_from_iter, Connection, OptionalExtension};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use uuid::Uuid;

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

#[derive(Debug, Clone)]
struct StudentMatchRow {
    id: String,
    display_name: String,
    sort_order: i64,
    student_no: Option<String>,
    last_name: String,
    first_name: String,
}

#[derive(Debug, Clone)]
struct CommentSetFitMeta {
    set_id: String,
    max_chars: usize,
    fit_width: usize,
    fit_lines: usize,
    bank_short: Option<String>,
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

fn list_students_for_class(
    conn: &Connection,
    class_id: &str,
) -> Result<Vec<BasicStudent>, HandlerErr> {
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

fn mark_set_exists(
    conn: &Connection,
    class_id: &str,
    mark_set_id: &str,
) -> Result<bool, HandlerErr> {
    conn.query_row(
        "SELECT 1 FROM mark_sets WHERE id = ? AND class_id = ?",
        (mark_set_id, class_id),
        |r| r.get::<_, i64>(0),
    )
    .optional()
    .map(|v| v.is_some())
    .map_err(|e| HandlerErr {
        code: "db_query_failed",
        message: e.to_string(),
        details: None,
    })
}

fn list_student_match_rows(
    conn: &Connection,
    class_id: &str,
) -> Result<Vec<StudentMatchRow>, HandlerErr> {
    let mut stmt = conn
        .prepare(
            "SELECT id, last_name, first_name, sort_order, student_no
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
        let last_name: String = r.get(1)?;
        let first_name: String = r.get(2)?;
        Ok(StudentMatchRow {
            id: r.get(0)?,
            display_name: format!("{}, {}", last_name, first_name),
            sort_order: r.get(3)?,
            student_no: r.get(4)?,
            last_name,
            first_name,
        })
    })
    .and_then(|it| it.collect::<Result<Vec<_>, _>>())
    .map_err(|e| HandlerErr {
        code: "db_query_failed",
        message: e.to_string(),
        details: None,
    })
}

fn load_comment_set_fit_meta(
    conn: &Connection,
    class_id: &str,
    mark_set_id: &str,
    set_number: i64,
) -> Result<CommentSetFitMeta, HandlerErr> {
    let row: Option<CommentSetFitMeta> = conn
        .query_row(
            "SELECT id, max_chars, fit_width, fit_lines, bank_short
             FROM comment_set_indexes
             WHERE class_id = ? AND mark_set_id = ? AND set_number = ?",
            (class_id, mark_set_id, set_number),
            |r| {
                let max_chars: i64 = r.get(1)?;
                let fit_width: i64 = r.get(2)?;
                let fit_lines: i64 = r.get(3)?;
                Ok(CommentSetFitMeta {
                    set_id: r.get(0)?,
                    max_chars: max_chars.max(0) as usize,
                    fit_width: fit_width.max(0) as usize,
                    fit_lines: fit_lines.max(0) as usize,
                    bank_short: r.get(4)?,
                })
            },
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    row.ok_or_else(|| HandlerErr {
        code: "not_found",
        message: "comment set not found".to_string(),
        details: None,
    })
}

fn load_remarks_for_set(
    conn: &Connection,
    set_id: &str,
) -> Result<HashMap<String, String>, HandlerErr> {
    let mut stmt = conn
        .prepare(
            "SELECT student_id, remark
             FROM comment_set_remarks
             WHERE comment_set_index_id = ?",
        )
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let rows = stmt
        .query_map([set_id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    Ok(HashMap::from_iter(rows))
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

fn parse_fit_profile(profile: &str) -> Option<(usize, usize)> {
    let mut nums = Vec::new();
    let mut cur = String::new();
    for ch in profile.chars() {
        if ch.is_ascii_digit() {
            cur.push(ch);
        } else if !cur.is_empty() {
            if let Ok(v) = cur.parse::<usize>() {
                nums.push(v);
            }
            cur.clear();
        }
    }
    if !cur.is_empty() {
        if let Ok(v) = cur.parse::<usize>() {
            nums.push(v);
        }
    }
    if nums.len() >= 2 && nums[0] > 0 && nums[1] > 0 {
        Some((nums[0], nums[1]))
    } else {
        None
    }
}

fn resolve_effective_fit_constraints(
    conn: &Connection,
    meta: &CommentSetFitMeta,
) -> Result<(usize, usize, usize), HandlerErr> {
    let mut fit_width = meta.fit_width;
    let mut fit_lines = meta.fit_lines;
    if let Some(bank_short) = meta.bank_short.as_deref().and_then(non_empty_trimmed) {
        let profile: Option<Option<String>> = conn
            .query_row(
                "SELECT fit_profile FROM comment_banks WHERE UPPER(short_name) = UPPER(?)",
                [&bank_short],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| HandlerErr {
                code: "db_query_failed",
                message: e.to_string(),
                details: None,
            })?;
        if let Some(Some(p)) = profile {
            if let Some((w, l)) = parse_fit_profile(&p) {
                fit_width = w;
                fit_lines = l;
            }
        }
    }
    Ok((meta.max_chars.max(1), fit_width, fit_lines))
}

fn truncate_chars(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}

fn apply_fit_constraints(
    text: &str,
    max_chars: usize,
    fit_width: usize,
    fit_lines: usize,
) -> (String, bool) {
    let mut out = text.trim().to_string();
    let mut truncated = false;

    if max_chars > 0 && out.chars().count() > max_chars {
        out = truncate_chars(&out, max_chars);
        truncated = true;
    }

    if fit_width > 0 && fit_lines > 0 {
        let cap = fit_width.saturating_mul(fit_lines);
        if cap > 0 && out.chars().count() > cap {
            out = truncate_chars(&out, cap);
            truncated = true;
        }
    }

    (out, truncated)
}

fn choose_transfer_target(
    source: &StudentMatchRow,
    used_targets: &HashSet<String>,
    by_student_no: &HashMap<String, Vec<String>>,
    by_name: &HashMap<String, Vec<String>>,
    mode: &str,
) -> Option<String> {
    let pick_unique = |candidates: Option<&Vec<String>>| -> Option<String> {
        let values = candidates?;
        let mut free = values
            .iter()
            .filter(|id| !used_targets.contains(id.as_str()))
            .cloned()
            .collect::<Vec<_>>();
        free.dedup();
        if free.len() == 1 {
            free.into_iter().next()
        } else {
            None
        }
    };

    if mode == "student_no_then_name" {
        if let Some(student_no) = source.student_no.as_deref().and_then(non_empty_trimmed) {
            if let Some(id) = pick_unique(by_student_no.get(normalize_key(&student_no).as_str())) {
                return Some(id);
            }
        }
    }

    let name_key = normalized_name_key(&source.last_name, &source.first_name);
    pick_unique(by_name.get(name_key.as_str()))
}

fn transfer_text_by_policy(
    source: &str,
    target: &str,
    policy: &str,
    separator: &str,
) -> Option<String> {
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

fn comments_sets_list(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    let mark_set_id = get_required_str(params, "markSetId")?;
    if !mark_set_exists(conn, &class_id, &mark_set_id)? {
        return Err(HandlerErr {
            code: "not_found",
            message: "mark set not found".to_string(),
            details: None,
        });
    }
    let mut stmt = conn
        .prepare(
            "SELECT set_number, title, fit_mode, fit_font_size, fit_width, fit_lines, fit_subj, max_chars, is_default, bank_short
             FROM comment_set_indexes
             WHERE class_id = ? AND mark_set_id = ?
             ORDER BY set_number",
        )
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let sets = stmt
        .query_map((&class_id, &mark_set_id), |r| {
            Ok(json!({
                "setNumber": r.get::<_, i64>(0)?,
                "title": r.get::<_, String>(1)?,
                "fitMode": r.get::<_, i64>(2)?,
                "fitFontSize": r.get::<_, i64>(3)?,
                "fitWidth": r.get::<_, i64>(4)?,
                "fitLines": r.get::<_, i64>(5)?,
                "fitSubj": r.get::<_, String>(6)?,
                "maxChars": r.get::<_, i64>(7)?,
                "isDefault": r.get::<_, i64>(8)? != 0,
                "bankShort": r.get::<_, Option<String>>(9)?
            }))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    Ok(json!({ "sets": sets }))
}

fn comments_sets_open(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    let mark_set_id = get_required_str(params, "markSetId")?;
    let set_number = params
        .get("setNumber")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| HandlerErr {
            code: "bad_params",
            message: "missing setNumber".to_string(),
            details: None,
        })?;
    let set_row: Option<(String, i64, String, i64, i64, i64, i64, String, i64, i64, Option<String>)> = conn
        .query_row(
            "SELECT id, set_number, title, fit_mode, fit_font_size, fit_width, fit_lines, fit_subj, max_chars, is_default, bank_short
             FROM comment_set_indexes
             WHERE class_id = ? AND mark_set_id = ? AND set_number = ?",
            (&class_id, &mark_set_id, set_number),
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
                    r.get(9)?,
                    r.get(10)?,
                ))
            },
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let Some((
        set_id,
        set_number,
        title,
        fit_mode,
        fit_font_size,
        fit_width,
        fit_lines,
        fit_subj,
        max_chars,
        is_default,
        bank_short,
    )) = set_row
    else {
        return Err(HandlerErr {
            code: "not_found",
            message: "comment set not found".to_string(),
            details: None,
        });
    };

    let students = list_students_for_class(conn, &class_id)?;
    let mut remark_by_student: HashMap<String, String> = HashMap::new();
    let mut stmt = conn
        .prepare(
            "SELECT student_id, remark FROM comment_set_remarks WHERE comment_set_index_id = ?",
        )
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let rows = stmt
        .query_map([&set_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    for (sid, remark) in rows {
        remark_by_student.insert(sid, remark);
    }
    let remarks_by_student: Vec<serde_json::Value> = students
        .iter()
        .map(|s| {
            json!({
                "studentId": s.id,
                "displayName": s.display_name,
                "sortOrder": s.sort_order,
                "active": s.active,
                "remark": remark_by_student.get(&s.id).cloned().unwrap_or_default(),
            })
        })
        .collect();

    Ok(json!({
        "set": {
            "id": set_id,
            "setNumber": set_number,
            "title": title,
            "fitMode": fit_mode,
            "fitFontSize": fit_font_size,
            "fitWidth": fit_width,
            "fitLines": fit_lines,
            "fitSubj": fit_subj,
            "maxChars": max_chars,
            "isDefault": is_default != 0,
            "bankShort": bank_short
        },
        "remarksByStudent": remarks_by_student
    }))
}

fn parse_remarks_by_student(
    raw: Option<&serde_json::Value>,
) -> Result<Vec<(String, String)>, HandlerErr> {
    let Some(raw) = raw else {
        return Ok(Vec::new());
    };
    if let Some(arr) = raw.as_array() {
        let mut out = Vec::new();
        for item in arr {
            let Some(student_id) = item.get("studentId").and_then(|v| v.as_str()) else {
                continue;
            };
            let remark = item
                .get("remark")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            out.push((student_id.to_string(), remark));
        }
        return Ok(out);
    }
    if let Some(map) = raw.as_object() {
        let mut out = Vec::new();
        for (student_id, value) in map {
            let remark = value.as_str().unwrap_or("").to_string();
            out.push((student_id.clone(), remark));
        }
        return Ok(out);
    }
    Err(HandlerErr {
        code: "bad_params",
        message: "remarksByStudent must be array or object".to_string(),
        details: None,
    })
}

fn comments_sets_upsert(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    let mark_set_id = get_required_str(params, "markSetId")?;
    if !mark_set_exists(conn, &class_id, &mark_set_id)? {
        return Err(HandlerErr {
            code: "not_found",
            message: "mark set not found".to_string(),
            details: None,
        });
    }
    let title = params
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Comment Set")
        .trim()
        .to_string();
    let fit_mode = params.get("fitMode").and_then(|v| v.as_i64()).unwrap_or(0);
    let fit_font_size = params
        .get("fitFontSize")
        .and_then(|v| v.as_i64())
        .unwrap_or(9);
    let fit_width = params
        .get("fitWidth")
        .and_then(|v| v.as_i64())
        .unwrap_or(83);
    let fit_lines = params
        .get("fitLines")
        .and_then(|v| v.as_i64())
        .unwrap_or(12);
    let fit_subj = params
        .get("fitSubj")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let max_chars = params
        .get("maxChars")
        .and_then(|v| v.as_i64())
        .unwrap_or(100)
        .max(100);
    let is_default = params
        .get("isDefault")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let bank_short = params
        .get("bankShort")
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .and_then(|s| if s.is_empty() { None } else { Some(s) });
    let requested_set_number = params.get("setNumber").and_then(|v| v.as_i64());
    let remarks_by_student = parse_remarks_by_student(params.get("remarksByStudent"))?;

    let tx = conn.unchecked_transaction().map_err(|e| HandlerErr {
        code: "db_tx_failed",
        message: e.to_string(),
        details: None,
    })?;

    let set_number = if let Some(v) = requested_set_number {
        v.max(1)
    } else {
        tx.query_row(
            "SELECT COALESCE(MAX(set_number), 0) + 1 FROM comment_set_indexes WHERE mark_set_id = ?",
            [&mark_set_id],
            |r| r.get::<_, i64>(0),
        )
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?
    };

    if is_default {
        tx.execute(
            "UPDATE comment_set_indexes SET is_default = 0 WHERE mark_set_id = ?",
            [&mark_set_id],
        )
        .map_err(|e| HandlerErr {
            code: "db_update_failed",
            message: e.to_string(),
            details: Some(json!({ "table": "comment_set_indexes" })),
        })?;
    }

    let existing_id: Option<String> = tx
        .query_row(
            "SELECT id FROM comment_set_indexes WHERE mark_set_id = ? AND set_number = ?",
            (&mark_set_id, set_number),
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let set_id = existing_id.unwrap_or_else(|| Uuid::new_v4().to_string());
    tx.execute(
        "INSERT INTO comment_set_indexes(
           id, class_id, mark_set_id, set_number, title, fit_mode, fit_font_size, fit_width, fit_lines, fit_subj, max_chars, is_default, bank_short
         ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(mark_set_id, set_number) DO UPDATE SET
           title = excluded.title,
           fit_mode = excluded.fit_mode,
           fit_font_size = excluded.fit_font_size,
           fit_width = excluded.fit_width,
           fit_lines = excluded.fit_lines,
           fit_subj = excluded.fit_subj,
           max_chars = excluded.max_chars,
           is_default = excluded.is_default,
           bank_short = excluded.bank_short",
        (
            &set_id,
            &class_id,
            &mark_set_id,
            set_number,
            &title,
            fit_mode,
            fit_font_size,
            fit_width,
            fit_lines,
            &fit_subj,
            max_chars,
            if is_default { 1 } else { 0 },
            bank_short.as_deref(),
        ),
    )
    .map_err(|e| HandlerErr {
        code: "db_insert_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "comment_set_indexes" })),
    })?;

    for (student_id, remark) in remarks_by_student {
        let trimmed = remark.trim().to_string();
        if trimmed.is_empty() {
            tx.execute(
                "DELETE FROM comment_set_remarks WHERE comment_set_index_id = ? AND student_id = ?",
                (&set_id, &student_id),
            )
            .map_err(|e| HandlerErr {
                code: "db_delete_failed",
                message: e.to_string(),
                details: Some(json!({ "table": "comment_set_remarks" })),
            })?;
            continue;
        }
        let remark_id = Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO comment_set_remarks(id, comment_set_index_id, student_id, remark)
             VALUES(?, ?, ?, ?)
             ON CONFLICT(comment_set_index_id, student_id) DO UPDATE SET
               remark = excluded.remark",
            (&remark_id, &set_id, &student_id, &trimmed),
        )
        .map_err(|e| HandlerErr {
            code: "db_insert_failed",
            message: e.to_string(),
            details: Some(json!({ "table": "comment_set_remarks" })),
        })?;
    }

    tx.commit().map_err(|e| HandlerErr {
        code: "db_commit_failed",
        message: e.to_string(),
        details: None,
    })?;
    Ok(json!({ "setNumber": set_number }))
}

fn comments_sets_delete(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    let mark_set_id = get_required_str(params, "markSetId")?;
    let set_number = params
        .get("setNumber")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| HandlerErr {
            code: "bad_params",
            message: "missing setNumber".to_string(),
            details: None,
        })?;
    let tx = conn.unchecked_transaction().map_err(|e| HandlerErr {
        code: "db_tx_failed",
        message: e.to_string(),
        details: None,
    })?;
    let set_id: Option<String> = tx
        .query_row(
            "SELECT id FROM comment_set_indexes WHERE class_id = ? AND mark_set_id = ? AND set_number = ?",
            (&class_id, &mark_set_id, set_number),
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let Some(set_id) = set_id else {
        return Err(HandlerErr {
            code: "not_found",
            message: "comment set not found".to_string(),
            details: None,
        });
    };
    tx.execute(
        "DELETE FROM comment_set_remarks WHERE comment_set_index_id = ?",
        [&set_id],
    )
    .map_err(|e| HandlerErr {
        code: "db_delete_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "comment_set_remarks" })),
    })?;
    tx.execute("DELETE FROM comment_set_indexes WHERE id = ?", [&set_id])
        .map_err(|e| HandlerErr {
            code: "db_delete_failed",
            message: e.to_string(),
            details: Some(json!({ "table": "comment_set_indexes" })),
        })?;
    tx.commit().map_err(|e| HandlerErr {
        code: "db_commit_failed",
        message: e.to_string(),
        details: None,
    })?;
    Ok(json!({ "ok": true }))
}

fn comments_remarks_upsert_one(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    let mark_set_id = get_required_str(params, "markSetId")?;
    let set_number = params
        .get("setNumber")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| HandlerErr {
            code: "bad_params",
            message: "missing setNumber".to_string(),
            details: None,
        })?;
    let student_id = get_required_str(params, "studentId")?;
    let remark = params
        .get("remark")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .to_string();

    if !mark_set_exists(conn, &class_id, &mark_set_id)? {
        return Err(HandlerErr {
            code: "not_found",
            message: "mark set not found".to_string(),
            details: None,
        });
    }

    let student_exists: Option<i64> = conn
        .query_row(
            "SELECT 1 FROM students WHERE class_id = ? AND id = ?",
            (&class_id, &student_id),
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    if student_exists.is_none() {
        return Err(HandlerErr {
            code: "not_found",
            message: "student not found".to_string(),
            details: None,
        });
    }

    let set_id: Option<String> = conn
        .query_row(
            "SELECT id
             FROM comment_set_indexes
             WHERE class_id = ? AND mark_set_id = ? AND set_number = ?",
            (&class_id, &mark_set_id, set_number),
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let Some(set_id) = set_id else {
        return Err(HandlerErr {
            code: "not_found",
            message: "comment set not found".to_string(),
            details: None,
        });
    };

    if remark.is_empty() {
        conn.execute(
            "DELETE FROM comment_set_remarks WHERE comment_set_index_id = ? AND student_id = ?",
            (&set_id, &student_id),
        )
        .map_err(|e| HandlerErr {
            code: "db_delete_failed",
            message: e.to_string(),
            details: Some(json!({ "table": "comment_set_remarks" })),
        })?;
    } else {
        let remark_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO comment_set_remarks(id, comment_set_index_id, student_id, remark)
             VALUES(?, ?, ?, ?)
             ON CONFLICT(comment_set_index_id, student_id) DO UPDATE SET
               remark = excluded.remark",
            (&remark_id, &set_id, &student_id, &remark),
        )
        .map_err(|e| HandlerErr {
            code: "db_insert_failed",
            message: e.to_string(),
            details: Some(json!({ "table": "comment_set_remarks" })),
        })?;
    }

    Ok(json!({ "ok": true }))
}

fn parse_student_match_mode(params: &serde_json::Value) -> Result<String, HandlerErr> {
    let mode = params
        .get("studentMatchMode")
        .and_then(|v| v.as_str())
        .unwrap_or("student_no_then_name")
        .trim()
        .to_ascii_lowercase();
    if mode == "student_no_then_name" || mode == "name_only" {
        Ok(mode)
    } else {
        Err(HandlerErr {
            code: "bad_params",
            message: "studentMatchMode must be student_no_then_name or name_only".to_string(),
            details: None,
        })
    }
}

fn parse_transfer_policy(params: &serde_json::Value) -> Result<String, HandlerErr> {
    let policy = params
        .get("policy")
        .and_then(|v| v.as_str())
        .unwrap_or("fill_blank")
        .trim()
        .to_ascii_lowercase();
    if ["replace", "append", "fill_blank", "source_if_longer"]
        .contains(&policy.as_str())
    {
        Ok(policy)
    } else {
        Err(HandlerErr {
            code: "bad_params",
            message: "policy must be replace|append|fill_blank|source_if_longer".to_string(),
            details: None,
        })
    }
}

fn parse_transfer_scope(
    params: &serde_json::Value,
) -> Result<(&'static str, HashSet<String>), HandlerErr> {
    let scope = params
        .get("targetScope")
        .and_then(|v| v.as_str())
        .unwrap_or("all_matched")
        .trim()
        .to_ascii_lowercase();
    let mut selected = HashSet::new();
    if scope == "selected_target_students" {
        let Some(arr) = params
            .get("selectedTargetStudentIds")
            .and_then(|v| v.as_array())
        else {
            return Err(HandlerErr {
                code: "bad_params",
                message:
                    "selectedTargetStudentIds must be provided for targetScope=selected_target_students"
                        .to_string(),
                details: None,
            });
        };
        for v in arr {
            if let Some(s) = v.as_str() {
                if !s.trim().is_empty() {
                    selected.insert(s.trim().to_string());
                }
            }
        }
        Ok(("selected_target_students", selected))
    } else if scope == "all_matched" {
        Ok(("all_matched", selected))
    } else {
        Err(HandlerErr {
            code: "bad_params",
            message: "targetScope must be all_matched or selected_target_students".to_string(),
            details: None,
        })
    }
}

fn build_transfer_pairs(
    source_students: &[StudentMatchRow],
    target_students: &[StudentMatchRow],
    match_mode: &str,
) -> (Vec<(StudentMatchRow, Option<StudentMatchRow>)>, HashSet<String>) {
    let mut by_student_no: HashMap<String, Vec<String>> = HashMap::new();
    let mut by_name: HashMap<String, Vec<String>> = HashMap::new();
    let mut target_by_id: HashMap<String, StudentMatchRow> = HashMap::new();
    for t in target_students {
        target_by_id.insert(t.id.clone(), t.clone());
        if let Some(student_no) = t.student_no.as_deref().and_then(non_empty_trimmed) {
            by_student_no
                .entry(normalize_key(&student_no))
                .or_default()
                .push(t.id.clone());
        }
        by_name
            .entry(normalized_name_key(&t.last_name, &t.first_name))
            .or_default()
            .push(t.id.clone());
    }

    let mut used_targets = HashSet::new();
    let mut pairs = Vec::new();
    for source in source_students {
        let pick = choose_transfer_target(
            source,
            &used_targets,
            &by_student_no,
            &by_name,
            match_mode,
        );
        let target = pick
            .as_deref()
            .and_then(|id| target_by_id.get(id))
            .cloned();
        if let Some(t) = target.as_ref() {
            used_targets.insert(t.id.clone());
        }
        pairs.push((source.clone(), target));
    }
    (pairs, used_targets)
}

fn comments_transfer_preview(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let source_class_id = get_required_str(params, "sourceClassId")?;
    let source_mark_set_id = get_required_str(params, "sourceMarkSetId")?;
    let source_set_number = params
        .get("sourceSetNumber")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| HandlerErr {
            code: "bad_params",
            message: "missing sourceSetNumber".to_string(),
            details: None,
        })?;
    let target_class_id = get_required_str(params, "targetClassId")?;
    let target_mark_set_id = get_required_str(params, "targetMarkSetId")?;
    let target_set_number = params
        .get("targetSetNumber")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| HandlerErr {
            code: "bad_params",
            message: "missing targetSetNumber".to_string(),
            details: None,
        })?;
    let match_mode = parse_student_match_mode(params)?;

    if !mark_set_exists(conn, &source_class_id, &source_mark_set_id)? {
        return Err(HandlerErr {
            code: "not_found",
            message: "source mark set not found".to_string(),
            details: None,
        });
    }
    if !mark_set_exists(conn, &target_class_id, &target_mark_set_id)? {
        return Err(HandlerErr {
            code: "not_found",
            message: "target mark set not found".to_string(),
            details: None,
        });
    }

    let source_meta = load_comment_set_fit_meta(
        conn,
        &source_class_id,
        &source_mark_set_id,
        source_set_number,
    )?;
    let target_meta = load_comment_set_fit_meta(
        conn,
        &target_class_id,
        &target_mark_set_id,
        target_set_number,
    )?;

    let source_students = list_student_match_rows(conn, &source_class_id)?;
    let target_students = list_student_match_rows(conn, &target_class_id)?;
    let source_remarks = load_remarks_for_set(conn, &source_meta.set_id)?;
    let target_remarks = load_remarks_for_set(conn, &target_meta.set_id)?;

    let (pairs, used_targets) = build_transfer_pairs(&source_students, &target_students, &match_mode);
    let mut matched = 0usize;
    let mut same = 0usize;
    let mut different = 0usize;
    let mut rows = Vec::new();

    for (source, target) in pairs {
        let source_remark = source_remarks
            .get(&source.id)
            .cloned()
            .unwrap_or_default();
        if let Some(target) = target {
            matched += 1;
            let target_remark = target_remarks
                .get(&target.id)
                .cloned()
                .unwrap_or_default();
            let status = if source_remark.trim() == target_remark.trim() {
                same += 1;
                "same"
            } else {
                different += 1;
                "different"
            };
            rows.push(json!({
                "sourceStudentId": source.id,
                "targetStudentId": target.id,
                "sourceDisplayName": source.display_name,
                "targetDisplayName": target.display_name,
                "sourceRemark": source_remark,
                "targetRemark": target_remark,
                "status": status
            }));
        } else {
            rows.push(json!({
                "sourceStudentId": source.id,
                "sourceDisplayName": source.display_name,
                "sourceRemark": source_remark,
                "targetRemark": "",
                "status": "source_only"
            }));
        }
    }

    let mut target_only = 0usize;
    for target in &target_students {
        if used_targets.contains(&target.id) {
            continue;
        }
        target_only += 1;
        rows.push(json!({
            "targetStudentId": target.id,
            "targetDisplayName": target.display_name,
            "sourceRemark": "",
            "targetRemark": target_remarks.get(&target.id).cloned().unwrap_or_default(),
            "status": "target_only"
        }));
    }

    let unmatched_source = source_students.len().saturating_sub(matched);
    let unmatched_target = target_students.len().saturating_sub(matched);

    Ok(json!({
        "counts": {
            "sourceRows": source_students.len(),
            "targetRows": target_students.len(),
            "matched": matched,
            "unmatchedSource": unmatched_source,
            "unmatchedTarget": unmatched_target,
            "same": same,
            "different": different,
            "sourceOnly": unmatched_source,
            "targetOnly": target_only
        },
        "rows": rows
    }))
}

fn comments_transfer_apply(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let source_class_id = get_required_str(params, "sourceClassId")?;
    let source_mark_set_id = get_required_str(params, "sourceMarkSetId")?;
    let source_set_number = params
        .get("sourceSetNumber")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| HandlerErr {
            code: "bad_params",
            message: "missing sourceSetNumber".to_string(),
            details: None,
        })?;
    let target_class_id = get_required_str(params, "targetClassId")?;
    let target_mark_set_id = get_required_str(params, "targetMarkSetId")?;
    let target_set_number = params
        .get("targetSetNumber")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| HandlerErr {
            code: "bad_params",
            message: "missing targetSetNumber".to_string(),
            details: None,
        })?;
    let match_mode = parse_student_match_mode(params)?;
    let policy = parse_transfer_policy(params)?;
    let separator = params
        .get("separator")
        .and_then(|v| v.as_str())
        .unwrap_or(" ");
    let (scope, selected_targets) = parse_transfer_scope(params)?;

    if !mark_set_exists(conn, &source_class_id, &source_mark_set_id)? {
        return Err(HandlerErr {
            code: "not_found",
            message: "source mark set not found".to_string(),
            details: None,
        });
    }
    if !mark_set_exists(conn, &target_class_id, &target_mark_set_id)? {
        return Err(HandlerErr {
            code: "not_found",
            message: "target mark set not found".to_string(),
            details: None,
        });
    }

    let source_meta = load_comment_set_fit_meta(
        conn,
        &source_class_id,
        &source_mark_set_id,
        source_set_number,
    )?;
    let target_meta = load_comment_set_fit_meta(
        conn,
        &target_class_id,
        &target_mark_set_id,
        target_set_number,
    )?;
    let (max_chars, fit_width, fit_lines) = resolve_effective_fit_constraints(conn, &target_meta)?;

    let source_students = list_student_match_rows(conn, &source_class_id)?;
    let target_students = list_student_match_rows(conn, &target_class_id)?;
    let source_remarks = load_remarks_for_set(conn, &source_meta.set_id)?;
    let target_remarks = load_remarks_for_set(conn, &target_meta.set_id)?;
    let (pairs, _used_targets) = build_transfer_pairs(&source_students, &target_students, &match_mode);

    let tx = conn.unchecked_transaction().map_err(|e| HandlerErr {
        code: "db_tx_failed",
        message: e.to_string(),
        details: None,
    })?;

    let mut updated = 0usize;
    let mut skipped = 0usize;
    let mut unchanged = 0usize;
    let mut warnings = Vec::new();

    for (source, target) in pairs {
        let Some(target) = target else {
            skipped += 1;
            continue;
        };
        if scope == "selected_target_students" && !selected_targets.contains(&target.id) {
            skipped += 1;
            continue;
        }

        let source_remark = source_remarks
            .get(&source.id)
            .cloned()
            .unwrap_or_default();
        let target_remark = target_remarks
            .get(&target.id)
            .cloned()
            .unwrap_or_default();

        let Some(next_text_raw) =
            transfer_text_by_policy(&source_remark, &target_remark, &policy, separator)
        else {
            unchanged += 1;
            continue;
        };
        let (next_text, truncated) =
            apply_fit_constraints(&next_text_raw, max_chars, fit_width, fit_lines);

        if next_text.trim() == target_remark.trim() {
            unchanged += 1;
            continue;
        }

        if next_text.trim().is_empty() {
            tx.execute(
                "DELETE FROM comment_set_remarks
                 WHERE comment_set_index_id = ? AND student_id = ?",
                (&target_meta.set_id, &target.id),
            )
            .map_err(|e| HandlerErr {
                code: "db_delete_failed",
                message: e.to_string(),
                details: Some(json!({ "table": "comment_set_remarks" })),
            })?;
        } else {
            let row_id = Uuid::new_v4().to_string();
            tx.execute(
                "INSERT INTO comment_set_remarks(id, comment_set_index_id, student_id, remark)
                 VALUES(?, ?, ?, ?)
                 ON CONFLICT(comment_set_index_id, student_id) DO UPDATE SET
                   remark = excluded.remark",
                (&row_id, &target_meta.set_id, &target.id, &next_text),
            )
            .map_err(|e| HandlerErr {
                code: "db_insert_failed",
                message: e.to_string(),
                details: Some(json!({ "table": "comment_set_remarks" })),
            })?;
        }
        if truncated {
            warnings.push(json!({
                "studentId": target.id,
                "code": "fit_truncated",
                "message": "remark truncated by fit/max length constraints"
            }));
        }
        updated += 1;
    }

    tx.commit().map_err(|e| HandlerErr {
        code: "db_commit_failed",
        message: e.to_string(),
        details: None,
    })?;

    Ok(json!({
        "ok": true,
        "updated": updated,
        "skipped": skipped,
        "unchanged": unchanged,
        "warnings": warnings
    }))
}

fn comments_transfer_flood_fill(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let class_id = get_required_str(params, "classId")?;
    let mark_set_id = get_required_str(params, "markSetId")?;
    let set_number = params
        .get("setNumber")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| HandlerErr {
            code: "bad_params",
            message: "missing setNumber".to_string(),
            details: None,
        })?;
    let source_student_id = get_required_str(params, "sourceStudentId")?;
    let policy = parse_transfer_policy(params)?;
    let separator = params
        .get("separator")
        .and_then(|v| v.as_str())
        .unwrap_or(" ");
    let target_student_ids = params
        .get("targetStudentIds")
        .and_then(|v| v.as_array())
        .ok_or_else(|| HandlerErr {
            code: "bad_params",
            message: "missing targetStudentIds".to_string(),
            details: None,
        })?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();

    if !mark_set_exists(conn, &class_id, &mark_set_id)? {
        return Err(HandlerErr {
            code: "not_found",
            message: "mark set not found".to_string(),
            details: None,
        });
    }

    let meta = load_comment_set_fit_meta(conn, &class_id, &mark_set_id, set_number)?;
    let (max_chars, fit_width, fit_lines) = resolve_effective_fit_constraints(conn, &meta)?;
    let remarks = load_remarks_for_set(conn, &meta.set_id)?;
    let source_remark = remarks
        .get(&source_student_id)
        .cloned()
        .unwrap_or_default();

    let students = list_student_match_rows(conn, &class_id)?;
    let valid_targets = students
        .into_iter()
        .map(|s| s.id)
        .collect::<HashSet<_>>();

    let tx = conn.unchecked_transaction().map_err(|e| HandlerErr {
        code: "db_tx_failed",
        message: e.to_string(),
        details: None,
    })?;

    let mut updated = 0usize;
    let mut skipped = 0usize;
    for target_student_id in target_student_ids {
        if target_student_id == source_student_id || !valid_targets.contains(&target_student_id) {
            skipped += 1;
            continue;
        }
        let target_remark = remarks
            .get(&target_student_id)
            .cloned()
            .unwrap_or_default();
        let Some(next_text_raw) =
            transfer_text_by_policy(&source_remark, &target_remark, &policy, separator)
        else {
            skipped += 1;
            continue;
        };
        let (next_text, _truncated) =
            apply_fit_constraints(&next_text_raw, max_chars, fit_width, fit_lines);
        if next_text.trim() == target_remark.trim() {
            skipped += 1;
            continue;
        }
        if next_text.trim().is_empty() {
            tx.execute(
                "DELETE FROM comment_set_remarks
                 WHERE comment_set_index_id = ? AND student_id = ?",
                (&meta.set_id, &target_student_id),
            )
            .map_err(|e| HandlerErr {
                code: "db_delete_failed",
                message: e.to_string(),
                details: Some(json!({ "table": "comment_set_remarks" })),
            })?;
        } else {
            let row_id = Uuid::new_v4().to_string();
            tx.execute(
                "INSERT INTO comment_set_remarks(id, comment_set_index_id, student_id, remark)
                 VALUES(?, ?, ?, ?)
                 ON CONFLICT(comment_set_index_id, student_id) DO UPDATE SET
                   remark = excluded.remark",
                (&row_id, &meta.set_id, &target_student_id, &next_text),
            )
            .map_err(|e| HandlerErr {
                code: "db_insert_failed",
                message: e.to_string(),
                details: Some(json!({ "table": "comment_set_remarks" })),
            })?;
        }
        updated += 1;
    }

    tx.commit().map_err(|e| HandlerErr {
        code: "db_commit_failed",
        message: e.to_string(),
        details: None,
    })?;

    Ok(json!({ "ok": true, "updated": updated, "skipped": skipped }))
}

fn comments_banks_list(conn: &Connection) -> Result<serde_json::Value, HandlerErr> {
    let mut stmt = conn
        .prepare(
            "SELECT
               b.id,
               b.short_name,
               b.is_default,
               b.fit_profile,
               b.source_path,
               (SELECT COUNT(*) FROM comment_bank_entries e WHERE e.bank_id = b.id) AS entry_count
             FROM comment_banks b
             ORDER BY b.short_name",
        )
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let banks = stmt
        .query_map([], |r| {
            Ok(json!({
                "id": r.get::<_, String>(0)?,
                "shortName": r.get::<_, String>(1)?,
                "isDefault": r.get::<_, i64>(2)? != 0,
                "fitProfile": r.get::<_, Option<String>>(3)?,
                "sourcePath": r.get::<_, Option<String>>(4)?,
                "entryCount": r.get::<_, i64>(5)?
            }))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    Ok(json!({ "banks": banks }))
}

fn comments_banks_open(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let bank_id = get_required_str(params, "bankId")?;
    let bank: Option<serde_json::Value> = conn
        .query_row(
            "SELECT id, short_name, is_default, fit_profile, source_path FROM comment_banks WHERE id = ?",
            [&bank_id],
            |r| {
                Ok(json!({
                    "id": r.get::<_, String>(0)?,
                    "shortName": r.get::<_, String>(1)?,
                    "isDefault": r.get::<_, i64>(2)? != 0,
                    "fitProfile": r.get::<_, Option<String>>(3)?,
                    "sourcePath": r.get::<_, Option<String>>(4)?
                }))
            },
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let Some(bank) = bank else {
        return Err(HandlerErr {
            code: "not_found",
            message: "bank not found".to_string(),
            details: None,
        });
    };
    let mut stmt = conn
        .prepare(
            "SELECT id, sort_order, type_code, level_code, text
             FROM comment_bank_entries
             WHERE bank_id = ?
             ORDER BY sort_order",
        )
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let entries = stmt
        .query_map([&bank_id], |r| {
            Ok(json!({
                "id": r.get::<_, String>(0)?,
                "sortOrder": r.get::<_, i64>(1)?,
                "typeCode": r.get::<_, String>(2)?,
                "levelCode": r.get::<_, String>(3)?,
                "text": r.get::<_, String>(4)?,
            }))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    Ok(json!({ "bank": bank, "entries": entries }))
}

fn comments_banks_create(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let short_name = get_required_str(params, "shortName")?.trim().to_string();
    if short_name.is_empty() {
        return Err(HandlerErr {
            code: "bad_params",
            message: "shortName must not be empty".to_string(),
            details: None,
        });
    }
    let bank_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO comment_banks(id, short_name, is_default, fit_profile, source_path)
         VALUES(?, ?, 0, NULL, NULL)",
        (&bank_id, &short_name),
    )
    .map_err(|e| HandlerErr {
        code: "db_insert_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "comment_banks" })),
    })?;
    Ok(json!({ "bankId": bank_id }))
}

fn comments_banks_update_meta(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let bank_id = get_required_str(params, "bankId")?;
    let Some(patch) = params.get("patch").and_then(|v| v.as_object()) else {
        return Err(HandlerErr {
            code: "bad_params",
            message: "missing patch".to_string(),
            details: None,
        });
    };
    let tx = conn.unchecked_transaction().map_err(|e| HandlerErr {
        code: "db_tx_failed",
        message: e.to_string(),
        details: None,
    })?;
    if patch
        .get("isDefault")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        tx.execute("UPDATE comment_banks SET is_default = 0", [])
            .map_err(|e| HandlerErr {
                code: "db_update_failed",
                message: e.to_string(),
                details: Some(json!({ "table": "comment_banks" })),
            })?;
    }

    let mut set_parts: Vec<String> = Vec::new();
    let mut binds: Vec<Value> = Vec::new();
    if let Some(v) = patch.get("shortName") {
        let Some(s) = v.as_str() else {
            return Err(HandlerErr {
                code: "bad_params",
                message: "patch.shortName must be string".to_string(),
                details: None,
            });
        };
        set_parts.push("short_name = ?".into());
        binds.push(Value::Text(s.trim().to_string()));
    }
    if let Some(v) = patch.get("isDefault") {
        let Some(b) = v.as_bool() else {
            return Err(HandlerErr {
                code: "bad_params",
                message: "patch.isDefault must be boolean".to_string(),
                details: None,
            });
        };
        set_parts.push("is_default = ?".into());
        binds.push(Value::Integer(if b { 1 } else { 0 }));
    }
    if let Some(v) = patch.get("fitProfile") {
        set_parts.push("fit_profile = ?".into());
        if v.is_null() {
            binds.push(Value::Null);
        } else if let Some(s) = v.as_str() {
            binds.push(Value::Text(s.to_string()));
        } else {
            return Err(HandlerErr {
                code: "bad_params",
                message: "patch.fitProfile must be string|null".to_string(),
                details: None,
            });
        }
    }
    if let Some(v) = patch.get("sourcePath") {
        set_parts.push("source_path = ?".into());
        if v.is_null() {
            binds.push(Value::Null);
        } else if let Some(s) = v.as_str() {
            binds.push(Value::Text(s.to_string()));
        } else {
            return Err(HandlerErr {
                code: "bad_params",
                message: "patch.sourcePath must be string|null".to_string(),
                details: None,
            });
        }
    }

    if !set_parts.is_empty() {
        let sql = format!(
            "UPDATE comment_banks SET {} WHERE id = ?",
            set_parts.join(", ")
        );
        binds.push(Value::Text(bank_id.clone()));
        tx.execute(&sql, params_from_iter(binds))
            .map_err(|e| HandlerErr {
                code: "db_update_failed",
                message: e.to_string(),
                details: Some(json!({ "table": "comment_banks" })),
            })?;
    }
    tx.commit().map_err(|e| HandlerErr {
        code: "db_commit_failed",
        message: e.to_string(),
        details: None,
    })?;
    Ok(json!({ "ok": true }))
}

fn comments_banks_entry_upsert(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let bank_id = get_required_str(params, "bankId")?;
    let type_code = get_required_str(params, "typeCode")?;
    let level_code = get_required_str(params, "levelCode")?;
    let text = get_required_str(params, "text")?;
    let requested_sort = params.get("sortOrder").and_then(|v| v.as_i64());
    let entry_id = params
        .get("entryId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let tx = conn.unchecked_transaction().map_err(|e| HandlerErr {
        code: "db_tx_failed",
        message: e.to_string(),
        details: None,
    })?;

    let resolved_entry_id = entry_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let existing_sort: Option<i64> = tx
        .query_row(
            "SELECT sort_order FROM comment_bank_entries WHERE id = ? AND bank_id = ?",
            (&resolved_entry_id, &bank_id),
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;

    let mut target_sort = requested_sort.unwrap_or_else(|| {
        tx.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM comment_bank_entries WHERE bank_id = ?",
            [&bank_id],
            |r| r.get::<_, i64>(0),
        )
        .unwrap_or(0)
    });
    if target_sort < 0 {
        target_sort = 0;
    }

    if let Some(cur_sort) = existing_sort {
        if target_sort != cur_sort {
            if target_sort > cur_sort {
                tx.execute(
                    "UPDATE comment_bank_entries
                     SET sort_order = sort_order - 1
                     WHERE bank_id = ? AND sort_order > ? AND sort_order <= ?",
                    (&bank_id, cur_sort, target_sort),
                )
                .map_err(|e| HandlerErr {
                    code: "db_update_failed",
                    message: e.to_string(),
                    details: Some(json!({ "table": "comment_bank_entries" })),
                })?;
            } else {
                tx.execute(
                    "UPDATE comment_bank_entries
                     SET sort_order = sort_order + 1
                     WHERE bank_id = ? AND sort_order >= ? AND sort_order < ?",
                    (&bank_id, target_sort, cur_sort),
                )
                .map_err(|e| HandlerErr {
                    code: "db_update_failed",
                    message: e.to_string(),
                    details: Some(json!({ "table": "comment_bank_entries" })),
                })?;
            }
        }
    } else {
        tx.execute(
            "UPDATE comment_bank_entries
             SET sort_order = sort_order + 1
             WHERE bank_id = ? AND sort_order >= ?",
            (&bank_id, target_sort),
        )
        .map_err(|e| HandlerErr {
            code: "db_update_failed",
            message: e.to_string(),
            details: Some(json!({ "table": "comment_bank_entries" })),
        })?;
    }

    tx.execute(
        "INSERT INTO comment_bank_entries(id, bank_id, sort_order, type_code, level_code, text)
         VALUES(?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
           sort_order = excluded.sort_order,
           type_code = excluded.type_code,
           level_code = excluded.level_code,
           text = excluded.text",
        (
            &resolved_entry_id,
            &bank_id,
            target_sort,
            &type_code,
            &level_code,
            &text,
        ),
    )
    .map_err(|e| HandlerErr {
        code: "db_insert_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "comment_bank_entries" })),
    })?;

    tx.commit().map_err(|e| HandlerErr {
        code: "db_commit_failed",
        message: e.to_string(),
        details: None,
    })?;
    Ok(json!({ "entryId": resolved_entry_id }))
}

fn comments_banks_entry_delete(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let bank_id = get_required_str(params, "bankId")?;
    let entry_id = get_required_str(params, "entryId")?;
    let tx = conn.unchecked_transaction().map_err(|e| HandlerErr {
        code: "db_tx_failed",
        message: e.to_string(),
        details: None,
    })?;
    let sort_order: Option<i64> = tx
        .query_row(
            "SELECT sort_order FROM comment_bank_entries WHERE id = ? AND bank_id = ?",
            (&entry_id, &bank_id),
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let Some(sort_order) = sort_order else {
        return Err(HandlerErr {
            code: "not_found",
            message: "entry not found".to_string(),
            details: None,
        });
    };
    tx.execute(
        "DELETE FROM comment_bank_entries WHERE id = ? AND bank_id = ?",
        (&entry_id, &bank_id),
    )
    .map_err(|e| HandlerErr {
        code: "db_delete_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "comment_bank_entries" })),
    })?;
    tx.execute(
        "UPDATE comment_bank_entries
         SET sort_order = sort_order - 1
         WHERE bank_id = ? AND sort_order > ?",
        (&bank_id, sort_order),
    )
    .map_err(|e| HandlerErr {
        code: "db_update_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "comment_bank_entries" })),
    })?;
    tx.commit().map_err(|e| HandlerErr {
        code: "db_commit_failed",
        message: e.to_string(),
        details: None,
    })?;
    Ok(json!({ "ok": true }))
}

fn comments_banks_import_bnk(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let path = get_required_str(params, "path")?;
    let file_path = PathBuf::from(&path);
    let short_name = file_path
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| HandlerErr {
            code: "bad_params",
            message: "invalid path".to_string(),
            details: None,
        })?
        .to_string();
    let parsed = legacy::parse_bnk_file(&file_path).map_err(|e| HandlerErr {
        code: "legacy_parse_failed",
        message: e.to_string(),
        details: Some(json!({ "path": path })),
    })?;

    let tx = conn.unchecked_transaction().map_err(|e| HandlerErr {
        code: "db_tx_failed",
        message: e.to_string(),
        details: None,
    })?;
    let new_id = Uuid::new_v4().to_string();
    tx.execute(
        "INSERT INTO comment_banks(id, short_name, is_default, fit_profile, source_path)
         VALUES(?, ?, 0, ?, ?)
         ON CONFLICT(short_name) DO UPDATE SET
           fit_profile = excluded.fit_profile,
           source_path = excluded.source_path",
        (&new_id, &short_name, parsed.fit_profile.as_deref(), &path),
    )
    .map_err(|e| HandlerErr {
        code: "db_insert_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "comment_banks" })),
    })?;
    let bank_id: String = tx
        .query_row(
            "SELECT id FROM comment_banks WHERE short_name = ?",
            [&short_name],
            |r| r.get(0),
        )
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    tx.execute(
        "DELETE FROM comment_bank_entries WHERE bank_id = ?",
        [&bank_id],
    )
    .map_err(|e| HandlerErr {
        code: "db_delete_failed",
        message: e.to_string(),
        details: Some(json!({ "table": "comment_bank_entries" })),
    })?;
    for (sort_order, entry) in parsed.entries.iter().enumerate() {
        let eid = Uuid::new_v4().to_string();
        tx.execute(
            "INSERT INTO comment_bank_entries(id, bank_id, sort_order, type_code, level_code, text)
             VALUES(?, ?, ?, ?, ?, ?)",
            (
                &eid,
                &bank_id,
                sort_order as i64,
                &entry.type_code,
                &entry.level_code,
                &entry.text,
            ),
        )
        .map_err(|e| HandlerErr {
            code: "db_insert_failed",
            message: e.to_string(),
            details: Some(json!({ "table": "comment_bank_entries" })),
        })?;
    }
    tx.commit().map_err(|e| HandlerErr {
        code: "db_commit_failed",
        message: e.to_string(),
        details: None,
    })?;
    Ok(json!({ "bankId": bank_id }))
}

fn comments_banks_export_bnk(
    conn: &Connection,
    params: &serde_json::Value,
) -> Result<serde_json::Value, HandlerErr> {
    let bank_id = get_required_str(params, "bankId")?;
    let out_path = get_required_str(params, "path")?;
    let bank_meta: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT short_name, fit_profile FROM comment_banks WHERE id = ?",
            [&bank_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let Some((_short_name, fit_profile)) = bank_meta else {
        return Err(HandlerErr {
            code: "not_found",
            message: "bank not found".to_string(),
            details: None,
        });
    };
    let mut stmt = conn
        .prepare(
            "SELECT sort_order, type_code, level_code, text
             FROM comment_bank_entries
             WHERE bank_id = ?
             ORDER BY sort_order",
        )
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let entries = stmt
        .query_map([&bank_id], |r| {
            Ok(legacy::ParsedBnkEntry {
                sort_order: r.get::<_, i64>(0)? as usize,
                type_code: r.get(1)?,
                level_code: r.get(2)?,
                text: r.get(3)?,
            })
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        .map_err(|e| HandlerErr {
            code: "db_query_failed",
            message: e.to_string(),
            details: None,
        })?;
    let text = legacy::serialize_bnk_file(&legacy::ParsedBnkFile {
        fit_profile,
        entries,
    });
    let out = PathBuf::from(&out_path);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|e| HandlerErr {
            code: "io_failed",
            message: e.to_string(),
            details: Some(json!({ "path": out_path })),
        })?;
    }
    std::fs::write(&out, text).map_err(|e| HandlerErr {
        code: "io_failed",
        message: e.to_string(),
        details: Some(json!({ "path": out_path })),
    })?;
    Ok(json!({ "ok": true }))
}

fn handle_comments_sets_list(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_sets_list(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_comments_sets_open(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_sets_open(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_comments_sets_upsert(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_sets_upsert(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_comments_sets_delete(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_sets_delete(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_comments_banks_list(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_banks_list(conn) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_comments_banks_open(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_banks_open(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_comments_banks_create(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_banks_create(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_comments_banks_update_meta(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_banks_update_meta(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_comments_banks_entry_upsert(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_banks_entry_upsert(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_comments_banks_entry_delete(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_banks_entry_delete(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_comments_banks_import_bnk(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_banks_import_bnk(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_comments_banks_export_bnk(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_banks_export_bnk(conn, &req.params) {
        Ok(result) => ok(&req.id, result),
        Err(error) => error.response(&req.id),
    }
}

fn handle_comments_transfer_preview(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_transfer_preview(conn, &req.params) {
        Ok(v) => ok(&req.id, v),
        Err(e) => e.response(&req.id),
    }
}

fn handle_comments_transfer_apply(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_transfer_apply(conn, &req.params) {
        Ok(v) => ok(&req.id, v),
        Err(e) => e.response(&req.id),
    }
}

fn handle_comments_transfer_flood_fill(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_transfer_flood_fill(conn, &req.params) {
        Ok(v) => ok(&req.id, v),
        Err(e) => e.response(&req.id),
    }
}

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "comments.sets.list" => Some(handle_comments_sets_list(state, req)),
        "comments.sets.open" => Some(handle_comments_sets_open(state, req)),
        "comments.sets.upsert" => Some(handle_comments_sets_upsert(state, req)),
        "comments.sets.delete" => Some(handle_comments_sets_delete(state, req)),
        "comments.remarks.upsertOne" => Some(handle_comments_remarks_upsert_one(state, req)),
        "comments.banks.list" => Some(handle_comments_banks_list(state, req)),
        "comments.banks.open" => Some(handle_comments_banks_open(state, req)),
        "comments.banks.create" => Some(handle_comments_banks_create(state, req)),
        "comments.banks.updateMeta" => Some(handle_comments_banks_update_meta(state, req)),
        "comments.banks.entryUpsert" => Some(handle_comments_banks_entry_upsert(state, req)),
        "comments.banks.entryDelete" => Some(handle_comments_banks_entry_delete(state, req)),
        "comments.banks.importBnk" => Some(handle_comments_banks_import_bnk(state, req)),
        "comments.banks.exportBnk" => Some(handle_comments_banks_export_bnk(state, req)),
        "comments.transfer.preview" => Some(handle_comments_transfer_preview(state, req)),
        "comments.transfer.apply" => Some(handle_comments_transfer_apply(state, req)),
        "comments.transfer.floodFill" => Some(handle_comments_transfer_flood_fill(state, req)),
        _ => None,
    }
}

fn handle_comments_remarks_upsert_one(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    match comments_remarks_upsert_one(conn, &req.params) {
        Ok(v) => ok(&req.id, v),
        Err(e) => e.response(&req.id),
    }
}
