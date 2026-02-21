use crate::ipc::handlers::classes as classes_handler;
use crate::ipc::types::{AppState, Request};
use crate::legacy;
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Serialize)]
struct OkResp {
    id: String,
    ok: bool,
    result: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct ErrObj {
    code: String,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct ErrResp {
    id: String,
    ok: bool,
    error: ErrObj,
}

fn normalize_key(s: &str) -> String {
    s.trim().to_ascii_lowercase()
}

fn normalize_opt_key(s: Option<&str>) -> Option<String> {
    s.map(normalize_key).filter(|v| !v.is_empty())
}

fn student_name_key(last_name: &str, first_name: &str) -> String {
    format!("{}|{}", normalize_key(last_name), normalize_key(first_name))
}

fn assessment_collision_key(
    date: Option<&str>,
    category_name: Option<&str>,
    title: &str,
    term: Option<i64>,
) -> String {
    format!(
        "{}|{}|{}|{}",
        normalize_key(date.unwrap_or("")),
        normalize_key(category_name.unwrap_or("")),
        normalize_key(title),
        term.unwrap_or(0)
    )
}

fn rewrite_response_id(mut value: serde_json::Value, id: &str) -> serde_json::Value {
    if let Some(obj) = value.as_object_mut() {
        obj.insert("id".into(), json!(id));
    }
    value
}

fn class_exists(conn: &Connection, class_id: &str) -> Result<bool, rusqlite::Error> {
    Ok(conn
        .query_row("SELECT 1 FROM classes WHERE id = ?", [class_id], |r| {
            r.get::<_, i64>(0)
        })
        .optional()?
        .is_some())
}

fn class_meta_year_token_from_cl_file(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.trim().to_ascii_uppercase())
        .filter(|ext| ext.starts_with('Y') && ext.len() >= 2)
}

fn cleanup_temp_class(state: &mut AppState, temp_class_id: &str) {
    let cleanup_req = Request {
        id: "__cleanup_temp_import_class".into(),
        method: "classes.delete".into(),
        params: json!({ "classId": temp_class_id }),
    };
    let _ = classes_handler::try_handle(state, &cleanup_req);
}

fn handle_class_import_legacy(state: &mut AppState, req: Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return json!(ErrResp {
            id: req.id,
            ok: false,
            error: ErrObj {
                code: "no_workspace".into(),
                message: "select a workspace first".into(),
                details: None
            }
        });
    };

    let legacy_folder = req
        .params
        .get("legacyClassFolderPath")
        .and_then(|v| v.as_str())
        .map(PathBuf::from);

    let Some(legacy_folder) = legacy_folder else {
        return json!(ErrResp {
            id: req.id,
            ok: false,
            error: ErrObj {
                code: "bad_params".into(),
                message: "missing legacyClassFolderPath".into(),
                details: None
            }
        });
    };

    let cl_file = match legacy::find_cl_file(&legacy_folder) {
        Ok(v) => v,
        Err(e) => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "legacy_no_cl".into(),
                    message: e.to_string(),
                    details: Some(json!({ "folder": legacy_folder.to_string_lossy() }))
                }
            })
        }
    };

    let parsed = match legacy::parse_legacy_cl(&cl_file) {
        Ok(v) => v,
        Err(e) => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "legacy_parse_failed".into(),
                    message: e.to_string(),
                    details: Some(json!({ "clFile": cl_file.to_string_lossy() }))
                }
            })
        }
    };

    let class_id = Uuid::new_v4().to_string();
    let class_name = parsed.class_name;

    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "db_tx_failed".into(),
                    message: e.to_string(),
                    details: None
                }
            })
        }
    };

    if let Err(e) = tx.execute(
        "INSERT INTO classes(id, name) VALUES(?, ?)",
        [&class_id, &class_name],
    ) {
        let _ = tx.rollback();
        return json!(ErrResp {
            id: req.id,
            ok: false,
            error: ErrObj {
                code: "db_insert_failed".into(),
                message: e.to_string(),
                details: None
            }
        });
    }

    let mut imported = 0usize;
    let mut student_ids_by_sort: Vec<String> = Vec::new();
    for (sort_order, s) in parsed.students.into_iter().enumerate() {
        let sid = Uuid::new_v4().to_string();
        let active_i = if s.active { 1 } else { 0 };
        let student_no = s.student_no.unwrap_or_default();
        let birth_date = s.birth_date.unwrap_or_default();
        let mark_set_mask = s.mark_set_mask.unwrap_or_else(|| "TBA".into());
        let res = tx.execute(
            "INSERT INTO students(id, class_id, last_name, first_name, student_no, birth_date, active, sort_order, raw_line, mark_set_mask)
             VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                &sid,
                &class_id,
                &s.last_name,
                &s.first_name,
                &student_no,
                &birth_date,
                active_i,
                sort_order as i64,
                &s.raw_line,
                &mark_set_mask,
            ),
        );
        if res.is_ok() {
            imported += 1;
            student_ids_by_sort.push(sid);
        }
    }

    // Best-effort import class-level student notes (*NOTE.TXT).
    if let Some(note_file) = match legacy::find_note_file(&legacy_folder) {
        Ok(v) => v,
        Err(e) => {
            let _ = tx.rollback();
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "legacy_read_failed".into(),
                    message: e.to_string(),
                    details: Some(json!({ "folder": legacy_folder.to_string_lossy() }))
                }
            });
        }
    } {
        let notes = match legacy::parse_legacy_note_file(&note_file) {
            Ok(v) => v,
            Err(e) => {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "legacy_parse_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "noteFile": note_file.to_string_lossy() }))
                    }
                });
            }
        };

        let mut ins = match tx.prepare(
            "INSERT INTO student_notes(id, class_id, student_id, note)
             VALUES(?, ?, ?, ?)
             ON CONFLICT(class_id, student_id) DO UPDATE SET
               note = excluded.note",
        ) {
            Ok(s) => s,
            Err(e) => {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_insert_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "student_notes" }))
                    }
                });
            }
        };

        let max = std::cmp::min(notes.len(), student_ids_by_sort.len());
        for s_idx in 0..max {
            let note = notes[s_idx].trim().to_string();
            if note.is_empty() {
                continue;
            }
            let nid = Uuid::new_v4().to_string();
            let student_id = &student_ids_by_sort[s_idx];
            if let Err(e) = ins.execute((&nid, &class_id, student_id, &note)) {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_insert_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "student_notes" }))
                    }
                });
            }
        }
    }

    let mut attendance_imported = false;
    let mut seating_imported = false;
    let mut banks_imported = 0usize;
    let mut comment_sets_imported = 0usize;
    let mut comment_remarks_imported = 0usize;
    let mut loaned_items_imported = 0usize;
    let mut device_mappings_imported = 0usize;
    let mut combined_comment_sets_imported = 0usize;
    let mut warnings: Vec<serde_json::Value> = Vec::new();

    // Best-effort attendance import (.ATN).
    match legacy::find_attendance_file(&legacy_folder) {
        Ok(Some(att_file)) => {
            let att = match legacy::parse_legacy_attendance_file(&att_file) {
                Ok(v) => v,
                Err(e) => {
                    let _ = tx.rollback();
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "legacy_parse_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "attendanceFile": att_file.to_string_lossy() }))
                        }
                    });
                }
            };

            if let Err(e) = tx.execute(
                "INSERT INTO attendance_settings(class_id, school_year_start_month)
                 VALUES(?, ?)
                 ON CONFLICT(class_id) DO UPDATE SET
                   school_year_start_month = excluded.school_year_start_month",
                (&class_id, att.school_year_start_month as i64),
            ) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_insert_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "attendance_settings" }))
                    }
                });
            }

            for m in &att.months {
                if let Err(e) = tx.execute(
                    "INSERT INTO attendance_months(class_id, month, type_of_day_codes)
                     VALUES(?, ?, ?)
                     ON CONFLICT(class_id, month) DO UPDATE SET
                       type_of_day_codes = excluded.type_of_day_codes",
                    (&class_id, m.month as i64, &m.type_of_day_codes),
                ) {
                    let _ = tx.rollback();
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_insert_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "attendance_months" }))
                        }
                    });
                }

                let max_students =
                    std::cmp::min(student_ids_by_sort.len(), m.student_day_codes.len());
                for s_idx in 0..max_students {
                    let student_id = &student_ids_by_sort[s_idx];
                    let day_codes = &m.student_day_codes[s_idx];
                    if let Err(e) = tx.execute(
                        "INSERT INTO attendance_student_months(class_id, student_id, month, day_codes)
                         VALUES(?, ?, ?, ?)
                         ON CONFLICT(class_id, student_id, month) DO UPDATE SET
                           day_codes = excluded.day_codes",
                        (&class_id, student_id, m.month as i64, day_codes),
                    ) {
                        let _ = tx.rollback();
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "db_insert_failed".into(),
                                message: e.to_string(),
                                details: Some(json!({ "table": "attendance_student_months" }))
                            }
                        });
                    }
                }
            }

            attendance_imported = true;
        }
        Ok(None) => {
            warnings.push(json!({
                "code": "legacy_missing_attendance_file",
                "folder": legacy_folder.to_string_lossy()
            }));
        }
        Err(e) => {
            let _ = tx.rollback();
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "legacy_read_failed".into(),
                    message: e.to_string(),
                    details: Some(json!({ "folder": legacy_folder.to_string_lossy() }))
                }
            });
        }
    }

    // Best-effort seating import (.SPL).
    match legacy::find_seating_file(&legacy_folder) {
        Ok(Some(spl_file)) => {
            let spl = match legacy::parse_legacy_seating_file(&spl_file) {
                Ok(v) => v,
                Err(e) => {
                    let _ = tx.rollback();
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "legacy_parse_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "seatingFile": spl_file.to_string_lossy() }))
                        }
                    });
                }
            };

            if let Err(e) = tx.execute(
                "INSERT INTO seating_plans(class_id, rows, seats_per_row, blocked_mask)
                 VALUES(?, ?, ?, ?)
                 ON CONFLICT(class_id) DO UPDATE SET
                   rows = excluded.rows,
                   seats_per_row = excluded.seats_per_row,
                   blocked_mask = excluded.blocked_mask",
                (
                    &class_id,
                    spl.rows as i64,
                    spl.seats_per_row as i64,
                    &spl.blocked_mask,
                ),
            ) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_insert_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "seating_plans" }))
                    }
                });
            }
            if let Err(e) = tx.execute(
                "DELETE FROM seating_assignments WHERE class_id = ?",
                [&class_id],
            ) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "seating_assignments" }))
                    }
                });
            }
            let max_students = std::cmp::min(student_ids_by_sort.len(), spl.seat_codes.len());
            for s_idx in 0..max_students {
                let seat_code = spl.seat_codes[s_idx];
                if seat_code <= 0 {
                    continue;
                }
                let student_id = &student_ids_by_sort[s_idx];
                if let Err(e) = tx.execute(
                    "INSERT INTO seating_assignments(class_id, student_id, seat_code)
                     VALUES(?, ?, ?)",
                    (&class_id, student_id, seat_code as i64),
                ) {
                    let _ = tx.rollback();
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_insert_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "seating_assignments" }))
                        }
                    });
                }
            }
            seating_imported = true;
        }
        Ok(None) => {
            warnings.push(json!({
                "code": "legacy_missing_seating_file",
                "folder": legacy_folder.to_string_lossy()
            }));
        }
        Err(e) => {
            let _ = tx.rollback();
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "legacy_read_failed".into(),
                    message: e.to_string(),
                    details: Some(json!({ "folder": legacy_folder.to_string_lossy() }))
                }
            });
        }
    }

    // Best-effort ICC import (device/class codes matrix).
    match legacy::find_icc_file(&legacy_folder) {
        Ok(Some(icc_file)) => {
            let icc = match legacy::parse_legacy_icc_file(&icc_file) {
                Ok(v) => v,
                Err(e) => {
                    let _ = tx.rollback();
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "legacy_parse_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "iccFile": icc_file.to_string_lossy() }))
                        }
                    });
                }
            };

            let max_students = std::cmp::min(student_ids_by_sort.len(), icc.last_student);
            for s_idx in 0..max_students {
                let student_id = &student_ids_by_sort[s_idx];
                let codes_row = icc
                    .codes
                    .get(s_idx + 1)
                    .cloned()
                    .unwrap_or_else(|| vec![String::new(); icc.subject_count + 1]);
                let primary_code = codes_row
                    .iter()
                    .skip(1)
                    .map(|s| s.trim())
                    .find(|s| !s.is_empty())
                    .unwrap_or("")
                    .to_string();
                let raw_line = serde_json::to_string(&json!({
                    "subjectCount": icc.subject_count,
                    "codes": codes_row
                }))
                .unwrap_or_else(|_| "[]".to_string());
                let did = Uuid::new_v4().to_string();
                if let Err(e) = tx.execute(
                    "INSERT INTO student_device_map(id, class_id, student_id, device_code, raw_line)
                     VALUES(?, ?, ?, ?, ?)
                     ON CONFLICT(class_id, student_id) DO UPDATE SET
                       device_code = excluded.device_code,
                       raw_line = excluded.raw_line",
                    (&did, &class_id, student_id, &primary_code, &raw_line),
                ) {
                    let _ = tx.rollback();
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_insert_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "student_device_map" }))
                        }
                    });
                }
                device_mappings_imported += 1;
            }
        }
        Ok(None) => {
            warnings.push(json!({
                "code": "legacy_missing_icc_file",
                "folder": legacy_folder.to_string_lossy()
            }));
        }
        Err(e) => {
            let _ = tx.rollback();
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "legacy_read_failed".into(),
                    message: e.to_string(),
                    details: Some(json!({ "folder": legacy_folder.to_string_lossy() }))
                }
            });
        }
    }

    // Best-effort bank import from parent fixture folder.
    let bnk_folder = legacy_folder
        .parent()
        .unwrap_or(&legacy_folder)
        .to_path_buf();
    match legacy::find_bnk_files(&bnk_folder) {
        Ok(files) => {
            for bnk_file in files {
                let parsed_bnk = match legacy::parse_bnk_file(&bnk_file) {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = tx.rollback();
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "legacy_parse_failed".into(),
                                message: e.to_string(),
                                details: Some(json!({ "bnkFile": bnk_file.to_string_lossy() }))
                            }
                        });
                    }
                };
                let short_name = bnk_file
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                if short_name.is_empty() {
                    continue;
                }
                let bank_id = Uuid::new_v4().to_string();
                if let Err(e) = tx.execute(
                    "INSERT INTO comment_banks(id, short_name, is_default, fit_profile, source_path)
                     VALUES(?, ?, 0, ?, ?)
                     ON CONFLICT(short_name) DO UPDATE SET
                       fit_profile = excluded.fit_profile,
                       source_path = excluded.source_path",
                    (
                        &bank_id,
                        &short_name,
                        parsed_bnk.fit_profile.as_deref(),
                        bnk_file.to_string_lossy().as_ref(),
                    ),
                ) {
                    let _ = tx.rollback();
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_insert_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "comment_banks" }))
                        }
                    });
                }

                let resolved_bank_id: String = match tx.query_row(
                    "SELECT id FROM comment_banks WHERE short_name = ?",
                    [&short_name],
                    |r| r.get(0),
                ) {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = tx.rollback();
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "db_query_failed".into(),
                                message: e.to_string(),
                                details: Some(json!({ "table": "comment_banks" }))
                            }
                        });
                    }
                };

                if let Err(e) = tx.execute(
                    "DELETE FROM comment_bank_entries WHERE bank_id = ?",
                    [&resolved_bank_id],
                ) {
                    let _ = tx.rollback();
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_delete_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "comment_bank_entries" }))
                        }
                    });
                }

                for (sort_order, entry) in parsed_bnk.entries.iter().enumerate() {
                    let eid = Uuid::new_v4().to_string();
                    if let Err(e) = tx.execute(
                        "INSERT INTO comment_bank_entries(id, bank_id, sort_order, type_code, level_code, text)
                         VALUES(?, ?, ?, ?, ?, ?)",
                        (
                            &eid,
                            &resolved_bank_id,
                            sort_order as i64,
                            &entry.type_code,
                            &entry.level_code,
                            &entry.text,
                        ),
                    ) {
                        let _ = tx.rollback();
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "db_insert_failed".into(),
                                message: e.to_string(),
                                details: Some(json!({ "table": "comment_bank_entries" }))
                            }
                        });
                    }
                }

                banks_imported += 1;
            }
        }
        Err(e) => {
            let _ = tx.rollback();
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "legacy_read_failed".into(),
                    message: e.to_string(),
                    details: Some(json!({ "folder": bnk_folder.to_string_lossy() }))
                }
            });
        }
    }

    let mut mark_sets_imported = 0usize;
    let mut assessments_imported = 0usize;
    let mut scores_imported = 0usize;
    let mut imported_mark_files: Vec<String> = Vec::new();
    let mut missing_mark_files: Vec<serde_json::Value> = Vec::new();
    let mut mark_set_id_by_source_stem: HashMap<String, String> = HashMap::new();

    for def in &parsed.mark_sets {
        let mark_file = match legacy::find_mark_file(&legacy_folder, &def.file_prefix) {
            Ok(v) => v,
            Err(e) => {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "legacy_read_failed".into(),
                        message: e.to_string(),
                        details: Some(
                            json!({ "folder": legacy_folder.to_string_lossy(), "filePrefix": def.file_prefix })
                        )
                    }
                });
            }
        };

        let Some(mark_file) = mark_file else {
            missing_mark_files.push(json!({ "code": def.code, "filePrefix": def.file_prefix }));
            continue;
        };

        let parsed_mark = match legacy::parse_legacy_mark_file(&mark_file) {
            Ok(v) => v,
            Err(e) => {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "legacy_parse_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "markFile": mark_file.to_string_lossy() }))
                    }
                });
            }
        };

        let mark_filename = mark_file
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        let mark_set_id = Uuid::new_v4().to_string();
        let misc = parsed_mark.misc.as_ref();
        let full_code = misc.map(|m| m.full_code.trim()).filter(|s| !s.is_empty());
        let room = misc.map(|m| m.room.trim()).filter(|s| !s.is_empty());
        let day = misc.map(|m| m.day.trim()).filter(|s| !s.is_empty());
        let period = misc.map(|m| m.period.trim()).filter(|s| !s.is_empty());
        let weight_method: i64 = misc.map(|m| m.weight_method as i64).unwrap_or(1);
        let calc_method: i64 = misc.map(|m| m.calc_method as i64).unwrap_or(0);
        if let Err(e) = tx.execute(
            "INSERT INTO mark_sets(
               id,
               class_id,
               code,
               file_prefix,
               description,
               weight,
               source_filename,
               sort_order,
               full_code,
               room,
               day,
               period,
               weight_method,
               calc_method
             ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            (
                &mark_set_id,
                &class_id,
                &def.code,
                &def.file_prefix,
                &def.description,
                &def.weight,
                &mark_filename,
                def.sort_order as i64,
                full_code,
                room,
                day,
                period,
                weight_method,
                calc_method,
            ),
        ) {
            let _ = tx.rollback();
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "db_insert_failed".into(),
                    message: e.to_string(),
                    details: Some(json!({ "table": "mark_sets" }))
                }
            });
        }
        if let Some(stem) = mark_file.file_stem().and_then(|s| s.to_str()) {
            mark_set_id_by_source_stem.insert(stem.to_ascii_uppercase(), mark_set_id.clone());
        }

        for (i, cat) in parsed_mark.categories.iter().enumerate() {
            let cid = Uuid::new_v4().to_string();
            if let Err(e) = tx.execute(
                "INSERT INTO categories(id, mark_set_id, name, weight, sort_order) VALUES(?, ?, ?, ?, ?)",
                (&cid, &mark_set_id, &cat.name, cat.weight, i as i64),
            ) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_insert_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "categories" }))
                    }
                });
            }
        }

        let mut assessment_ids_by_idx: Vec<String> = Vec::new();
        for a in &parsed_mark.assessments {
            let aid = Uuid::new_v4().to_string();
            if let Err(e) = tx.execute(
                "INSERT INTO assessments(id, mark_set_id, idx, date, category_name, title, term, legacy_kind, weight, out_of, avg_percent, avg_raw)
                 VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                (
                    &aid,
                    &mark_set_id,
                    a.idx as i64,
                    &a.date,
                    &a.category_name,
                    &a.title,
                    a.term,
                    a.legacy_kind,
                    a.weight,
                    a.out_of,
                    a.avg_percent,
                    a.avg_raw,
                ),
            ) {
                let _ = tx.rollback();
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_insert_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "assessments" }))
                    }
                });
            }
            assessment_ids_by_idx.push(aid);
        }

        // Insert scores with legacy mark-state parity:
        // - raw == 0  => no_mark (excluded, displays blank)
        // - raw < 0   => zero (counts as 0, displays 0)
        // - raw > 0   => scored
        for (a_idx, a) in parsed_mark.assessments.iter().enumerate() {
            let assessment_id = &assessment_ids_by_idx[a_idx];
            let max_students = std::cmp::min(student_ids_by_sort.len(), parsed_mark.last_student);
            for s_idx in 0..max_students {
                let student_id = &student_ids_by_sort[s_idx];
                let (raw_value, status) = match a.raw_scores[s_idx] {
                    legacy::LegacyScore::NoMark => (Some(0.0), "no_mark"),
                    legacy::LegacyScore::Zero => (None, "zero"),
                    legacy::LegacyScore::Scored(v) => (Some(v), "scored"),
                };
                let sid = Uuid::new_v4().to_string();
                if let Err(e) = tx.execute(
                    "INSERT INTO scores(id, assessment_id, student_id, raw_value, status) VALUES(?, ?, ?, ?, ?)",
                    (&sid, assessment_id, student_id, raw_value, status),
                ) {
                    let _ = tx.rollback();
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_insert_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "scores" }))
                        }
                    });
                }
                scores_imported += 1;
            }
        }

        // Best-effort import companions: .TYP (assessment types) and .RMK (remarks).
        // These aren't required for the grid to function, but they matter for parity.
        let typ_file = mark_file.with_extension("TYP");
        if typ_file.is_file() {
            let types = match legacy::parse_legacy_typ_file(&typ_file) {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "legacy_parse_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "typFile": typ_file.to_string_lossy() }))
                        }
                    });
                }
            };
            let max = std::cmp::min(types.len(), assessment_ids_by_idx.len());
            let mut up = match tx.prepare("UPDATE assessments SET legacy_type = ? WHERE id = ?") {
                Ok(s) => s,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_update_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "assessments" }))
                        }
                    });
                }
            };
            for i in 0..max {
                if let Err(e) = up.execute((types[i] as i64, &assessment_ids_by_idx[i])) {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_update_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "assessments" }))
                        }
                    });
                }
            }
        }

        let rmk_file = mark_file.with_extension("RMK");
        if rmk_file.is_file() {
            let rmk = match legacy::parse_legacy_rmk_file(&rmk_file) {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "legacy_parse_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "rmkFile": rmk_file.to_string_lossy() }))
                        }
                    });
                }
            };

            let max_entries =
                std::cmp::min(rmk.remarks_by_entry.len(), assessment_ids_by_idx.len());
            let max_students = std::cmp::min(student_ids_by_sort.len(), rmk.last_student);

            let mut up = match tx
                .prepare("UPDATE scores SET remark = ? WHERE assessment_id = ? AND student_id = ?")
            {
                Ok(s) => s,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_update_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "scores" }))
                        }
                    });
                }
            };

            for a_idx in 0..max_entries {
                let assessment_id = &assessment_ids_by_idx[a_idx];
                let remarks = &rmk.remarks_by_entry[a_idx];
                for s_idx in 0..max_students {
                    let remark = remarks.get(s_idx).cloned().unwrap_or_default();
                    let remark = remark.trim().to_string();
                    if remark.is_empty() {
                        continue;
                    }
                    let student_id = &student_ids_by_sort[s_idx];
                    if let Err(e) = up.execute((&remark, assessment_id, student_id)) {
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "db_update_failed".into(),
                                message: e.to_string(),
                                details: Some(json!({ "table": "scores" }))
                            }
                        });
                    }
                }
            }
        }

        // Best-effort import IDX + per-set Rn files for comment sets.
        let idx_file = mark_file.with_extension("IDX");
        if idx_file.is_file() {
            let parsed_idx = match legacy::parse_legacy_idx_file(&idx_file) {
                Ok(v) => v,
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "legacy_parse_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "idxFile": idx_file.to_string_lossy() }))
                        }
                    });
                }
            };

            // Clear existing imported sets for this mark set before writing.
            if let Err(e) = tx.execute(
                "DELETE FROM comment_set_remarks
                 WHERE comment_set_index_id IN (
                   SELECT id FROM comment_set_indexes WHERE mark_set_id = ?
                 )",
                [&mark_set_id],
            ) {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "comment_set_remarks" }))
                    }
                });
            }
            if let Err(e) = tx.execute(
                "DELETE FROM comment_set_indexes WHERE mark_set_id = ?",
                [&mark_set_id],
            ) {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_delete_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "comment_set_indexes" }))
                    }
                });
            }

            let idx_bank_short = parsed_idx.bank_short.clone();
            for set in parsed_idx.sets {
                let csi_id = Uuid::new_v4().to_string();
                let bank_short = set
                    .bank_short
                    .clone()
                    .or_else(|| idx_bank_short.clone())
                    .map(|s| s.trim().to_string())
                    .and_then(|s| if s.is_empty() { None } else { Some(s) });
                if let Err(e) = tx.execute(
                    "INSERT INTO comment_set_indexes(
                       id,
                       class_id,
                       mark_set_id,
                       set_number,
                       title,
                       fit_mode,
                       fit_font_size,
                       fit_width,
                       fit_lines,
                       fit_subj,
                       max_chars,
                       is_default,
                       bank_short
                     ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    (
                        &csi_id,
                        &class_id,
                        &mark_set_id,
                        set.set_number as i64,
                        &set.title,
                        set.fit_mode as i64,
                        set.fit_font_size as i64,
                        set.fit_width as i64,
                        set.fit_lines as i64,
                        &set.fit_subj,
                        set.max_chars as i64,
                        if set.is_default { 1 } else { 0 },
                        bank_short.as_deref(),
                    ),
                ) {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_insert_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "table": "comment_set_indexes" }))
                        }
                    });
                }

                comment_sets_imported += 1;

                let r_file = mark_file.with_extension(format!("R{}", set.set_number));
                if !r_file.is_file() {
                    continue;
                }
                let parsed_r = match legacy::parse_legacy_r_comment_file(&r_file) {
                    Ok(v) => v,
                    Err(e) => {
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "legacy_parse_failed".into(),
                                message: e.to_string(),
                                details: Some(json!({ "remarkFile": r_file.to_string_lossy() }))
                            }
                        });
                    }
                };
                let max_students = std::cmp::min(student_ids_by_sort.len(), parsed_r.remarks.len());
                for s_idx in 0..max_students {
                    let remark = parsed_r.remarks[s_idx].trim().to_string();
                    if remark.is_empty() {
                        continue;
                    }
                    let rid = Uuid::new_v4().to_string();
                    let student_id = &student_ids_by_sort[s_idx];
                    if let Err(e) = tx.execute(
                        "INSERT INTO comment_set_remarks(id, comment_set_index_id, student_id, remark)
                         VALUES(?, ?, ?, ?)
                         ON CONFLICT(comment_set_index_id, student_id) DO UPDATE SET
                           remark = excluded.remark",
                        (&rid, &csi_id, student_id, &remark),
                    ) {
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "db_insert_failed".into(),
                                message: e.to_string(),
                                details: Some(json!({ "table": "comment_set_remarks" }))
                            }
                        });
                    }
                    comment_remarks_imported += 1;
                }
            }
        }

        mark_sets_imported += 1;
        assessments_imported += parsed_mark.assessments.len();
        imported_mark_files.push(mark_filename);
    }

    // Best-effort import TBK companion files (loaned items).
    match legacy::find_tbk_files(&legacy_folder) {
        Ok(tbk_files) => {
            if tbk_files.is_empty() {
                warnings.push(json!({
                    "code": "legacy_missing_tbk_file",
                    "folder": legacy_folder.to_string_lossy()
                }));
            }
            for tbk_file in tbk_files {
                let parsed_tbk = match legacy::parse_legacy_tbk_file(&tbk_file) {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = tx.rollback();
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "legacy_parse_failed".into(),
                                message: e.to_string(),
                                details: Some(json!({ "tbkFile": tbk_file.to_string_lossy() }))
                            }
                        });
                    }
                };
                let source_stem = tbk_file
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_ascii_uppercase();
                let mark_set_id = mark_set_id_by_source_stem.get(&source_stem).cloned();
                let max_students =
                    std::cmp::min(student_ids_by_sort.len(), parsed_tbk.last_student);
                for item in parsed_tbk.items {
                    for s_idx in 0..max_students {
                        let item_id = item
                            .assignments
                            .get(s_idx)
                            .map(|a| a.item_id.trim().to_string())
                            .unwrap_or_default();
                        let note = item
                            .assignments
                            .get(s_idx)
                            .map(|a| a.note.trim().to_string())
                            .unwrap_or_default();
                        if item_id.is_empty() && note.is_empty() {
                            continue;
                        }
                        let raw_line = serde_json::to_string(&json!({
                            "title": item.title,
                            "publisher": item.publisher,
                            "cost": item.cost,
                            "itemId": item_id,
                            "note": note
                        }))
                        .unwrap_or_else(|_| "{}".to_string());
                        let loaned_id = Uuid::new_v4().to_string();
                        let student_id = &student_ids_by_sort[s_idx];
                        if let Err(e) = tx.execute(
                            "INSERT INTO loaned_items(id, class_id, student_id, mark_set_id, item_name, quantity, notes, raw_line)
                             VALUES(?, ?, ?, ?, ?, ?, ?, ?)",
                            (
                                &loaned_id,
                                &class_id,
                                student_id,
                                mark_set_id.as_deref(),
                                &item.title,
                                item.cost,
                                if note.is_empty() { None } else { Some(note.as_str()) },
                                &raw_line,
                            ),
                        ) {
                            let _ = tx.rollback();
                            return json!(ErrResp {
                                id: req.id,
                                ok: false,
                                error: ErrObj {
                                    code: "db_insert_failed".into(),
                                    message: e.to_string(),
                                    details: Some(json!({ "table": "loaned_items" }))
                                }
                            });
                        }
                        loaned_items_imported += 1;
                    }
                }
            }
        }
        Err(e) => {
            let _ = tx.rollback();
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "legacy_read_failed".into(),
                    message: e.to_string(),
                    details: Some(json!({ "folder": legacy_folder.to_string_lossy() }))
                }
            });
        }
    }

    // Best-effort merge ALL!<class>.IDX combined comment sets.
    match legacy::find_all_idx_file(&legacy_folder) {
        Ok(Some(all_idx_file)) => {
            let parsed_idx = match legacy::parse_legacy_idx_file(&all_idx_file) {
                Ok(v) => v,
                Err(e) => {
                    let _ = tx.rollback();
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "legacy_parse_failed".into(),
                            message: e.to_string(),
                            details: Some(json!({ "idxFile": all_idx_file.to_string_lossy() }))
                        }
                    });
                }
            };

            let mut mark_set_ids: Vec<String> =
                mark_set_id_by_source_stem.values().cloned().collect();
            mark_set_ids.sort();
            mark_set_ids.dedup();

            let idx_bank_short = parsed_idx.bank_short.clone();
            for mark_set_id in mark_set_ids {
                for set in &parsed_idx.sets {
                    let existing_id: Option<String> = match tx
                        .query_row(
                            "SELECT id FROM comment_set_indexes WHERE mark_set_id = ? AND set_number = ?",
                            (&mark_set_id, set.set_number as i64),
                            |r| r.get(0),
                        )
                        .optional()
                    {
                        Ok(v) => v,
                        Err(e) => {
                            let _ = tx.rollback();
                            return json!(ErrResp {
                                id: req.id,
                                ok: false,
                                error: ErrObj {
                                    code: "db_query_failed".into(),
                                    message: e.to_string(),
                                    details: Some(json!({ "table": "comment_set_indexes" }))
                                }
                            });
                        }
                    };
                    let target_set_number = if existing_id.is_some() {
                        match tx.query_row(
                            "SELECT COALESCE(MAX(set_number), 0) FROM comment_set_indexes WHERE mark_set_id = ?",
                            [&mark_set_id],
                            |r| r.get::<_, i64>(0),
                        ) {
                            Ok(v) => v + 1,
                            Err(e) => {
                                let _ = tx.rollback();
                                return json!(ErrResp {
                                    id: req.id,
                                    ok: false,
                                    error: ErrObj {
                                        code: "db_query_failed".into(),
                                        message: e.to_string(),
                                        details: Some(json!({ "table": "comment_set_indexes" }))
                                    }
                                });
                            }
                        }
                    } else {
                        set.set_number as i64
                    };

                    let csi_id = Uuid::new_v4().to_string();
                    let bank_short = set
                        .bank_short
                        .clone()
                        .or_else(|| idx_bank_short.clone())
                        .map(|s| s.trim().to_string())
                        .and_then(|s| if s.is_empty() { None } else { Some(s) });
                    if let Err(e) = tx.execute(
                        "INSERT INTO comment_set_indexes(
                           id,
                           class_id,
                           mark_set_id,
                           set_number,
                           title,
                           fit_mode,
                           fit_font_size,
                           fit_width,
                           fit_lines,
                           fit_subj,
                           max_chars,
                           is_default,
                           bank_short
                         ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                        (
                            &csi_id,
                            &class_id,
                            &mark_set_id,
                            target_set_number,
                            &set.title,
                            set.fit_mode as i64,
                            set.fit_font_size as i64,
                            set.fit_width as i64,
                            set.fit_lines as i64,
                            &set.fit_subj,
                            set.max_chars as i64,
                            if set.is_default { 1 } else { 0 },
                            bank_short.as_deref(),
                        ),
                    ) {
                        let _ = tx.rollback();
                        return json!(ErrResp {
                            id: req.id,
                            ok: false,
                            error: ErrObj {
                                code: "db_insert_failed".into(),
                                message: e.to_string(),
                                details: Some(json!({ "table": "comment_set_indexes" }))
                            }
                        });
                    }
                    comment_sets_imported += 1;
                    combined_comment_sets_imported += 1;

                    let r_file = all_idx_file.with_extension(format!("R{}", set.set_number));
                    if !r_file.is_file() {
                        continue;
                    }
                    let parsed_r = match legacy::parse_legacy_r_comment_file(&r_file) {
                        Ok(v) => v,
                        Err(e) => {
                            let _ = tx.rollback();
                            return json!(ErrResp {
                                id: req.id,
                                ok: false,
                                error: ErrObj {
                                    code: "legacy_parse_failed".into(),
                                    message: e.to_string(),
                                    details: Some(
                                        json!({ "remarkFile": r_file.to_string_lossy() })
                                    )
                                }
                            });
                        }
                    };
                    let max_students =
                        std::cmp::min(student_ids_by_sort.len(), parsed_r.remarks.len());
                    for s_idx in 0..max_students {
                        let remark = parsed_r.remarks[s_idx].trim().to_string();
                        if remark.is_empty() {
                            continue;
                        }
                        let rid = Uuid::new_v4().to_string();
                        let student_id = &student_ids_by_sort[s_idx];
                        if let Err(e) = tx.execute(
                            "INSERT INTO comment_set_remarks(id, comment_set_index_id, student_id, remark)
                             VALUES(?, ?, ?, ?)
                             ON CONFLICT(comment_set_index_id, student_id) DO UPDATE SET
                               remark = excluded.remark",
                            (&rid, &csi_id, student_id, &remark),
                        ) {
                            let _ = tx.rollback();
                            return json!(ErrResp {
                                id: req.id,
                                ok: false,
                                error: ErrObj {
                                    code: "db_insert_failed".into(),
                                    message: e.to_string(),
                                    details: Some(json!({ "table": "comment_set_remarks" }))
                                }
                            });
                        }
                        comment_remarks_imported += 1;
                    }
                }
            }
        }
        Ok(None) => {
            warnings.push(json!({
                "code": "legacy_missing_all_idx_file",
                "folder": legacy_folder.to_string_lossy()
            }));
        }
        Err(e) => {
            let _ = tx.rollback();
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "legacy_read_failed".into(),
                    message: e.to_string(),
                    details: Some(json!({ "folder": legacy_folder.to_string_lossy() }))
                }
            });
        }
    }

    if let Err(e) = tx.commit() {
        return json!(ErrResp {
            id: req.id,
            ok: false,
            error: ErrObj {
                code: "db_commit_failed".into(),
                message: e.to_string(),
                details: None
            }
        });
    }

    json!(OkResp {
        id: req.id,
        ok: true,
        result: json!({
            "classId": class_id,
            "name": class_name,
            "studentsImported": imported,
            "markSetsImported": mark_sets_imported,
            "assessmentsImported": assessments_imported,
            "scoresImported": scores_imported,
            "attendanceImported": attendance_imported,
            "seatingImported": seating_imported,
            "banksImported": banks_imported,
            "commentSetsImported": comment_sets_imported,
            "commentRemarksImported": comment_remarks_imported,
            "loanedItemsImported": loaned_items_imported,
            "deviceMappingsImported": device_mappings_imported,
            "combinedCommentSetsImported": combined_comment_sets_imported,
            "sourceClFile": cl_file.to_string_lossy(),
            "importedMarkFiles": imported_mark_files,
            "missingMarkFiles": missing_mark_files,
            "warnings": warnings,
        })
    })
}

fn import_legacy_temp_class(
    state: &mut AppState,
    req_id: &str,
    legacy_folder: &Path,
) -> Result<(String, Vec<String>, String, Vec<serde_json::Value>), serde_json::Value> {
    let tmp_req = Request {
        id: format!("{req_id}-temp-import"),
        method: "class.importLegacy".into(),
        params: json!({
            "legacyClassFolderPath": legacy_folder.to_string_lossy()
        }),
    };
    let resp = handle_class_import_legacy(state, tmp_req);
    let is_ok = resp.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    if !is_ok {
        return Err(rewrite_response_id(resp, req_id));
    }
    let result = resp.get("result").cloned().unwrap_or_else(|| json!({}));
    let class_id = result
        .get("classId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            json!(ErrResp {
                id: req_id.to_string(),
                ok: false,
                error: ErrObj {
                    code: "legacy_parse_failed".into(),
                    message: "temporary import did not return classId".into(),
                    details: None
                }
            })
        })?;
    let imported_mark_files = result
        .get("importedMarkFiles")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let source_cl_file = result
        .get("sourceClFile")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let warnings = result
        .get("warnings")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    Ok((class_id, imported_mark_files, source_cl_file, warnings))
}

fn handle_classes_legacy_preview(state: &mut AppState, req: Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return json!(ErrResp {
            id: req.id,
            ok: false,
            error: ErrObj {
                code: "no_workspace".into(),
                message: "select a workspace first".into(),
                details: None
            }
        });
    };
    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "bad_params".into(),
                    message: "missing classId".into(),
                    details: None
                }
            })
        }
    };
    let legacy_folder = req
        .params
        .get("legacyClassFolderPath")
        .and_then(|v| v.as_str())
        .map(PathBuf::from);
    let Some(legacy_folder) = legacy_folder else {
        return json!(ErrResp {
            id: req.id,
            ok: false,
            error: ErrObj {
                code: "bad_params".into(),
                message: "missing legacyClassFolderPath".into(),
                details: None
            }
        });
    };

    match class_exists(conn, &class_id) {
        Ok(true) => {}
        Ok(false) => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "not_found".into(),
                    message: "class not found".into(),
                    details: None
                }
            })
        }
        Err(e) => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "db_query_failed".into(),
                    message: e.to_string(),
                    details: None
                }
            })
        }
    }

    let cl_file = match legacy::find_cl_file(&legacy_folder) {
        Ok(v) => v,
        Err(e) => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "legacy_no_cl".into(),
                    message: e.to_string(),
                    details: Some(json!({ "folder": legacy_folder.to_string_lossy() }))
                }
            })
        }
    };
    let parsed = match legacy::parse_legacy_cl(&cl_file) {
        Ok(v) => v,
        Err(e) => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "legacy_parse_failed".into(),
                    message: e.to_string(),
                    details: Some(json!({ "clFile": cl_file.to_string_lossy() }))
                }
            })
        }
    };

    let target_students: Vec<(String, String, String, Option<String>)> = match conn.prepare(
        "SELECT id, last_name, first_name, student_no FROM students WHERE class_id = ? ORDER BY sort_order",
    ) {
        Ok(mut stmt) => match stmt
            .query_map([&class_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .and_then(|it| it.collect::<Result<Vec<_>, _>>())
        {
            Ok(v) => v,
            Err(e) => {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_query_failed".into(),
                        message: e.to_string(),
                        details: None
                    }
                })
            }
        },
        Err(e) => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "db_query_failed".into(),
                    message: e.to_string(),
                    details: None
                }
            })
        }
    };

    let mut by_student_no: HashMap<String, Vec<String>> = HashMap::new();
    let mut by_name: HashMap<String, Vec<String>> = HashMap::new();
    for (id, last, first, student_no) in &target_students {
        if let Some(no_key) = normalize_opt_key(student_no.as_deref()) {
            by_student_no.entry(no_key).or_default().push(id.clone());
        }
        by_name
            .entry(student_name_key(last, first))
            .or_default()
            .push(id.clone());
    }

    let mut used_target_ids: HashSet<String> = HashSet::new();
    let mut matched = 0usize;
    let mut new_count = 0usize;
    let mut ambiguous = 0usize;
    let mut warnings: Vec<serde_json::Value> = Vec::new();

    for (row_idx, src) in parsed.students.iter().enumerate() {
        let mut candidates: Vec<String> = Vec::new();
        let mut resolved: Option<String> = None;

        if let Some(no_key) = normalize_opt_key(src.student_no.as_deref()) {
            let ids = by_student_no.get(&no_key).cloned().unwrap_or_default();
            if ids.len() > 1 {
                candidates = ids;
            } else if ids.len() == 1 {
                let id = ids[0].clone();
                if used_target_ids.contains(&id) {
                    candidates = ids;
                } else {
                    resolved = Some(id);
                }
            }
        }
        if resolved.is_none() && candidates.is_empty() {
            let name_key = student_name_key(&src.last_name, &src.first_name);
            let ids = by_name.get(&name_key).cloned().unwrap_or_default();
            if ids.len() > 1 {
                candidates = ids;
            } else if ids.len() == 1 {
                let id = ids[0].clone();
                if used_target_ids.contains(&id) {
                    candidates = ids;
                } else {
                    resolved = Some(id);
                }
            }
        }

        if !candidates.is_empty() {
            ambiguous += 1;
            warnings.push(json!({
                "code": "ambiguous_student_match",
                "row": row_idx,
                "lastName": src.last_name,
                "firstName": src.first_name,
                "studentNo": src.student_no,
                "candidateIds": candidates
            }));
            continue;
        }

        if let Some(id) = resolved {
            used_target_ids.insert(id);
            matched += 1;
        } else {
            new_count += 1;
        }
    }

    let local_only = target_students
        .iter()
        .filter(|(id, _, _, _)| !used_target_ids.contains(id))
        .count();

    let existing_mark_set_codes: HashSet<String> =
        match conn.prepare("SELECT code FROM mark_sets WHERE class_id = ?") {
            Ok(mut stmt) => match stmt
                .query_map([&class_id], |r| r.get::<_, String>(0))
                .and_then(|it| it.collect::<Result<Vec<_>, _>>())
            {
                Ok(v) => v.into_iter().map(|s| s.to_ascii_uppercase()).collect(),
                Err(e) => {
                    return json!(ErrResp {
                        id: req.id,
                        ok: false,
                        error: ErrObj {
                            code: "db_query_failed".into(),
                            message: e.to_string(),
                            details: None
                        }
                    })
                }
            },
            Err(e) => {
                return json!(ErrResp {
                    id: req.id,
                    ok: false,
                    error: ErrObj {
                        code: "db_query_failed".into(),
                        message: e.to_string(),
                        details: None
                    }
                })
            }
        };
    let mut mark_sets_matched = 0usize;
    let mut mark_sets_new = 0usize;
    for def in &parsed.mark_sets {
        if existing_mark_set_codes.contains(&def.code.to_ascii_uppercase()) {
            mark_sets_matched += 1;
        } else {
            mark_sets_new += 1;
        }
    }

    json!(OkResp {
        id: req.id,
        ok: true,
        result: json!({
            "sourceClFile": cl_file.to_string_lossy(),
            "className": parsed.class_name,
            "classCode": cl_file.file_stem().and_then(|s| s.to_str()).map(|s| s.to_string()),
            "markSetDefs": parsed.mark_sets.iter().map(|def| json!({
                "filePrefix": def.file_prefix,
                "code": def.code,
                "description": def.description,
                "weight": def.weight,
                "sortOrder": def.sort_order
            })).collect::<Vec<_>>(),
            "students": {
                "incoming": parsed.students.len(),
                "matched": matched,
                "new": new_count,
                "ambiguous": ambiguous,
                "localOnly": local_only
            },
            "markSets": {
                "incoming": parsed.mark_sets.len(),
                "matched": mark_sets_matched,
                "new": mark_sets_new
            },
            "warnings": warnings
        })
    })
}

fn handle_classes_update_from_legacy(state: &mut AppState, req: Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return json!(ErrResp {
            id: req.id,
            ok: false,
            error: ErrObj {
                code: "no_workspace".into(),
                message: "select a workspace first".into(),
                details: None
            }
        });
    };
    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "bad_params".into(),
                    message: "missing classId".into(),
                    details: None
                }
            })
        }
    };
    match class_exists(conn, &class_id) {
        Ok(true) => {}
        Ok(false) => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "not_found".into(),
                    message: "class not found".into(),
                    details: None
                }
            })
        }
        Err(e) => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "db_query_failed".into(),
                    message: e.to_string(),
                    details: None
                }
            })
        }
    }

    let legacy_folder = req
        .params
        .get("legacyClassFolderPath")
        .and_then(|v| v.as_str())
        .map(PathBuf::from);
    let Some(legacy_folder) = legacy_folder else {
        return json!(ErrResp {
            id: req.id,
            ok: false,
            error: ErrObj {
                code: "bad_params".into(),
                message: "missing legacyClassFolderPath".into(),
                details: None
            }
        });
    };

    let mode = req
        .params
        .get("mode")
        .and_then(|v| v.as_str())
        .unwrap_or("upsert_preserve");
    if mode != "upsert_preserve" && mode != "replace_snapshot" {
        return json!(ErrResp {
            id: req.id,
            ok: false,
            error: ErrObj {
                code: "bad_params".into(),
                message: "invalid mode".into(),
                details: Some(json!({ "mode": mode }))
            }
        });
    }

    let collision_policy = req
        .params
        .get("collisionPolicy")
        .and_then(|v| v.as_str())
        .unwrap_or("merge_existing");
    if collision_policy != "merge_existing"
        && collision_policy != "append_new"
        && collision_policy != "stop_on_collision"
    {
        return json!(ErrResp {
            id: req.id,
            ok: false,
            error: ErrObj {
                code: "bad_params".into(),
                message: "invalid collisionPolicy".into(),
                details: Some(json!({ "collisionPolicy": collision_policy }))
            }
        });
    }
    let preserve_local_validity = req
        .params
        .get("preserveLocalValidity")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let (temp_class_id, imported_mark_files, source_cl_file, mut warnings) =
        match import_legacy_temp_class(state, &req.id, &legacy_folder) {
            Ok(v) => v,
            Err(resp) => return resp,
        };

    let merge_result = (|| -> Result<serde_json::Value, ErrObj> {
        let Some(conn) = state.db.as_ref() else {
            return Err(ErrObj {
                code: "no_workspace".into(),
                message: "select a workspace first".into(),
                details: None,
            });
        };
        let tx = conn.unchecked_transaction().map_err(|e| ErrObj {
            code: "db_tx_failed".into(),
            message: e.to_string(),
            details: None,
        })?;

        let target_students: Vec<(
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            i64,
            i64,
            Option<String>,
        )> = tx
            .prepare(
                "SELECT id, last_name, first_name, student_no, birth_date, active, sort_order, mark_set_mask
                 FROM students
                 WHERE class_id = ?
                 ORDER BY sort_order",
            )
            .map_err(|e| ErrObj {
                code: "db_query_failed".into(),
                message: e.to_string(),
                details: None,
            })?
            .query_map([&class_id], |r| {
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
            })
            .and_then(|it| it.collect::<Result<Vec<_>, _>>())
            .map_err(|e| ErrObj {
                code: "db_query_failed".into(),
                message: e.to_string(),
                details: None,
            })?;

        let source_students: Vec<(
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            i64,
            i64,
            String,
            Option<String>,
        )> = tx
            .prepare(
                "SELECT id, last_name, first_name, student_no, birth_date, active, sort_order, raw_line, mark_set_mask
                 FROM students
                 WHERE class_id = ?
                 ORDER BY sort_order",
            )
            .map_err(|e| ErrObj {
                code: "db_query_failed".into(),
                message: e.to_string(),
                details: None,
            })?
            .query_map([&temp_class_id], |r| {
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
            })
            .and_then(|it| it.collect::<Result<Vec<_>, _>>())
            .map_err(|e| ErrObj {
                code: "db_query_failed".into(),
                message: e.to_string(),
                details: None,
            })?;

        let mut by_student_no: HashMap<String, Vec<String>> = HashMap::new();
        let mut by_name: HashMap<String, Vec<String>> = HashMap::new();
        for (id, last, first, student_no, _, _, _, _) in &target_students {
            if let Some(no_key) = normalize_opt_key(student_no.as_deref()) {
                by_student_no.entry(no_key).or_default().push(id.clone());
            }
            by_name
                .entry(student_name_key(last, first))
                .or_default()
                .push(id.clone());
        }

        let mut used_target_ids: HashSet<String> = HashSet::new();
        let mut source_to_target_student: HashMap<String, String> = HashMap::new();
        let mut desired_order: Vec<String> = Vec::new();
        let mut students_matched = 0usize;
        let mut students_created = 0usize;
        let mut students_updated = 0usize;
        let mut students_ambiguous_skipped = 0usize;

        for (row_idx, src) in source_students.iter().enumerate() {
            let (
                source_student_id,
                src_last_name,
                src_first_name,
                src_student_no,
                src_birth_date,
                src_active,
                _src_sort,
                src_raw_line,
                src_mark_set_mask,
            ) = src;

            let mut resolved_target_id: Option<String> = None;
            let mut ambiguous_candidates: Vec<String> = Vec::new();

            if let Some(no_key) = normalize_opt_key(src_student_no.as_deref()) {
                let ids = by_student_no.get(&no_key).cloned().unwrap_or_default();
                if ids.len() > 1 {
                    ambiguous_candidates = ids;
                } else if ids.len() == 1 {
                    let id = ids[0].clone();
                    if used_target_ids.contains(&id) {
                        ambiguous_candidates = ids;
                    } else {
                        resolved_target_id = Some(id);
                    }
                }
            }

            if resolved_target_id.is_none() && ambiguous_candidates.is_empty() {
                let name_key = student_name_key(src_last_name, src_first_name);
                let ids = by_name.get(&name_key).cloned().unwrap_or_default();
                if ids.len() > 1 {
                    ambiguous_candidates = ids;
                } else if ids.len() == 1 {
                    let id = ids[0].clone();
                    if used_target_ids.contains(&id) {
                        ambiguous_candidates = ids;
                    } else {
                        resolved_target_id = Some(id);
                    }
                }
            }

            if !ambiguous_candidates.is_empty() {
                students_ambiguous_skipped += 1;
                warnings.push(json!({
                    "code": "ambiguous_student_match",
                    "row": row_idx,
                    "lastName": src_last_name,
                    "firstName": src_first_name,
                    "studentNo": src_student_no,
                    "candidateIds": ambiguous_candidates
                }));
                continue;
            }

            if let Some(target_student_id) = resolved_target_id {
                used_target_ids.insert(target_student_id.clone());
                let source_mask = src_mark_set_mask
                    .clone()
                    .unwrap_or_else(|| "TBA".to_string());
                let updated = if preserve_local_validity {
                    tx.execute(
                        "UPDATE students
                         SET last_name = ?,
                             first_name = ?,
                             student_no = ?,
                             birth_date = ?,
                             raw_line = ?,
                             updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
                         WHERE id = ?",
                        (
                            src_last_name,
                            src_first_name,
                            src_student_no,
                            src_birth_date,
                            src_raw_line,
                            &target_student_id,
                        ),
                    )
                } else {
                    tx.execute(
                        "UPDATE students
                         SET last_name = ?,
                             first_name = ?,
                             student_no = ?,
                             birth_date = ?,
                             raw_line = ?,
                             active = ?,
                             mark_set_mask = ?,
                             updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
                         WHERE id = ?",
                        (
                            src_last_name,
                            src_first_name,
                            src_student_no,
                            src_birth_date,
                            src_raw_line,
                            src_active,
                            &source_mask,
                            &target_student_id,
                        ),
                    )
                };
                updated.map_err(|e| ErrObj {
                    code: "db_update_failed".into(),
                    message: e.to_string(),
                    details: Some(json!({ "table": "students", "id": target_student_id })),
                })?;
                students_matched += 1;
                students_updated += 1;
                source_to_target_student
                    .insert(source_student_id.clone(), target_student_id.clone());
                desired_order.push(target_student_id);
            } else {
                let new_student_id = Uuid::new_v4().to_string();
                let source_mask = src_mark_set_mask
                    .clone()
                    .unwrap_or_else(|| "TBA".to_string());
                tx.execute(
                    "INSERT INTO students(
                        id,
                        class_id,
                        last_name,
                        first_name,
                        student_no,
                        birth_date,
                        active,
                        sort_order,
                        raw_line,
                        mark_set_mask,
                        updated_at
                     ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, strftime('%Y-%m-%dT%H:%M:%SZ','now'))",
                    (
                        &new_student_id,
                        &class_id,
                        src_last_name,
                        src_first_name,
                        src_student_no,
                        src_birth_date,
                        src_active,
                        0i64,
                        src_raw_line,
                        &source_mask,
                    ),
                )
                .map_err(|e| ErrObj {
                    code: "db_insert_failed".into(),
                    message: e.to_string(),
                    details: Some(json!({ "table": "students" })),
                })?;
                students_created += 1;
                source_to_target_student.insert(source_student_id.clone(), new_student_id.clone());
                desired_order.push(new_student_id);
            }
        }

        let local_only_ids: Vec<String> = target_students
            .iter()
            .filter(|(id, _, _, _, _, _, _, _)| !used_target_ids.contains(id))
            .map(|(id, ..)| id.clone())
            .collect();
        let students_local_only = local_only_ids.len();
        desired_order.extend(local_only_ids.clone());

        for (idx, student_id) in desired_order.iter().enumerate() {
            tx.execute(
                "UPDATE students
                 SET sort_order = ?,
                     updated_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
                 WHERE id = ?",
                (idx as i64, student_id),
            )
            .map_err(|e| ErrObj {
                code: "db_update_failed".into(),
                message: e.to_string(),
                details: Some(json!({ "table": "students", "id": student_id })),
            })?;
        }

        let source_mark_sets: Vec<(
            String,
            String,
            String,
            String,
            Option<f64>,
            Option<String>,
            i64,
            Option<String>,
            Option<String>,
            Option<String>,
            Option<String>,
            i64,
            i64,
            Option<String>,
        )> = tx
            .prepare(
                "SELECT
                    id,
                    code,
                    file_prefix,
                    description,
                    weight,
                    source_filename,
                    sort_order,
                    full_code,
                    room,
                    day,
                    period,
                    weight_method,
                    calc_method,
                    block_title
                 FROM mark_sets
                 WHERE class_id = ?
                 ORDER BY sort_order",
            )
            .map_err(|e| ErrObj {
                code: "db_query_failed".into(),
                message: e.to_string(),
                details: None,
            })?
            .query_map([&temp_class_id], |r| {
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
                    r.get(11)?,
                    r.get(12)?,
                    r.get(13)?,
                ))
            })
            .and_then(|it| it.collect::<Result<Vec<_>, _>>())
            .map_err(|e| ErrObj {
                code: "db_query_failed".into(),
                message: e.to_string(),
                details: None,
            })?;

        let target_mark_sets: Vec<(String, String, Option<String>)> = tx
            .prepare("SELECT id, code, deleted_at FROM mark_sets WHERE class_id = ?")
            .map_err(|e| ErrObj {
                code: "db_query_failed".into(),
                message: e.to_string(),
                details: None,
            })?
            .query_map([&class_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .and_then(|it| it.collect::<Result<Vec<_>, _>>())
            .map_err(|e| ErrObj {
                code: "db_query_failed".into(),
                message: e.to_string(),
                details: None,
            })?;

        let mut target_mark_set_by_code: HashMap<String, Vec<(String, Option<String>)>> =
            HashMap::new();
        for (id, code, deleted_at) in target_mark_sets {
            target_mark_set_by_code
                .entry(code.to_ascii_uppercase())
                .or_default()
                .push((id, deleted_at));
        }

        let mut mark_sets_matched = 0usize;
        let mut mark_sets_created = 0usize;
        let mut mark_sets_undeleted = 0usize;
        let mut source_to_target_mark_set: HashMap<String, String> = HashMap::new();
        let mut touched_target_mark_sets: HashSet<String> = HashSet::new();
        let mut source_code_set: HashSet<String> = HashSet::new();

        for source_ms in &source_mark_sets {
            let (
                source_ms_id,
                code,
                file_prefix,
                description,
                weight,
                source_filename,
                sort_order,
                full_code,
                room,
                day,
                period,
                weight_method,
                calc_method,
                block_title,
            ) = source_ms;

            source_code_set.insert(code.to_ascii_uppercase());
            let maybe_targets = target_mark_set_by_code
                .get(&code.to_ascii_uppercase())
                .cloned()
                .unwrap_or_default();

            let target_mark_set_id = if maybe_targets.is_empty() {
                let new_mark_set_id = Uuid::new_v4().to_string();
                tx.execute(
                    "INSERT INTO mark_sets(
                        id,
                        class_id,
                        code,
                        file_prefix,
                        description,
                        weight,
                        source_filename,
                        sort_order,
                        full_code,
                        room,
                        day,
                        period,
                        weight_method,
                        calc_method,
                        is_default,
                        deleted_at,
                        block_title
                     ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, NULL, ?)",
                    (
                        &new_mark_set_id,
                        &class_id,
                        code,
                        file_prefix,
                        description,
                        weight,
                        source_filename,
                        sort_order,
                        full_code,
                        room,
                        day,
                        period,
                        weight_method,
                        calc_method,
                        block_title,
                    ),
                )
                .map_err(|e| ErrObj {
                    code: "db_insert_failed".into(),
                    message: e.to_string(),
                    details: Some(json!({ "table": "mark_sets", "code": code })),
                })?;
                mark_sets_created += 1;
                new_mark_set_id
            } else {
                if maybe_targets.len() > 1 {
                    warnings.push(json!({
                        "code": "duplicate_target_mark_set_code",
                        "markSetCode": code,
                        "targetIds": maybe_targets.iter().map(|(id, _)| id.clone()).collect::<Vec<_>>()
                    }));
                }
                let (existing_mark_set_id, deleted_at) = maybe_targets[0].clone();
                tx.execute(
                    "UPDATE mark_sets
                     SET file_prefix = ?,
                         description = ?,
                         weight = ?,
                         source_filename = ?,
                         sort_order = ?,
                         full_code = ?,
                         room = ?,
                         day = ?,
                         period = ?,
                         weight_method = ?,
                         calc_method = ?,
                         deleted_at = NULL,
                         block_title = ?
                     WHERE id = ? AND class_id = ?",
                    (
                        file_prefix,
                        description,
                        weight,
                        source_filename,
                        sort_order,
                        full_code,
                        room,
                        day,
                        period,
                        weight_method,
                        calc_method,
                        block_title,
                        &existing_mark_set_id,
                        &class_id,
                    ),
                )
                .map_err(|e| ErrObj {
                    code: "db_update_failed".into(),
                    message: e.to_string(),
                    details: Some(json!({ "table": "mark_sets", "id": existing_mark_set_id })),
                })?;
                mark_sets_matched += 1;
                if deleted_at.is_some() {
                    mark_sets_undeleted += 1;
                }
                existing_mark_set_id
            };

            touched_target_mark_sets.insert(target_mark_set_id.clone());
            source_to_target_mark_set.insert(source_ms_id.clone(), target_mark_set_id);
        }

        if mode == "replace_snapshot" {
            let target_ids: Vec<String> = tx
                .prepare("SELECT id, code FROM mark_sets WHERE class_id = ?")
                .map_err(|e| ErrObj {
                    code: "db_query_failed".into(),
                    message: e.to_string(),
                    details: None,
                })?
                .query_map([&class_id], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                })
                .and_then(|it| it.collect::<Result<Vec<_>, _>>())
                .map_err(|e| ErrObj {
                    code: "db_query_failed".into(),
                    message: e.to_string(),
                    details: None,
                })?
                .into_iter()
                .filter_map(|(id, code)| {
                    if source_code_set.contains(&code.to_ascii_uppercase()) {
                        None
                    } else {
                        Some(id)
                    }
                })
                .collect();
            for id in target_ids {
                tx.execute(
                    "UPDATE mark_sets
                     SET deleted_at = strftime('%Y-%m-%dT%H:%M:%fZ','now'),
                         is_default = 0
                     WHERE id = ?",
                    [&id],
                )
                .map_err(|e| ErrObj {
                    code: "db_update_failed".into(),
                    message: e.to_string(),
                    details: Some(json!({ "table": "mark_sets", "id": id })),
                })?;
            }
        }

        let mut assessments_matched = 0usize;
        let mut assessments_created = 0usize;
        let mut assessments_updated = 0usize;
        let mut scores_upserted = 0usize;

        for (source_mark_set_id, target_mark_set_id) in source_to_target_mark_set {
            tx.execute(
                "DELETE FROM categories WHERE mark_set_id = ?",
                [&target_mark_set_id],
            )
            .map_err(|e| ErrObj {
                code: "db_delete_failed".into(),
                message: e.to_string(),
                details: Some(json!({ "table": "categories", "markSetId": target_mark_set_id })),
            })?;

            let source_categories: Vec<(String, Option<f64>, i64)> = tx
                .prepare(
                    "SELECT name, weight, sort_order
                     FROM categories
                     WHERE mark_set_id = ?
                     ORDER BY sort_order",
                )
                .map_err(|e| ErrObj {
                    code: "db_query_failed".into(),
                    message: e.to_string(),
                    details: None,
                })?
                .query_map([&source_mark_set_id], |r| {
                    Ok((r.get(0)?, r.get(1)?, r.get(2)?))
                })
                .and_then(|it| it.collect::<Result<Vec<_>, _>>())
                .map_err(|e| ErrObj {
                    code: "db_query_failed".into(),
                    message: e.to_string(),
                    details: None,
                })?;

            for (name, weight, sort_order) in source_categories {
                tx.execute(
                    "INSERT INTO categories(id, mark_set_id, name, weight, sort_order)
                     VALUES(?, ?, ?, ?, ?)",
                    (
                        Uuid::new_v4().to_string(),
                        &target_mark_set_id,
                        &name,
                        weight,
                        sort_order,
                    ),
                )
                .map_err(|e| ErrObj {
                    code: "db_insert_failed".into(),
                    message: e.to_string(),
                    details: Some(
                        json!({ "table": "categories", "markSetId": target_mark_set_id }),
                    ),
                })?;
            }

            let source_assessments: Vec<(
                String,
                i64,
                Option<String>,
                Option<String>,
                String,
                Option<i64>,
                Option<i64>,
                Option<i64>,
                Option<f64>,
                Option<f64>,
                Option<f64>,
                Option<f64>,
            )> = tx
                .prepare(
                    "SELECT
                        id,
                        idx,
                        date,
                        category_name,
                        title,
                        term,
                        legacy_kind,
                        legacy_type,
                        weight,
                        out_of,
                        avg_percent,
                        avg_raw
                     FROM assessments
                     WHERE mark_set_id = ?
                     ORDER BY idx",
                )
                .map_err(|e| ErrObj {
                    code: "db_query_failed".into(),
                    message: e.to_string(),
                    details: None,
                })?
                .query_map([&source_mark_set_id], |r| {
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
                        r.get(11)?,
                    ))
                })
                .and_then(|it| it.collect::<Result<Vec<_>, _>>())
                .map_err(|e| ErrObj {
                    code: "db_query_failed".into(),
                    message: e.to_string(),
                    details: None,
                })?;

            let target_assessments: Vec<(
                String,
                i64,
                Option<String>,
                Option<String>,
                String,
                Option<i64>,
            )> = tx
                .prepare(
                    "SELECT id, idx, date, category_name, title, term
                     FROM assessments
                     WHERE mark_set_id = ?
                     ORDER BY idx",
                )
                .map_err(|e| ErrObj {
                    code: "db_query_failed".into(),
                    message: e.to_string(),
                    details: None,
                })?
                .query_map([&target_mark_set_id], |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                    ))
                })
                .and_then(|it| it.collect::<Result<Vec<_>, _>>())
                .map_err(|e| ErrObj {
                    code: "db_query_failed".into(),
                    message: e.to_string(),
                    details: None,
                })?;

            let mut target_by_key: HashMap<String, Vec<(String, i64)>> = HashMap::new();
            let mut next_idx = target_assessments
                .iter()
                .map(|(_, idx, _, _, _, _)| *idx)
                .max()
                .unwrap_or(-1)
                + 1;
            for (id, idx, date, category_name, title, term) in target_assessments {
                let key = assessment_collision_key(
                    date.as_deref(),
                    category_name.as_deref(),
                    &title,
                    term,
                );
                target_by_key.entry(key).or_default().push((id, idx));
            }

            let mut source_to_target_assessment: HashMap<String, String> = HashMap::new();
            for source_assessment in source_assessments {
                let (
                    source_assessment_id,
                    _source_idx,
                    source_date,
                    source_category_name,
                    source_title,
                    source_term,
                    source_legacy_kind,
                    source_legacy_type,
                    source_weight,
                    source_out_of,
                    source_avg_percent,
                    source_avg_raw,
                ) = source_assessment;
                let key = assessment_collision_key(
                    source_date.as_deref(),
                    source_category_name.as_deref(),
                    &source_title,
                    source_term,
                );

                let mut matched_target: Option<(String, i64)> = None;
                if collision_policy != "append_new" {
                    if let Some(candidates) = target_by_key.get_mut(&key) {
                        if let Some(first) = candidates.first().cloned() {
                            if collision_policy == "stop_on_collision" {
                                return Err(ErrObj {
                                    code: "collision_conflict".into(),
                                    message: "assessment collision detected".into(),
                                    details: Some(json!({
                                        "markSetId": target_mark_set_id,
                                        "sourceAssessmentId": source_assessment_id,
                                        "sourceTitle": source_title,
                                        "targetAssessmentId": first.0,
                                        "collisionKey": key
                                    })),
                                });
                            }
                            matched_target = Some(first);
                            candidates.remove(0);
                        }
                    }
                }

                let target_assessment_id = if let Some((existing_assessment_id, _)) = matched_target
                {
                    tx.execute(
                        "UPDATE assessments
                         SET date = ?,
                             category_name = ?,
                             title = ?,
                             term = ?,
                             legacy_kind = ?,
                             legacy_type = ?,
                             weight = ?,
                             out_of = ?,
                             avg_percent = ?,
                             avg_raw = ?
                         WHERE id = ?",
                        (
                            source_date.as_deref(),
                            source_category_name.as_deref(),
                            &source_title,
                            source_term,
                            source_legacy_kind,
                            source_legacy_type,
                            source_weight,
                            source_out_of,
                            source_avg_percent,
                            source_avg_raw,
                            &existing_assessment_id,
                        ),
                    )
                    .map_err(|e| ErrObj {
                        code: "db_update_failed".into(),
                        message: e.to_string(),
                        details: Some(
                            json!({ "table": "assessments", "id": existing_assessment_id }),
                        ),
                    })?;
                    assessments_matched += 1;
                    assessments_updated += 1;
                    existing_assessment_id
                } else {
                    let new_assessment_id = Uuid::new_v4().to_string();
                    tx.execute(
                        "INSERT INTO assessments(
                            id,
                            mark_set_id,
                            idx,
                            date,
                            category_name,
                            title,
                            term,
                            legacy_kind,
                            legacy_type,
                            weight,
                            out_of,
                            avg_percent,
                            avg_raw
                         ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                        (
                            &new_assessment_id,
                            &target_mark_set_id,
                            next_idx,
                            source_date.as_deref(),
                            source_category_name.as_deref(),
                            &source_title,
                            source_term,
                            source_legacy_kind,
                            source_legacy_type,
                            source_weight,
                            source_out_of,
                            source_avg_percent,
                            source_avg_raw,
                        ),
                    )
                    .map_err(|e| ErrObj {
                        code: "db_insert_failed".into(),
                        message: e.to_string(),
                        details: Some(
                            json!({ "table": "assessments", "markSetId": target_mark_set_id }),
                        ),
                    })?;
                    next_idx += 1;
                    assessments_created += 1;
                    new_assessment_id
                };

                source_to_target_assessment.insert(source_assessment_id, target_assessment_id);
            }

            for (source_assessment_id, target_assessment_id) in source_to_target_assessment {
                let source_scores: Vec<(String, Option<f64>, String, Option<String>)> = tx
                    .prepare(
                        "SELECT student_id, raw_value, status, remark
                         FROM scores
                         WHERE assessment_id = ?",
                    )
                    .map_err(|e| ErrObj {
                        code: "db_query_failed".into(),
                        message: e.to_string(),
                        details: None,
                    })?
                    .query_map([&source_assessment_id], |r| {
                        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
                    })
                    .and_then(|it| it.collect::<Result<Vec<_>, _>>())
                    .map_err(|e| ErrObj {
                        code: "db_query_failed".into(),
                        message: e.to_string(),
                        details: None,
                    })?;

                for (source_student_id, raw_value, status, remark) in source_scores {
                    let Some(target_student_id) = source_to_target_student.get(&source_student_id)
                    else {
                        warnings.push(json!({
                            "code": "missing_target_student_for_score",
                            "sourceStudentId": source_student_id,
                            "sourceAssessmentId": source_assessment_id
                        }));
                        continue;
                    };
                    let score_id = Uuid::new_v4().to_string();
                    tx.execute(
                        "INSERT INTO scores(id, assessment_id, student_id, raw_value, status, remark)
                         VALUES(?, ?, ?, ?, ?, ?)
                         ON CONFLICT(assessment_id, student_id) DO UPDATE SET
                           raw_value = excluded.raw_value,
                           status = excluded.status,
                           remark = excluded.remark",
                        (
                            &score_id,
                            &target_assessment_id,
                            target_student_id,
                            raw_value,
                            &status,
                            remark.as_deref(),
                        ),
                    )
                    .map_err(|e| ErrObj {
                        code: "db_insert_failed".into(),
                        message: e.to_string(),
                        details: Some(json!({ "table": "scores", "assessmentId": target_assessment_id })),
                    })?;
                    scores_upserted += 1;
                }
            }
        }

        tx.execute(
            "INSERT INTO class_meta(class_id, created_from_wizard)
             VALUES(?, 0)
             ON CONFLICT(class_id) DO NOTHING",
            [&class_id],
        )
        .map_err(|e| ErrObj {
            code: "db_insert_failed".into(),
            message: e.to_string(),
            details: Some(json!({ "table": "class_meta" })),
        })?;
        let legacy_year_token = class_meta_year_token_from_cl_file(Path::new(&source_cl_file));
        tx.execute(
            "UPDATE class_meta
             SET legacy_folder_path = ?,
                 legacy_cl_file = ?,
                 legacy_year_token = ?,
                 last_imported_at = strftime('%Y-%m-%dT%H:%M:%SZ','now')
             WHERE class_id = ?",
            (
                legacy_folder.to_string_lossy().to_string(),
                &source_cl_file,
                legacy_year_token,
                &class_id,
            ),
        )
        .map_err(|e| ErrObj {
            code: "db_update_failed".into(),
            message: e.to_string(),
            details: Some(json!({ "table": "class_meta", "classId": class_id })),
        })?;

        let warnings_key = format!("classes.lastImportWarnings.{class_id}");
        let warnings_json = serde_json::to_string(&warnings).unwrap_or_else(|_| "[]".to_string());
        tx.execute(
            "INSERT INTO workspace_settings(key, value_json)
             VALUES(?, ?)
             ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json",
            (&warnings_key, &warnings_json),
        )
        .map_err(|e| ErrObj {
            code: "db_update_failed".into(),
            message: e.to_string(),
            details: Some(json!({ "table": "workspace_settings", "key": warnings_key })),
        })?;

        tx.commit().map_err(|e| ErrObj {
            code: "db_tx_failed".into(),
            message: e.to_string(),
            details: None,
        })?;

        Ok(json!({
            "ok": true,
            "classId": class_id,
            "students": {
                "matched": students_matched,
                "created": students_created,
                "updated": students_updated,
                "localOnly": students_local_only,
                "ambiguousSkipped": students_ambiguous_skipped
            },
            "markSets": {
                "matched": mark_sets_matched,
                "created": mark_sets_created,
                "undeleted": mark_sets_undeleted
            },
            "assessments": {
                "matched": assessments_matched,
                "created": assessments_created,
                "updated": assessments_updated
            },
            "scores": {
                "upserted": scores_upserted
            },
            "warnings": warnings,
            "sourceClFile": source_cl_file,
            "importedMarkFiles": imported_mark_files
        }))
    })();

    cleanup_temp_class(state, &temp_class_id);

    match merge_result {
        Ok(result) => json!(OkResp {
            id: req.id,
            ok: true,
            result
        }),
        Err(error) => json!(ErrResp {
            id: req.id,
            ok: false,
            error
        }),
    }
}

fn handle_marksets_list(state: &mut AppState, req: Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return json!(ErrResp {
            id: req.id,
            ok: false,
            error: ErrObj {
                code: "no_workspace".into(),
                message: "select a workspace first".into(),
                details: None
            }
        });
    };
    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "bad_params".into(),
                    message: "missing classId".into(),
                    details: None
                }
            })
        }
    };

    let include_deleted = req
        .params
        .get("includeDeleted")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mut stmt = match conn.prepare(
        "SELECT id, code, description, sort_order, is_default, deleted_at
         FROM mark_sets
         WHERE class_id = ?
           AND (? OR deleted_at IS NULL)
         ORDER BY sort_order",
    ) {
        Ok(s) => s,
        Err(e) => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "db_query_failed".into(),
                    message: e.to_string(),
                    details: None
                }
            })
        }
    };

    let rows = stmt
        .query_map((&class_id, include_deleted), |row| {
            let id: String = row.get(0)?;
            let code: String = row.get(1)?;
            let description: String = row.get(2)?;
            let sort_order: i64 = row.get(3)?;
            let is_default: i64 = row.get(4)?;
            let deleted_at: Option<String> = row.get(5)?;
            Ok(json!({
                "id": id,
                "code": code,
                "description": description,
                "sortOrder": sort_order,
                "isDefault": is_default != 0,
                "deletedAt": deleted_at
            }))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>());

    match rows {
        Ok(mark_sets) => json!(OkResp {
            id: req.id,
            ok: true,
            result: json!({ "markSets": mark_sets })
        }),
        Err(e) => json!(ErrResp {
            id: req.id,
            ok: false,
            error: ErrObj {
                code: "db_query_failed".into(),
                message: e.to_string(),
                details: None
            }
        }),
    }
}

fn handle_markset_open(state: &mut AppState, req: Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return json!(ErrResp {
            id: req.id,
            ok: false,
            error: ErrObj {
                code: "no_workspace".into(),
                message: "select a workspace first".into(),
                details: None
            }
        });
    };
    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "bad_params".into(),
                    message: "missing classId".into(),
                    details: None
                }
            })
        }
    };
    let mark_set_id = match req.params.get("markSetId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "bad_params".into(),
                    message: "missing markSetId".into(),
                    details: None
                }
            })
        }
    };

    let ms_row: Option<(String, String, String)> = match conn
        .query_row(
            "SELECT id, code, description
             FROM mark_sets
             WHERE id = ? AND class_id = ? AND deleted_at IS NULL",
            (&mark_set_id, &class_id),
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
    {
        Ok(v) => v,
        Err(e) => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "db_query_failed".into(),
                    message: e.to_string(),
                    details: None
                }
            })
        }
    };
    let Some((ms_id, ms_code, ms_desc)) = ms_row else {
        return json!(ErrResp {
            id: req.id,
            ok: false,
            error: ErrObj {
                code: "not_found".into(),
                message: "mark set not found".into(),
                details: None
            }
        });
    };

    let mut stud_stmt = match conn.prepare(
        "SELECT id, last_name, first_name, sort_order, active FROM students WHERE class_id = ? ORDER BY sort_order",
    ) {
        Ok(s) => s,
        Err(e) => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "db_query_failed".into(),
                    message: e.to_string(),
                    details: None
                }
            })
        }
    };
    let students_json: Vec<serde_json::Value> = match stud_stmt
        .query_map([&class_id], |row| {
            let id: String = row.get(0)?;
            let last: String = row.get(1)?;
            let first: String = row.get(2)?;
            let sort_order: i64 = row.get(3)?;
            let active: i64 = row.get(4)?;
            let display_name = format!("{}, {}", last, first);
            Ok(json!({
                "id": id,
                "displayName": display_name,
                "sortOrder": sort_order,
                "active": active != 0
            }))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
    {
        Ok(v) => v,
        Err(e) => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "db_query_failed".into(),
                    message: e.to_string(),
                    details: None
                }
            })
        }
    };

    let mut assess_stmt = match conn.prepare(
        "SELECT id, idx, date, category_name, title, weight, out_of FROM assessments WHERE mark_set_id = ? ORDER BY idx",
    ) {
        Ok(s) => s,
        Err(e) => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "db_query_failed".into(),
                    message: e.to_string(),
                    details: None
                }
            })
        }
    };
    let assessments_json: Vec<serde_json::Value> = match assess_stmt
        .query_map([&ms_id], |row| {
            let id: String = row.get(0)?;
            let idx: i64 = row.get(1)?;
            let date: Option<String> = row.get(2)?;
            let category_name: Option<String> = row.get(3)?;
            let title: String = row.get(4)?;
            let weight: Option<f64> = row.get(5)?;
            let out_of: Option<f64> = row.get(6)?;
            Ok(json!({
                "id": id,
                "idx": idx,
                "date": date,
                "categoryName": category_name,
                "title": title,
                "weight": weight,
                "outOf": out_of
            }))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>())
    {
        Ok(v) => v,
        Err(e) => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "db_query_failed".into(),
                    message: e.to_string(),
                    details: None
                }
            })
        }
    };

    json!(OkResp {
        id: req.id,
        ok: true,
        result: json!({
            "markSet": { "id": ms_id, "code": ms_code, "description": ms_desc },
            "students": students_json,
            "assessments": assessments_json,
            "rowCount": students_json.len(),
            "colCount": assessments_json.len()
        })
    })
}

fn handle_classes_update_from_attached_legacy(
    state: &mut AppState,
    req: Request,
) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return json!(ErrResp {
            id: req.id,
            ok: false,
            error: ErrObj {
                code: "no_workspace".into(),
                message: "select a workspace first".into(),
                details: None
            }
        });
    };

    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "bad_params".into(),
                    message: "missing classId".into(),
                    details: None
                }
            })
        }
    };

    match class_exists(conn, &class_id) {
        Ok(true) => {}
        Ok(false) => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "not_found".into(),
                    message: "class not found".into(),
                    details: None
                }
            })
        }
        Err(e) => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "db_query_failed".into(),
                    message: e.to_string(),
                    details: None
                }
            })
        }
    }

    let attached_path: Option<String> = match conn
        .query_row(
            "SELECT legacy_folder_path FROM class_meta WHERE class_id = ?",
            [&class_id],
            |r| r.get(0),
        )
        .optional()
    {
        Ok(v) => v,
        Err(e) => {
            return json!(ErrResp {
                id: req.id,
                ok: false,
                error: ErrObj {
                    code: "db_query_failed".into(),
                    message: e.to_string(),
                    details: None
                }
            })
        }
    };
    let attached_path = attached_path
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let Some(attached_path) = attached_path else {
        return json!(ErrResp {
            id: req.id,
            ok: false,
            error: ErrObj {
                code: "not_found".into(),
                message: "no attached legacy folder for class".into(),
                details: Some(json!({ "classId": class_id }))
            }
        });
    };

    let mut params = serde_json::Map::new();
    params.insert("classId".to_string(), json!(class_id));
    params.insert("legacyClassFolderPath".to_string(), json!(attached_path));
    if let Some(v) = req.params.get("mode") {
        params.insert("mode".to_string(), v.clone());
    }
    if let Some(v) = req.params.get("collisionPolicy") {
        params.insert("collisionPolicy".to_string(), v.clone());
    }
    if let Some(v) = req.params.get("preserveLocalValidity") {
        params.insert("preserveLocalValidity".to_string(), v.clone());
    }

    let proxy_req = Request {
        id: req.id,
        method: "classes.updateFromLegacy".to_string(),
        params: serde_json::Value::Object(params),
    };
    handle_classes_update_from_legacy(state, proxy_req)
}

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "class.importLegacy" => Some(handle_class_import_legacy(state, req.clone())),
        "classes.legacyPreview" => Some(handle_classes_legacy_preview(state, req.clone())),
        "classes.updateFromLegacy" => Some(handle_classes_update_from_legacy(state, req.clone())),
        "classes.updateFromAttachedLegacy" => {
            Some(handle_classes_update_from_attached_legacy(state, req.clone()))
        }
        "marksets.list" => Some(handle_marksets_list(state, req.clone())),
        "markset.open" => Some(handle_markset_open(state, req.clone())),
        _ => None,
    }
}
