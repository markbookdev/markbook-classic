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
fn class_wizard_create_and_meta_roundtrip() {
    let workspace = temp_dir("markbook-class-wizard");
    let (_child, mut stdin, mut reader) = spawn_sidecar();

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );

    let defaults = request_ok(
        &mut stdin,
        &mut reader,
        "2",
        "classes.wizardDefaults",
        json!({}),
    );
    assert!(defaults.get("defaults").is_some(), "missing defaults payload");

    let created = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "classes.createFromWizard",
        json!({
            "name": "8D (2026)",
            "classCode": "8D26",
            "schoolYear": "2025/2026",
            "schoolName": "Asylum School",
            "teacherName": "Rob Hedges",
            "calcMethodDefault": 2,
            "weightMethodDefault": 1,
            "schoolYearStartMonth": 9
        }),
    );
    let class_id = created
        .get("classId")
        .and_then(|v| v.as_str())
        .expect("classId")
        .to_string();
    assert_eq!(created.get("classCode").and_then(|v| v.as_str()), Some("8D26"));

    let meta = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "classes.meta.get",
        json!({ "classId": class_id.clone() }),
    );
    assert_eq!(
        meta.pointer("/meta/classCode").and_then(|v| v.as_str()),
        Some("8D26")
    );
    assert_eq!(
        meta.pointer("/meta/createdFromWizard")
            .and_then(|v| v.as_bool()),
        Some(true)
    );

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "classes.meta.update",
        json!({
            "classId": class_id.clone(),
            "patch": {
                "teacherName": "Updated Teacher",
                "schoolYearStartMonth": 8
            }
        }),
    );

    let meta2 = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "classes.meta.get",
        json!({ "classId": class_id }),
    );
    assert_eq!(
        meta2
            .pointer("/meta/teacherName")
            .and_then(|v| v.as_str()),
        Some("Updated Teacher")
    );
    assert_eq!(
        meta2
            .pointer("/meta/schoolYearStartMonth")
            .and_then(|v| v.as_i64()),
        Some(8)
    );
}
