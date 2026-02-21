use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(prefix: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "{}-{}",
        prefix,
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&p).expect("create temp dir");
    p
}

fn spawn_sidecar() -> (Child, ChildStdin, BufReader<ChildStdout>) {
    let exe = env!("CARGO_BIN_EXE_markbookd");
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn markbookd");
    let stdin = child.stdin.take().expect("child stdin");
    let stdout = child.stdout.take().expect("child stdout");
    (child, stdin, BufReader::new(stdout))
}

fn request(
    stdin: &mut ChildStdin,
    reader: &mut BufReader<ChildStdout>,
    id: &str,
    method: &str,
    params: serde_json::Value,
) -> serde_json::Value {
    let payload = json!({
        "id": id,
        "method": method,
        "params": params,
    });
    writeln!(stdin, "{}", payload).expect("write request");
    stdin.flush().expect("flush request");

    let mut line = String::new();
    reader.read_line(&mut line).expect("read response line");
    assert!(!line.trim().is_empty(), "empty response for {}", method);
    let value: serde_json::Value = serde_json::from_str(line.trim()).expect("parse response json");
    assert_eq!(value.get("id").and_then(|v| v.as_str()), Some(id));
    value
}

fn request_ok(
    stdin: &mut ChildStdin,
    reader: &mut BufReader<ChildStdout>,
    id: &str,
    method: &str,
    params: serde_json::Value,
) -> serde_json::Value {
    let value = request(stdin, reader, id, method, params);
    assert!(
        value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false),
        "{} failed: {}",
        method,
        value
    );
    value.get("result").cloned().unwrap_or_else(|| json!({}))
}

#[test]
fn setup_get_update_roundtrip_and_validation() {
    let workspace = temp_dir("markbook-setup-admin");
    let (_child, mut stdin, mut reader) = spawn_sidecar();

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );

    let initial = request_ok(&mut stdin, &mut reader, "2", "setup.get", json!({}));
    assert_eq!(
        initial
            .pointer("/analysis/defaultStudentScope")
            .and_then(|v| v.as_str()),
        Some("valid")
    );
    assert_eq!(
        initial
            .pointer("/analysis/defaultSortBy")
            .and_then(|v| v.as_str()),
        Some("sortOrder")
    );
    assert_eq!(
        initial
            .pointer("/printer/fontScale")
            .and_then(|v| v.as_i64()),
        Some(100)
    );

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "setup.update",
        json!({
            "section": "analysis",
            "patch": {
                "defaultStudentScope": "active",
                "histogramBins": 12,
                "defaultSortBy": "displayName",
                "defaultTopBottomCount": 8
            }
        }),
    );
    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "setup.update",
        json!({
            "section": "printer",
            "patch": {
                "fontScale": 110,
                "repeatHeaders": false
            }
        }),
    );
    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "4b",
        "setup.update",
        json!({
            "section": "integrations",
            "patch": {
                "defaultSisProfile": "sis_marks_v1",
                "defaultMatchMode": "name_only",
                "defaultCollisionPolicy": "append_new",
                "autoPreviewBeforeApply": false,
                "adminTransferDefaultPolicy": "append"
            }
        }),
    );

    let updated = request_ok(&mut stdin, &mut reader, "5", "setup.get", json!({}));
    assert_eq!(
        updated
            .pointer("/analysis/defaultStudentScope")
            .and_then(|v| v.as_str()),
        Some("active")
    );
    assert_eq!(
        updated
            .pointer("/analysis/histogramBins")
            .and_then(|v| v.as_i64()),
        Some(12)
    );
    assert_eq!(
        updated
            .pointer("/analysis/defaultSortBy")
            .and_then(|v| v.as_str()),
        Some("displayName")
    );
    assert_eq!(
        updated
            .pointer("/analysis/defaultTopBottomCount")
            .and_then(|v| v.as_i64()),
        Some(8)
    );
    assert_eq!(
        updated.pointer("/printer/fontScale").and_then(|v| v.as_i64()),
        Some(110)
    );
    assert_eq!(
        updated
            .pointer("/printer/repeatHeaders")
            .and_then(|v| v.as_bool()),
        Some(false)
    );
    assert_eq!(
        updated
            .pointer("/integrations/defaultSisProfile")
            .and_then(|v| v.as_str()),
        Some("sis_marks_v1")
    );
    assert_eq!(
        updated
            .pointer("/integrations/defaultMatchMode")
            .and_then(|v| v.as_str()),
        Some("name_only")
    );
    assert_eq!(
        updated
            .pointer("/integrations/defaultCollisionPolicy")
            .and_then(|v| v.as_str()),
        Some("append_new")
    );
    assert_eq!(
        updated
            .pointer("/integrations/autoPreviewBeforeApply")
            .and_then(|v| v.as_bool()),
        Some(false)
    );
    assert_eq!(
        updated
            .pointer("/integrations/adminTransferDefaultPolicy")
            .and_then(|v| v.as_str()),
        Some("append")
    );

    let invalid = request(
        &mut stdin,
        &mut reader,
        "6",
        "setup.update",
        json!({
            "section": "analysis",
            "patch": {
                "histogramBins": 99
            }
        }),
    );
    assert_eq!(invalid.get("ok").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(
        invalid
            .pointer("/error/code")
            .and_then(|v| v.as_str()),
        Some("bad_params")
    );
}
