use crate::db;
use crate::legacy;
use crate::ipc::error::{err, ok};
use crate::ipc::types::{AppState, Request};
use serde_json::json;
use std::path::PathBuf;

fn handle_health(state: &mut AppState, req: &Request) -> serde_json::Value {
    ok(
        &req.id,
        json!({
            "version": env!("CARGO_PKG_VERSION"),
            "workspacePath": state.workspace.as_ref().map(|p| p.to_string_lossy().to_string())
        }),
    )
}

fn handle_workspace_select(state: &mut AppState, req: &Request) -> serde_json::Value {
    let p = req
        .params
        .get("path")
        .and_then(|v| v.as_str())
        .map(PathBuf::from);
    let Some(path) = p else {
        return err(&req.id, "bad_params", "missing params.path", None);
    };

    match db::open_db(&path) {
        Ok(conn) => {
            state.workspace = Some(path.clone());
            // Best-effort: import user calc settings (mode levels + roff) from *_USR.CFG.
            // This must not prevent the workspace from opening.
            //
            // Note: this only writes base keys. User overrides (user_cfg.override.*) are preserved.
            if let Ok(Some(cfg_path)) = find_usr_cfg(&path) {
                if let Ok(cfg) = legacy::parse_legacy_user_cfg(&cfg_path) {
                    let _ = db::settings_set_json(
                        &conn,
                        "user_cfg.mode_levels",
                        &json!({
                            "activeLevels": cfg.mode_active_levels,
                            "vals": cfg.mode_vals,
                            "symbols": cfg.mode_symbols,
                        }),
                    );
                    let _ = db::settings_set_json(
                        &conn,
                        "user_cfg.roff",
                        &json!({ "roff": cfg.roff_default }),
                    );
                }
            }

            state.db = Some(conn);
            ok(&req.id, json!({ "workspacePath": path.to_string_lossy() }))
        }
        Err(e) => err(&req.id, "db_open_failed", format!("{e:?}"), None),
    }
}

fn default_calc_config() -> serde_json::Value {
    // Mirrors calc::default_mode_config() but is defined here to avoid exposing internal calc types.
    let mut vals = vec![0_i64; 22];
    vals[0] = 0;
    vals[1] = 50;
    vals[2] = 60;
    vals[3] = 70;
    vals[4] = 80;
    let symbols = vec!["".to_string(); 22];
    json!({
        "roff": true,
        "activeLevels": 4,
        "vals": vals,
        "symbols": symbols
    })
}

fn read_calc_config_from_settings(conn: &rusqlite::Connection, override_first: bool) -> anyhow::Result<serde_json::Value> {
    let base_levels = db::settings_get_json(conn, "user_cfg.mode_levels")?;
    let base_roff = db::settings_get_json(conn, "user_cfg.roff")?;
    let ov_levels = db::settings_get_json(conn, "user_cfg.override.mode_levels")?;
    let ov_roff = db::settings_get_json(conn, "user_cfg.override.roff")?;

    let mut cfg = default_calc_config();
    let mut cfg_obj = cfg.as_object_mut().expect("object");

    let pick_levels = if override_first && ov_levels.is_some() { ov_levels } else { base_levels.clone() };
    let pick_levels = if !override_first && base_levels.is_some() { base_levels } else { pick_levels };
    if let Some(v) = pick_levels {
        if let Some(obj) = v.as_object() {
            if let Some(n) = obj.get("activeLevels").and_then(|v| v.as_u64()) {
                cfg_obj.insert("activeLevels".to_string(), json!((n as i64).min(21)));
            }
            if let Some(arr) = obj.get("vals").and_then(|v| v.as_array()) {
                let mut vals: Vec<i64> = Vec::with_capacity(22);
                for x in arr.iter().take(22) {
                    vals.push(x.as_i64().unwrap_or(0));
                }
                while vals.len() < 22 {
                    vals.push(0);
                }
                cfg_obj.insert("vals".to_string(), json!(vals));
            }
            if let Some(arr) = obj.get("symbols").and_then(|v| v.as_array()) {
                let mut syms: Vec<String> = Vec::with_capacity(22);
                for x in arr.iter().take(22) {
                    syms.push(x.as_str().unwrap_or("").to_string());
                }
                while syms.len() < 22 {
                    syms.push("".to_string());
                }
                cfg_obj.insert("symbols".to_string(), json!(syms));
            }
        }
    }

    let pick_roff = if override_first && ov_roff.is_some() { ov_roff } else { base_roff.clone() };
    let pick_roff = if !override_first && base_roff.is_some() { base_roff } else { pick_roff };
    if let Some(v) = pick_roff {
        if let Some(obj) = v.as_object() {
            if let Some(b) = obj.get("roff").and_then(|v| v.as_bool()) {
                cfg_obj.insert("roff".to_string(), json!(b));
            }
        }
    }

    Ok(cfg)
}

fn handle_calc_config_get(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    let base_levels = match db::settings_get_json(conn, "user_cfg.mode_levels") {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let base_roff = match db::settings_get_json(conn, "user_cfg.roff") {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let ov_levels = match db::settings_get_json(conn, "user_cfg.override.mode_levels") {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let ov_roff = match db::settings_get_json(conn, "user_cfg.override.roff") {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };

    let base_present = base_levels.is_some() || base_roff.is_some();
    let override_present = ov_levels.is_some() || ov_roff.is_some();

    let cfg = match read_calc_config_from_settings(conn, true) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let obj = cfg.as_object().cloned().unwrap_or_default();
    let mut mode_vals = obj
        .get("vals")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_else(|| vec![]);
    while mode_vals.len() < 22 {
        mode_vals.push(json!(0));
    }
    if mode_vals.len() > 22 {
        mode_vals.truncate(22);
    }

    let mut mode_syms = obj
        .get("symbols")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_else(|| vec![]);
    while mode_syms.len() < 22 {
        mode_syms.push(json!(""));
    }
    if mode_syms.len() > 22 {
        mode_syms.truncate(22);
    }
    ok(
        &req.id,
        json!({
            "source": { "basePresent": base_present, "overridePresent": override_present },
            "roff": obj.get("roff").and_then(|v| v.as_bool()).unwrap_or(true),
            "modeActiveLevels": obj.get("activeLevels").and_then(|v| v.as_i64()).unwrap_or(4),
            "modeVals": mode_vals,
            "modeSymbols": mode_syms,
        }),
    )
}

fn handle_calc_config_update(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };

    // Use effective config as the merge base, but only write override keys.
    let mut cfg = match read_calc_config_from_settings(conn, true) {
        Ok(v) => v,
        Err(e) => return err(&req.id, "db_query_failed", e.to_string(), None),
    };
    let Some(cfg_obj) = cfg.as_object_mut() else {
        return err(&req.id, "db_query_failed", "invalid calc config base", None);
    };

    let touch_levels = req.params.get("modeActiveLevels").is_some()
        || req.params.get("modeVals").is_some()
        || req.params.get("modeSymbols").is_some();

    if let Some(v) = req.params.get("modeActiveLevels") {
        let Some(n) = v.as_i64() else {
            return err(&req.id, "bad_params", "modeActiveLevels must be integer", None);
        };
        if !(0..=21).contains(&n) {
            return err(&req.id, "bad_params", "modeActiveLevels must be 0..21", None);
        }
        cfg_obj.insert("activeLevels".to_string(), json!(n));
    }
    if let Some(v) = req.params.get("modeVals") {
        let Some(arr) = v.as_array() else {
            return err(&req.id, "bad_params", "modeVals must be number[22]", None);
        };
        if arr.len() != 22 {
            return err(&req.id, "bad_params", "modeVals must have length 22", None);
        }
        let mut vals: Vec<i64> = Vec::with_capacity(22);
        for x in arr.iter() {
            let Some(n) = x.as_i64() else {
                return err(&req.id, "bad_params", "modeVals must be integers", None);
            };
            vals.push(n);
        }
        cfg_obj.insert("vals".to_string(), json!(vals));
    }
    if let Some(v) = req.params.get("modeSymbols") {
        let Some(arr) = v.as_array() else {
            return err(&req.id, "bad_params", "modeSymbols must be string[22]", None);
        };
        if arr.len() != 22 {
            return err(&req.id, "bad_params", "modeSymbols must have length 22", None);
        }
        let mut syms: Vec<String> = Vec::with_capacity(22);
        for x in arr.iter() {
            let Some(s) = x.as_str() else {
                return err(&req.id, "bad_params", "modeSymbols must be strings", None);
            };
            syms.push(s.to_string());
        }
        cfg_obj.insert("symbols".to_string(), json!(syms));
    }
    if let Some(v) = req.params.get("roff") {
        let Some(b) = v.as_bool() else {
            return err(&req.id, "bad_params", "roff must be boolean", None);
        };
        if let Err(e) = db::settings_set_json(conn, "user_cfg.override.roff", &json!({ "roff": b })) {
            return err(&req.id, "db_update_failed", e.to_string(), None);
        }
    }

    if touch_levels {
        if let Err(e) = db::settings_set_json(
            conn,
            "user_cfg.override.mode_levels",
            &json!({
                "activeLevels": cfg_obj.get("activeLevels").cloned().unwrap_or(json!(4)),
                "vals": cfg_obj.get("vals").cloned().unwrap_or(json!([])),
                "symbols": cfg_obj.get("symbols").cloned().unwrap_or(json!([])),
            }),
        ) {
            return err(&req.id, "db_update_failed", e.to_string(), None);
        }
    }

    ok(&req.id, json!({ "ok": true }))
}

fn handle_calc_config_clear_override(state: &mut AppState, req: &Request) -> serde_json::Value {
    let Some(conn) = state.db.as_ref() else {
        return err(&req.id, "no_workspace", "select a workspace first", None);
    };
    if let Err(e) = db::settings_delete(conn, "user_cfg.override.mode_levels") {
        return err(&req.id, "db_update_failed", e.to_string(), None);
    }
    if let Err(e) = db::settings_delete(conn, "user_cfg.override.roff") {
        return err(&req.id, "db_update_failed", e.to_string(), None);
    }
    ok(&req.id, json!({ "ok": true }))
}

fn find_usr_cfg(workspace: &std::path::Path) -> anyhow::Result<Option<std::path::PathBuf>> {
    let mut best: Option<std::path::PathBuf> = None;
    for ent in std::fs::read_dir(workspace)? {
        let ent = ent?;
        let p = ent.path();
        if !p.is_file() {
            continue;
        }
        let Some(name) = p.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if name.to_ascii_uppercase().ends_with("_USR.CFG") {
            // Deterministic pick if multiple exist.
            if best.as_ref().map(|b| p < *b).unwrap_or(true) {
                best = Some(p);
            }
        }
    }
    Ok(best)
}

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "health" => Some(handle_health(state, req)),
        "workspace.select" => Some(handle_workspace_select(state, req)),
        "calc.config.get" => Some(handle_calc_config_get(state, req)),
        "calc.config.update" => Some(handle_calc_config_update(state, req)),
        "calc.config.clearOverride" => Some(handle_calc_config_clear_override(state, req)),
        _ => None,
    }
}
