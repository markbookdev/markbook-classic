use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_path(rel: &str) -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    base.join("../../").join(rel)
}

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
fn bulk_update_returns_rejection_diagnostics() {
    let workspace = temp_dir("markbook-grid-bulk-validation");
    let fixture_folder = fixture_path("fixtures/legacy/Sample25/MB8D25");

    let (_child, mut stdin, mut reader) = spawn_sidecar();

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );
    let import = request_ok(
        &mut stdin,
        &mut reader,
        "2",
        "class.importLegacy",
        json!({ "legacyClassFolderPath": fixture_folder.to_string_lossy() }),
    );
    let class_id = import
        .get("classId")
        .and_then(|v| v.as_str())
        .expect("classId")
        .to_string();

    let marksets = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "marksets.list",
        json!({ "classId": class_id.clone() }),
    );
    let mark_set_id = marksets
        .get("markSets")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("id"))
        .and_then(|v| v.as_str())
        .expect("markSetId")
        .to_string();

    let raw = request(
        &mut stdin,
        &mut reader,
        "bulk",
        "grid.bulkUpdate",
        json!({
            "classId": class_id,
            "markSetId": mark_set_id,
            "edits": [
                { "row": 0, "col": 0, "state": "scored", "value": 8.5 },
                { "row": 0, "col": 1, "state": "scored", "value": -2.0 },
                { "row": -1, "col": 0, "state": "no_mark", "value": null },
                { "row": 9999, "col": 0, "state": "no_mark", "value": null }
            ]
        }),
    );

    assert_eq!(raw.get("ok").and_then(|v| v.as_bool()), Some(true));
    let result = raw.get("result").cloned().unwrap_or_else(|| json!({}));

    assert_eq!(result.get("updated").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(result.get("rejected").and_then(|v| v.as_u64()), Some(3));

    let errors = result
        .get("errors")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert_eq!(errors.len(), 3);

    assert!(errors.iter().any(|e| {
        e.get("code").and_then(|v| v.as_str()) == Some("bad_params")
            && e.get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .contains("negative")
    }));
    assert!(errors.iter().any(|e| {
        e.get("code").and_then(|v| v.as_str()) == Some("bad_params")
            && e.get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .contains("missing/invalid row")
    }));
    assert!(errors.iter().any(|e| {
        e.get("code").and_then(|v| v.as_str()) == Some("not_found")
            && e.get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .contains("student")
    }));
}
