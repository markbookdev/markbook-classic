use crate::ipc::router::handle_request_legacy;
use crate::ipc::types::{AppState, Request};

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "students.list"
        | "students.create"
        | "students.update"
        | "students.reorder"
        | "students.delete"
        | "notes.get"
        | "notes.update" => Some(handle_request_legacy(state, req.clone())),
        _ => None,
    }
}
