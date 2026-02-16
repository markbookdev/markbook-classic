use crate::ipc::router::handle_request_legacy;
use crate::ipc::types::{AppState, Request};

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "backup.exportWorkspaceBundle"
        | "backup.importWorkspaceBundle"
        | "exchange.exportClassCsv"
        | "exchange.importClassCsv" => Some(handle_request_legacy(state, req.clone())),
        _ => None,
    }
}
