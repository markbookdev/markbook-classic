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
        _ => None,
    }
}
