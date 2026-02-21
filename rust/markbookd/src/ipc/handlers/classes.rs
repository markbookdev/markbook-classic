use crate::db;
use crate::ipc::error::{err, ok};
use crate::ipc::types::{AppState, Request};
use rusqlite::types::Value;
use rusqlite::OptionalExtension;
use serde_json::json;
use uuid::Uuid;

fn handle_classes_list(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return ok(&req.id, json!({ "classes": [] }));
    };

    // Include basic counts so the UI can show a useful dashboard.
    // Use correlated subqueries to avoid double-counting from joins.
    let mut stmt = match conn.prepare(
        "SELECT
           c.id,
           c.name,
           (SELECT COUNT(*) FROM students s WHERE s.class_id = c.id) AS student_count,
           (SELECT COUNT(*) FROM mark_sets ms WHERE ms.class_id = c.id AND ms.deleted_at IS NULL) AS mark_set_count
         FROM classes c
         ORDER BY c.name",
    ) {
        Ok(s) => s,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    let rows = stmt
        .query_map([], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let student_count: i64 = row.get(2)?;
            let mark_set_count: i64 = row.get(3)?;
            Ok(json!({
                "id": id,
                "name": name,
                "studentCount": student_count,
                "markSetCount": mark_set_count
            }))
        })
        .and_then(|it| it.collect::<Result<Vec<_>, _>>());

    match rows {
        Ok(classes) => ok(&req.id, json!({ "classes": classes })),
        Err(e) => err(&req.id, "db_query_failed", e.to_string(), None),
    }
}

fn handle_classes_create(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let name = match req.params.get("name").and_then(|v| v.as_str()) {
        Some(v) => v.trim().to_string(),
        None => return err(&req.id, "bad_params", "missing name", None),
    };
    if name.is_empty() {
        return err(&req.id, "bad_params", "name must not be empty", None);
    }

    let class_id = Uuid::new_v4().to_string();
    if let Err(e) = conn.execute(
        "INSERT INTO classes(id, name) VALUES(?, ?)",
        (&class_id, &name),
    ) {
        return err(
            &req.id,
            "db_insert_failed",
            e.to_string(),
            Some(json!({ "table": "classes" })),
        );
    }

    ok(&req.id, json!({ "classId": class_id, "name": name }))
}

fn normalize_opt_string(v: Option<&serde_json::Value>) -> Result<Option<String>, &'static str> {
    let Some(v) = v else {
        return Ok(None);
    };
    if v.is_null() {
        return Ok(None);
    }
    let Some(s) = v.as_str() else {
        return Err("must be string or null");
    };
    let t = s.trim().to_string();
    if t.is_empty() {
        Ok(None)
    } else {
        Ok(Some(t))
    }
}

fn handle_classes_wizard_defaults(state: &mut AppState, req: &Request) -> serde_json::Value {
    if state.db.is_none() {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    }
    ok(
        &req.id,
        json!({
            "defaults": {
                "name": "",
                "classCode": "",
                "schoolYear": "",
                "schoolName": "",
                "teacherName": "",
                "calcMethodDefault": 0,
                "weightMethodDefault": 1,
                "schoolYearStartMonth": 9
            }
        }),
    )
}

fn handle_classes_create_from_wizard(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    let Some(payload) = req.params.as_object() else {
        return err(&req.id, "bad_params", "invalid params", None);
    };

    let name = match payload.get("name").and_then(|v| v.as_str()) {
        Some(v) => v.trim().to_string(),
        None => return err(&req.id, "bad_params", "missing name", None),
    };
    if name.is_empty() {
        return err(&req.id, "bad_params", "name must not be empty", None);
    }
    let class_code = match payload.get("classCode").and_then(|v| v.as_str()) {
        Some(v) => v.trim().to_string(),
        None => return err(&req.id, "bad_params", "missing classCode", None),
    };
    if class_code.is_empty() {
        return err(&req.id, "bad_params", "classCode must not be empty", None);
    }
    if class_code.len() > 15 {
        return err(
            &req.id,
            "bad_params",
            "classCode must be 15 chars or fewer",
            None,
        );
    }

    let school_year = match normalize_opt_string(payload.get("schoolYear")) {
        Ok(v) => v,
        Err(_) => {
            return err(
                &req.id,
                "bad_params",
                "schoolYear must be string or null",
                None,
            )
        }
    };
    let school_name = match normalize_opt_string(payload.get("schoolName")) {
        Ok(v) => v,
        Err(_) => {
            return err(
                &req.id,
                "bad_params",
                "schoolName must be string or null",
                None,
            )
        }
    };
    let teacher_name = match normalize_opt_string(payload.get("teacherName")) {
        Ok(v) => v,
        Err(_) => {
            return err(
                &req.id,
                "bad_params",
                "teacherName must be string or null",
                None,
            )
        }
    };

    let calc_method_default = payload
        .get("calcMethodDefault")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    if !(0..=4).contains(&calc_method_default) {
        return err(
            &req.id,
            "bad_params",
            "calcMethodDefault must be 0..4",
            None,
        );
    }
    let weight_method_default = payload
        .get("weightMethodDefault")
        .and_then(|v| v.as_i64())
        .unwrap_or(1);
    if !(0..=2).contains(&weight_method_default) {
        return err(
            &req.id,
            "bad_params",
            "weightMethodDefault must be 0, 1, or 2",
            None,
        );
    }
    let school_year_start_month = payload
        .get("schoolYearStartMonth")
        .and_then(|v| v.as_i64())
        .unwrap_or(9);
    if !(1..=12).contains(&school_year_start_month) {
        return err(
            &req.id,
            "bad_params",
            "schoolYearStartMonth must be 1..12",
            None,
        );
    }

    let class_id = Uuid::new_v4().to_string();
    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return err(&req.id, "db_tx_failed", e.to_string(), None),
    };

    if let Err(e) = tx.execute(
        "INSERT INTO classes(id, name) VALUES(?, ?)",
        (&class_id, &name),
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_insert_failed",
            e.to_string(),
            Some(json!({ "table": "classes" })),
        );
    }

    if let Err(e) = tx.execute(
        "INSERT INTO class_meta(
            class_id,
            class_code,
            school_year,
            school_name,
            teacher_name,
            calc_method_default,
            weight_method_default,
            school_year_start_month,
            created_from_wizard
        ) VALUES(?, ?, ?, ?, ?, ?, ?, ?, 1)",
        (
            &class_id,
            &class_code,
            &school_year,
            &school_name,
            &teacher_name,
            calc_method_default,
            weight_method_default,
            school_year_start_month,
        ),
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_insert_failed",
            e.to_string(),
            Some(json!({ "table": "class_meta" })),
        );
    }

    if let Err(e) = tx.commit() {
        return err(&req.id, "db_commit_failed", e.to_string(), None);
    }

    ok(
        &req.id,
        json!({
            "classId": class_id,
            "name": name,
            "classCode": class_code
        }),
    )
}

fn handle_classes_meta_get(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };

    let row: Option<(String, String)> = match conn
        .query_row(
            "SELECT id, name FROM classes WHERE id = ?",
            [&class_id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let Some((id, name)) = row else {
        return err(&req.id, "not_found", "class not found", None);
    };

    let warnings_key = format!("classes.lastImportWarnings.{class_id}");
    let warnings_count = match db::settings_get_json(conn, &warnings_key) {
        Ok(Some(v)) => v
            .as_array()
            .map(|arr| arr.len() as i64)
            .or_else(|| {
                v.get("warnings")
                    .and_then(|w| w.as_array())
                    .map(|arr| arr.len() as i64)
            })
            .unwrap_or(0),
        Ok(None) => 0,
        Err(_) => 0,
    };

    let meta = conn
        .query_row(
            "SELECT
                class_code,
                school_year,
                school_name,
                teacher_name,
                calc_method_default,
                weight_method_default,
                school_year_start_month,
                created_from_wizard,
                legacy_folder_path,
                legacy_cl_file,
                legacy_year_token,
                last_imported_at
             FROM class_meta
             WHERE class_id = ?",
            [&class_id],
            |r| {
                Ok(json!({
                    "classCode": r.get::<_, Option<String>>(0)?,
                    "schoolYear": r.get::<_, Option<String>>(1)?,
                    "schoolName": r.get::<_, Option<String>>(2)?,
                    "teacherName": r.get::<_, Option<String>>(3)?,
                    "calcMethodDefault": r.get::<_, Option<i64>>(4)?,
                    "weightMethodDefault": r.get::<_, Option<i64>>(5)?,
                    "schoolYearStartMonth": r.get::<_, Option<i64>>(6)?,
                    "createdFromWizard": r.get::<_, i64>(7)? != 0,
                    "legacyFolderPath": r.get::<_, Option<String>>(8)?,
                    "legacyClFile": r.get::<_, Option<String>>(9)?,
                    "legacyYearToken": r.get::<_, Option<String>>(10)?,
                    "lastImportedAt": r.get::<_, Option<String>>(11)?,
                    "lastImportWarningsCount": warnings_count
                }))
            },
        )
        .optional();

    let meta = match meta {
        Ok(Some(v)) => v,
        Ok(None) => json!({
            "classCode": null,
            "schoolYear": null,
            "schoolName": null,
            "teacherName": null,
            "calcMethodDefault": null,
            "weightMethodDefault": null,
            "schoolYearStartMonth": null,
            "createdFromWizard": false,
            "legacyFolderPath": null,
            "legacyClFile": null,
            "legacyYearToken": null,
            "lastImportedAt": null,
            "lastImportWarningsCount": warnings_count
        }),
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    ok(
        &req.id,
        json!({
            "class": { "id": id, "name": name },
            "meta": meta
        }),
    )
}

fn handle_classes_meta_update(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };
    let Some(patch) = req.params.get("patch").and_then(|v| v.as_object()) else {
        return err(&req.id, "bad_params", "missing/invalid patch", None);
    };

    let exists: Option<i64> = match conn
        .query_row("SELECT 1 FROM classes WHERE id = ?", [&class_id], |r| {
            r.get(0)
        })
        .optional()
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    if exists.is_none() {
        return err(&req.id, "not_found", "class not found", None);
    }

    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return err(&req.id, "db_tx_failed", e.to_string(), None),
    };

    if let Some(v) = patch.get("name") {
        let Some(name) = v.as_str() else {
            let _ = tx.rollback();
            return err(&req.id, "bad_params", "patch.name must be string", None);
        };
        let name = name.trim().to_string();
        if name.is_empty() {
            let _ = tx.rollback();
            return err(&req.id, "bad_params", "patch.name must not be empty", None);
        }
        if let Err(e) = tx.execute(
            "UPDATE classes SET name = ? WHERE id = ?",
            (&name, &class_id),
        ) {
            let _ = tx.rollback();
            return err(
                &req.id,
                "db_update_failed",
                e.to_string(),
                Some(json!({ "table": "classes" })),
            );
        }
    }

    if let Err(e) = tx.execute(
        "INSERT INTO class_meta(class_id, created_from_wizard) VALUES(?, 0)
         ON CONFLICT(class_id) DO NOTHING",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_insert_failed",
            e.to_string(),
            Some(json!({ "table": "class_meta" })),
        );
    }

    let mut set_parts: Vec<String> = Vec::new();
    let mut bind_values: Vec<Value> = Vec::new();

    if patch.contains_key("classCode") {
        match normalize_opt_string(patch.get("classCode")) {
            Ok(Some(v)) => {
                if v.len() > 15 {
                    let _ = tx.rollback();
                    return err(
                        &req.id,
                        "bad_params",
                        "patch.classCode must be 15 chars or fewer",
                        None,
                    );
                }
                set_parts.push("class_code = ?".into());
                bind_values.push(Value::Text(v));
            }
            Ok(None) => {
                set_parts.push("class_code = ?".into());
                bind_values.push(Value::Null);
            }
            Err(_) => {
                let _ = tx.rollback();
                return err(
                    &req.id,
                    "bad_params",
                    "patch.classCode must be string or null",
                    None,
                );
            }
        }
    }
    if patch.contains_key("schoolYear") {
        match normalize_opt_string(patch.get("schoolYear")) {
            Ok(Some(v)) => {
                set_parts.push("school_year = ?".into());
                bind_values.push(Value::Text(v));
            }
            Ok(None) => {
                set_parts.push("school_year = ?".into());
                bind_values.push(Value::Null);
            }
            Err(_) => {
                let _ = tx.rollback();
                return err(
                    &req.id,
                    "bad_params",
                    "patch.schoolYear must be string or null",
                    None,
                );
            }
        }
    }
    if patch.contains_key("schoolName") {
        match normalize_opt_string(patch.get("schoolName")) {
            Ok(Some(v)) => {
                set_parts.push("school_name = ?".into());
                bind_values.push(Value::Text(v));
            }
            Ok(None) => {
                set_parts.push("school_name = ?".into());
                bind_values.push(Value::Null);
            }
            Err(_) => {
                let _ = tx.rollback();
                return err(
                    &req.id,
                    "bad_params",
                    "patch.schoolName must be string or null",
                    None,
                );
            }
        }
    }
    if patch.contains_key("teacherName") {
        match normalize_opt_string(patch.get("teacherName")) {
            Ok(Some(v)) => {
                set_parts.push("teacher_name = ?".into());
                bind_values.push(Value::Text(v));
            }
            Ok(None) => {
                set_parts.push("teacher_name = ?".into());
                bind_values.push(Value::Null);
            }
            Err(_) => {
                let _ = tx.rollback();
                return err(
                    &req.id,
                    "bad_params",
                    "patch.teacherName must be string or null",
                    None,
                );
            }
        }
    }
    if let Some(v) = patch.get("calcMethodDefault") {
        if v.is_null() {
            set_parts.push("calc_method_default = ?".into());
            bind_values.push(Value::Null);
        } else if let Some(n) = v.as_i64() {
            if !(0..=4).contains(&n) {
                let _ = tx.rollback();
                return err(
                    &req.id,
                    "bad_params",
                    "patch.calcMethodDefault must be 0..4",
                    None,
                );
            }
            set_parts.push("calc_method_default = ?".into());
            bind_values.push(Value::Integer(n));
        } else {
            let _ = tx.rollback();
            return err(
                &req.id,
                "bad_params",
                "patch.calcMethodDefault must be integer or null",
                None,
            );
        }
    }
    if let Some(v) = patch.get("weightMethodDefault") {
        if v.is_null() {
            set_parts.push("weight_method_default = ?".into());
            bind_values.push(Value::Null);
        } else if let Some(n) = v.as_i64() {
            if !(0..=2).contains(&n) {
                let _ = tx.rollback();
                return err(
                    &req.id,
                    "bad_params",
                    "patch.weightMethodDefault must be 0, 1, or 2",
                    None,
                );
            }
            set_parts.push("weight_method_default = ?".into());
            bind_values.push(Value::Integer(n));
        } else {
            let _ = tx.rollback();
            return err(
                &req.id,
                "bad_params",
                "patch.weightMethodDefault must be integer or null",
                None,
            );
        }
    }
    if let Some(v) = patch.get("schoolYearStartMonth") {
        if v.is_null() {
            set_parts.push("school_year_start_month = ?".into());
            bind_values.push(Value::Null);
        } else if let Some(n) = v.as_i64() {
            if !(1..=12).contains(&n) {
                let _ = tx.rollback();
                return err(
                    &req.id,
                    "bad_params",
                    "patch.schoolYearStartMonth must be 1..12",
                    None,
                );
            }
            set_parts.push("school_year_start_month = ?".into());
            bind_values.push(Value::Integer(n));
        } else {
            let _ = tx.rollback();
            return err(
                &req.id,
                "bad_params",
                "patch.schoolYearStartMonth must be integer or null",
                None,
            );
        }
    }

    if !set_parts.is_empty() {
        let sql = format!(
            "UPDATE class_meta SET {} WHERE class_id = ?",
            set_parts.join(", ")
        );
        bind_values.push(Value::Text(class_id.clone()));
        if let Err(e) = tx.execute(&sql, rusqlite::params_from_iter(bind_values)) {
            let _ = tx.rollback();
            return err(
                &req.id,
                "db_update_failed",
                e.to_string(),
                Some(json!({ "table": "class_meta" })),
            );
        }
    }

    if let Err(e) = tx.commit() {
        return err(&req.id, "db_commit_failed", e.to_string(), None);
    }

    ok(&req.id, json!({ "ok": true }))
}

fn handle_classes_import_link_get(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };

    let exists: Option<i64> = match conn
        .query_row("SELECT 1 FROM classes WHERE id = ?", [&class_id], |r| r.get(0))
        .optional()
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    if exists.is_none() {
        return err(&req.id, "not_found", "class not found", None);
    }

    let row = match conn
        .query_row(
            "SELECT legacy_folder_path, legacy_cl_file, legacy_year_token, last_imported_at
             FROM class_meta
             WHERE class_id = ?",
            [&class_id],
            |r| {
                Ok((
                    r.get::<_, Option<String>>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    let (legacy_folder_path, legacy_cl_file, legacy_year_token, last_imported_at) =
        row.unwrap_or((None, None, None, None));

    ok(
        &req.id,
        json!({
            "classId": class_id,
            "legacyClassFolderPath": legacy_folder_path,
            "legacyClFile": legacy_cl_file,
            "legacyYearToken": legacy_year_token,
            "lastImportedAt": last_imported_at
        }),
    )
}

fn handle_classes_import_link_set(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };
    let legacy_class_folder_path = match req
        .params
        .get("legacyClassFolderPath")
        .and_then(|v| v.as_str())
    {
        Some(v) => v.trim().to_string(),
        None => {
            return err(
                &req.id,
                "bad_params",
                "missing legacyClassFolderPath",
                None,
            )
        }
    };
    if legacy_class_folder_path.is_empty() {
        return err(
            &req.id,
            "bad_params",
            "legacyClassFolderPath must not be empty",
            None,
        );
    }

    let exists: Option<i64> = match conn
        .query_row("SELECT 1 FROM classes WHERE id = ?", [&class_id], |r| r.get(0))
        .optional()
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    if exists.is_none() {
        return err(&req.id, "not_found", "class not found", None);
    }

    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return err(&req.id, "db_tx_failed", e.to_string(), None),
    };

    if let Err(e) = tx.execute(
        "INSERT INTO class_meta(class_id, created_from_wizard)
         VALUES(?, 0)
         ON CONFLICT(class_id) DO NOTHING",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_insert_failed",
            e.to_string(),
            Some(json!({ "table": "class_meta" })),
        );
    }

    if let Err(e) = tx.execute(
        "UPDATE class_meta
         SET legacy_folder_path = ?
         WHERE class_id = ?",
        (&legacy_class_folder_path, &class_id),
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_update_failed",
            e.to_string(),
            Some(json!({ "table": "class_meta" })),
        );
    }

    if let Err(e) = tx.commit() {
        return err(&req.id, "db_commit_failed", e.to_string(), None);
    }

    ok(
        &req.id,
        json!({
            "ok": true,
            "classId": class_id,
            "legacyClassFolderPath": legacy_class_folder_path
        }),
    )
}

fn handle_classes_delete(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let class_id = match req.params.get("classId").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return err(&req.id, "bad_params", "missing classId", None),
    };

    let exists: Option<i64> = match conn
        .query_row("SELECT 1 FROM classes WHERE id = ?", [&class_id], |r| {
            r.get(0)
        })
        .optional()
    {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    if exists.is_none() {
        return err(&req.id, "not_found", "class not found", None);
    }

    let tx = match conn.unchecked_transaction() {
        Ok(t) => t,
        Err(e) => return err(&req.id, "db_tx_failed", e.to_string(), None),
    };

    // Explicitly delete in dependency order (no ON DELETE CASCADE).
    // NOTE: additional tables will be added over time; keep this list updated.
    if let Err(e) = tx.execute(
        "DELETE FROM scores
         WHERE assessment_id IN (
           SELECT a.id
           FROM assessments a
           JOIN mark_sets ms ON ms.id = a.mark_set_id
           WHERE ms.class_id = ?
         )",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "scores" })),
        );
    }

    if let Err(e) = tx.execute(
        "DELETE FROM comment_set_remarks
         WHERE comment_set_index_id IN (
           SELECT csi.id
           FROM comment_set_indexes csi
           WHERE csi.class_id = ?
         )",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "comment_set_remarks" })),
        );
    }

    if let Err(e) = tx.execute(
        "DELETE FROM comment_set_indexes WHERE class_id = ?",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "comment_set_indexes" })),
        );
    }

    if let Err(e) = tx.execute(
        "DELETE FROM attendance_student_months WHERE class_id = ?",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "attendance_student_months" })),
        );
    }

    if let Err(e) = tx.execute(
        "DELETE FROM attendance_months WHERE class_id = ?",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "attendance_months" })),
        );
    }

    if let Err(e) = tx.execute(
        "DELETE FROM attendance_settings WHERE class_id = ?",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "attendance_settings" })),
        );
    }

    if let Err(e) = tx.execute(
        "DELETE FROM seating_assignments WHERE class_id = ?",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "seating_assignments" })),
        );
    }

    if let Err(e) = tx.execute("DELETE FROM seating_plans WHERE class_id = ?", [&class_id]) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "seating_plans" })),
        );
    }

    if let Err(e) = tx.execute("DELETE FROM loaned_items WHERE class_id = ?", [&class_id]) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "loaned_items" })),
        );
    }

    if let Err(e) = tx.execute(
        "DELETE FROM student_device_map WHERE class_id = ?",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "student_device_map" })),
        );
    }

    if let Err(e) = tx.execute(
        "DELETE FROM assessments
         WHERE mark_set_id IN (SELECT id FROM mark_sets WHERE class_id = ?)",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "assessments" })),
        );
    }

    if let Err(e) = tx.execute(
        "DELETE FROM categories
         WHERE mark_set_id IN (SELECT id FROM mark_sets WHERE class_id = ?)",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "categories" })),
        );
    }

    if let Err(e) = tx.execute("DELETE FROM mark_sets WHERE class_id = ?", [&class_id]) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "mark_sets" })),
        );
    }

    if let Err(e) = tx.execute("DELETE FROM student_notes WHERE class_id = ?", [&class_id]) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "student_notes" })),
        );
    }

    if let Err(e) = tx.execute(
        "DELETE FROM learning_skills_cells WHERE class_id = ?",
        [&class_id],
    ) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "learning_skills_cells" })),
        );
    }

    if let Err(e) = tx.execute("DELETE FROM students WHERE class_id = ?", [&class_id]) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "students" })),
        );
    }

    if let Err(e) = tx.execute("DELETE FROM class_meta WHERE class_id = ?", [&class_id]) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "class_meta" })),
        );
    }

    if let Err(e) = tx.execute("DELETE FROM classes WHERE id = ?", [&class_id]) {
        let _ = tx.rollback();
        return err(
            &req.id,
            "db_delete_failed",
            e.to_string(),
            Some(json!({ "table": "classes" })),
        );
    }

    if let Err(e) = tx.commit() {
        return err(&req.id, "db_commit_failed", e.to_string(), None);
    }

    ok(&req.id, json!({ "ok": true }))
}

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "classes.list" => Some(handle_classes_list(state, req)),
        "classes.create" => Some(handle_classes_create(state, req)),
        "classes.wizardDefaults" => Some(handle_classes_wizard_defaults(state, req)),
        "classes.createFromWizard" => Some(handle_classes_create_from_wizard(state, req)),
        "classes.meta.get" => Some(handle_classes_meta_get(state, req)),
        "classes.meta.update" => Some(handle_classes_meta_update(state, req)),
        "classes.importLink.get" => Some(handle_classes_import_link_get(state, req)),
        "classes.importLink.set" => Some(handle_classes_import_link_set(state, req)),
        "classes.delete" => Some(handle_classes_delete(state, req)),
        _ => None,
    }
}
