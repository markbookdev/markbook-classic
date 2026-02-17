use crate::db;
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
            state.db = Some(conn);
            ok(&req.id, json!({ "workspacePath": path.to_string_lossy() }))
        }
        Err(e) => err(&req.id, "db_open_failed", format!("{e:?}"), None),
    }
}

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "health" => Some(handle_health(state, req)),
        "workspace.select" => Some(handle_workspace_select(state, req)),
        _ => None,
    }
}
