use crate::ipc::router::handle_request_legacy;
use crate::ipc::types::{AppState, Request};

pub fn try_handle(state: &mut AppState, req: &Request) -> Option<serde_json::Value> {
    match req.method.as_str() {
        "comments.sets.list"
        | "comments.sets.open"
        | "comments.sets.upsert"
        | "comments.sets.delete"
        | "comments.banks.list"
        | "comments.banks.open"
        | "comments.banks.create"
        | "comments.banks.updateMeta"
        | "comments.banks.entryUpsert"
        | "comments.banks.entryDelete"
        | "comments.banks.importBnk"
        | "comments.banks.exportBnk" => Some(handle_request_legacy(state, req.clone())),
        _ => None,
    }
}
