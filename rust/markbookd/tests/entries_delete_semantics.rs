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
fn entries_delete_soft_deletes_by_weight_only() {
    let workspace = temp_dir("markbook-entries-delete");
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

    let assessments_before = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "assessments.list",
        json!({ "classId": class_id.clone(), "markSetId": mark_set_id.clone(), "hideDeleted": false }),
    );
    let first_assessment = assessments_before
        .get("assessments")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .cloned()
        .expect("first assessment");
    let first_assessment_id = first_assessment
        .get("id")
        .and_then(|v| v.as_str())
        .expect("assessment id")
        .to_string();
    let first_idx = first_assessment
        .get("idx")
        .and_then(|v| v.as_i64())
        .expect("idx");

    let before_cell = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "grid.get",
        json!({
            "classId": class_id.clone(),
            "markSetId": mark_set_id.clone(),
            "rowStart": 0,
            "rowCount": 1,
            "colStart": first_idx,
            "colCount": 1
        }),
    );
    let before_value = before_cell
        .get("cells")
        .and_then(|v| v.as_array())
        .and_then(|rows| rows.first())
        .and_then(|row| row.as_array())
        .and_then(|row| row.first())
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "entries.delete",
        json!({
            "classId": class_id.clone(),
            "markSetId": mark_set_id.clone(),
            "assessmentId": first_assessment_id
        }),
    );

    let assessments_after = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "assessments.list",
        json!({ "classId": class_id.clone(), "markSetId": mark_set_id.clone(), "hideDeleted": false }),
    );
    let deleted_row = assessments_after
        .get("assessments")
        .and_then(|v| v.as_array())
        .and_then(|rows| {
            rows.iter().find(|r| {
                r.get("idx").and_then(|v| v.as_i64()) == Some(first_idx)
            })
        })
        .cloned()
        .expect("deleted row");
    assert_eq!(deleted_row.get("weight").and_then(|v| v.as_f64()), Some(0.0));
    assert_eq!(
        deleted_row.get("isDeletedLike").and_then(|v| v.as_bool()),
        Some(true)
    );

    let hidden_list = request_ok(
        &mut stdin,
        &mut reader,
        "8",
        "assessments.list",
        json!({ "classId": class_id.clone(), "markSetId": mark_set_id.clone(), "hideDeleted": true }),
    );
    let hidden_has_deleted = hidden_list
        .get("assessments")
        .and_then(|v| v.as_array())
        .map(|rows| rows.iter().any(|r| r.get("idx").and_then(|v| v.as_i64()) == Some(first_idx)))
        .unwrap_or(false);
    assert!(!hidden_has_deleted, "deleted-like entry should be hidden");

    let after_cell = request_ok(
        &mut stdin,
        &mut reader,
        "9",
        "grid.get",
        json!({
            "classId": class_id,
            "markSetId": mark_set_id,
            "rowStart": 0,
            "rowCount": 1,
            "colStart": first_idx,
            "colCount": 1
        }),
    );
    let after_value = after_cell
        .get("cells")
        .and_then(|v| v.as_array())
        .and_then(|rows| rows.first())
        .and_then(|row| row.as_array())
        .and_then(|row| row.first())
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    assert_eq!(after_value, before_value, "delete keeps existing marks");
}
