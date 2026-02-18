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
fn membership_bulk_set_updates_multiple_students_and_reports_failures() {
    let workspace = temp_dir("markbook-membership-bulk-set");
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

    let members = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "students.membership.get",
        json!({ "classId": class_id.clone() }),
    );
    let mark_set_id = members
        .get("markSets")
        .and_then(|v| v.as_array())
        .and_then(|v| v.first())
        .and_then(|v| v.get("id"))
        .and_then(|v| v.as_str())
        .expect("markSetId")
        .to_string();
    let student_a = members
        .get("students")
        .and_then(|v| v.as_array())
        .and_then(|v| v.first())
        .and_then(|v| v.get("id"))
        .and_then(|v| v.as_str())
        .expect("student 0")
        .to_string();
    let student_b = members
        .get("students")
        .and_then(|v| v.as_array())
        .and_then(|v| v.get(1))
        .and_then(|v| v.get("id"))
        .and_then(|v| v.as_str())
        .expect("student 1")
        .to_string();

    let raw = request(
        &mut stdin,
        &mut reader,
        "4",
        "students.membership.bulkSet",
        json!({
            "classId": class_id,
            "markSetId": mark_set_id,
            "updates": [
                { "studentId": student_a, "enabled": false },
                { "studentId": student_b, "enabled": false },
                { "studentId": "missing-student", "enabled": false }
            ]
        }),
    );
    assert_eq!(raw.get("ok").and_then(|v| v.as_bool()), Some(true));
    let result = raw.get("result").cloned().unwrap_or_else(|| json!({}));
    assert_eq!(result.get("updated").and_then(|v| v.as_u64()), Some(2));
    assert_eq!(
        result
            .get("failed")
            .and_then(|v| v.as_array())
            .map(|v| v.len()),
        Some(1)
    );

    let after = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "students.membership.get",
        json!({ "classId": class_id }),
    );
    let row = after
        .get("students")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|s| s.get("id").and_then(|v| v.as_str()) == Some(student_a.as_str()))
        })
        .cloned()
        .expect("updated student row");
    let mask = row.get("mask").and_then(|v| v.as_str()).unwrap_or("");
    assert_eq!(mask.chars().next(), Some('0'));
}
