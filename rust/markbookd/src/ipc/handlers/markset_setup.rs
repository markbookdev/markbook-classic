use crate::ipc::router::handle_request_legacy;
use crate::ipc::types::{AppState, Request};

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "categories.list"
        | "categories.create"
        | "categories.update"
        | "categories.delete"
        | "assessments.list"
        | "assessments.create"
        | "assessments.update"
        | "assessments.delete"
        | "assessments.reorder"
        | "markset.settings.get"
        | "markset.settings.update" => Some(handle_request_legacy(state, req.clone())),
        _ => None,
    }
}
