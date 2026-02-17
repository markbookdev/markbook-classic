use super::handlers;
use super::types::{AppState, Request};
use crate::ipc::error::err;

pub fn handle_request(state: &mut AppState, req: Request) -> serde_json::Value {
    if let Some(resp) = handlers::core::try_handle(state, &req) {
        return resp;
    }
    if let Some(resp) = handlers::classes::try_handle(state, &req) {
        return resp;
    }
    if let Some(resp) = handlers::import_legacy::try_handle(state, &req) {
        return resp;
    }
    if let Some(resp) = handlers::grid::try_handle(state, &req) {
        return resp;
    }
    if let Some(resp) = handlers::students::try_handle(state, &req) {
        return resp;
    }
    if let Some(resp) = handlers::markset_setup::try_handle(state, &req) {
        return resp;
    }
    if let Some(resp) = handlers::attendance::try_handle(state, &req) {
        return resp;
    }
    if let Some(resp) = handlers::seating::try_handle(state, &req) {
        return resp;
    }
    if let Some(resp) = handlers::comments::try_handle(state, &req) {
        return resp;
    }
    if let Some(resp) = handlers::reports::try_handle(state, &req) {
        return resp;
    }
    if let Some(resp) = handlers::backup_exchange::try_handle(state, &req) {
        return resp;
    }
    if let Some(resp) = handlers::assets::try_handle(state, &req) {
        return resp;
    }

    err(
        &req.id,
        "not_implemented",
        format!("unknown method: {}", req.method),
        None,
    )
}
