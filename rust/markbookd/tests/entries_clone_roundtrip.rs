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
fn entries_clone_save_apply_roundtrip_copies_scores() {
    let workspace = temp_dir("markbook-entries-clone");
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
    let all_mark_sets = marksets
        .get("markSets")
        .and_then(|v| v.as_array())
        .cloned()
        .expect("mark sets");
    assert!(all_mark_sets.len() >= 2, "fixture should have at least 2 mark sets");
    let source_mark_set_id = all_mark_sets[0]
        .get("id")
        .and_then(|v| v.as_str())
        .expect("source mark set id")
        .to_string();
    let target_mark_set_id = all_mark_sets[1]
        .get("id")
        .and_then(|v| v.as_str())
        .expect("target mark set id")
        .to_string();

    let source_assessments = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "assessments.list",
        json!({ "classId": class_id.clone(), "markSetId": source_mark_set_id.clone(), "hideDeleted": false }),
    );
    let first_source_assessment = source_assessments
        .get("assessments")
        .and_then(|v| v.as_array())
        .and_then(|rows| rows.first())
        .cloned()
        .expect("source assessment");
    let source_assessment_id = first_source_assessment
        .get("id")
        .and_then(|v| v.as_str())
        .expect("source assessment id")
        .to_string();
    let source_idx = first_source_assessment
        .get("idx")
        .and_then(|v| v.as_i64())
        .expect("source idx");

    let source_col = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "grid.get",
        json!({
            "classId": class_id.clone(),
            "markSetId": source_mark_set_id.clone(),
            "rowStart": 0,
            "rowCount": 27,
            "colStart": source_idx,
            "colCount": 1
        }),
    );
    let rows = source_col
        .get("cells")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let (probe_row, probe_value) = rows
        .iter()
        .enumerate()
        .find_map(|(idx, row)| {
            row.as_array()
                .and_then(|cols| cols.first())
                .cloned()
                .and_then(|v| {
                    if v.is_null() {
                        None
                    } else {
                        Some((idx as i64, v))
                    }
                })
        })
        .expect("at least one non-null source value");

    let target_before = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "assessments.list",
        json!({ "classId": class_id.clone(), "markSetId": target_mark_set_id.clone(), "hideDeleted": false }),
    );
    let target_before_count = target_before
        .get("assessments")
        .and_then(|v| v.as_array())
        .map(|rows| rows.len())
        .unwrap_or(0);

    let saved = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "entries.clone.save",
        json!({
            "classId": class_id.clone(),
            "markSetId": source_mark_set_id.clone(),
            "assessmentId": source_assessment_id
        }),
    );
    assert_eq!(
        saved
            .get("clone")
            .and_then(|v| v.get("sourceMarkSetId"))
            .and_then(|v| v.as_str()),
        Some(source_mark_set_id.as_str())
    );

    let peek = request_ok(
        &mut stdin,
        &mut reader,
        "8",
        "entries.clone.peek",
        json!({ "classId": class_id.clone() }),
    );
    assert_eq!(
        peek.get("clone")
            .and_then(|v| v.get("exists"))
            .and_then(|v| v.as_bool()),
        Some(true)
    );

    let applied = request_ok(
        &mut stdin,
        &mut reader,
        "9",
        "entries.clone.apply",
        json!({
            "classId": class_id.clone(),
            "markSetId": target_mark_set_id.clone(),
            "titleMode": "appendClone"
        }),
    );
    let new_assessment_id = applied
        .get("assessmentId")
        .and_then(|v| v.as_str())
        .expect("assessmentId")
        .to_string();

    let target_after = request_ok(
        &mut stdin,
        &mut reader,
        "10",
        "assessments.list",
        json!({ "classId": class_id.clone(), "markSetId": target_mark_set_id.clone(), "hideDeleted": false }),
    );
    let target_after_rows = target_after
        .get("assessments")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert_eq!(target_after_rows.len(), target_before_count + 1);
    let inserted = target_after_rows
        .iter()
        .find(|row| row.get("id").and_then(|v| v.as_str()) == Some(new_assessment_id.as_str()))
        .cloned()
        .expect("inserted assessment");
    let inserted_idx = inserted
        .get("idx")
        .and_then(|v| v.as_i64())
        .expect("inserted idx");

    let cloned_probe = request_ok(
        &mut stdin,
        &mut reader,
        "11",
        "grid.get",
        json!({
            "classId": class_id,
            "markSetId": target_mark_set_id,
            "rowStart": probe_row,
            "rowCount": 1,
            "colStart": inserted_idx,
            "colCount": 1
        }),
    );
    let cloned_value = cloned_probe
        .get("cells")
        .and_then(|v| v.as_array())
        .and_then(|rows| rows.first())
        .and_then(|row| row.as_array())
        .and_then(|row| row.first())
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    assert_eq!(
        cloned_value, probe_value,
        "clone apply should copy source score values by student sort order"
    );
}
