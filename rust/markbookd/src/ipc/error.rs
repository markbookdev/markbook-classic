use serde_json::json;

#[allow(dead_code)]
pub fn ok(id: &str, result: serde_json::Value) -> serde_json::Value {
    json!({
        "id": id,
        "ok": true,
        "result": result
    })
}

#[allow(dead_code)]
pub fn err(
    id: &str,
    code: &str,
    message: impl Into<String>,
    details: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut error = json!({
        "code": code,
        "message": message.into(),
    });
    if let Some(d) = details {
        error["details"] = d;
    }
    json!({
        "id": id,
        "ok": false,
        "error": error,
    })
}
