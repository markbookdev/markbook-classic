use crate::ipc::types::{AppState, Request};
use crate::legacy;
use rusqlite::OptionalExtension;
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
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

    let mut stmt = match conn.prepare(
        "SELECT id, code, description, sort_order FROM mark_sets WHERE class_id = ? ORDER BY sort_order",
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
        .query_map([&class_id], |row| {
            let id: String = row.get(0)?;
            let code: String = row.get(1)?;
            let description: String = row.get(2)?;
            let sort_order: i64 = row.get(3)?;
            Ok(json!({ "id": id, "code": code, "description": description, "sortOrder": sort_order }))
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
            "SELECT id, code, description FROM mark_sets WHERE id = ? AND class_id = ?",
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

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "class.importLegacy" => Some(handle_class_import_legacy(state, req.clone())),
        "marksets.list" => Some(handle_marksets_list(state, req.clone())),
        "markset.open" => Some(handle_markset_open(state, req.clone())),
        _ => None,
    }
}
