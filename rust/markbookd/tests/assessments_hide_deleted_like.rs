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
fn assessments_list_respects_deleted_like_and_hide_preference() {
    let workspace = temp_dir("markbook-hide-deleted-like");
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

    let pref_default = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "marks.pref.hideDeleted.get",
        json!({ "classId": class_id.clone(), "markSetId": mark_set_id.clone() }),
    );
    assert_eq!(
        pref_default.get("hideDeleted").and_then(|v| v.as_bool()),
        Some(true)
    );

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "marks.pref.hideDeleted.set",
        json!({ "classId": class_id.clone(), "markSetId": mark_set_id.clone(), "hideDeleted": false }),
    );
    let pref_false = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "marks.pref.hideDeleted.get",
        json!({ "classId": class_id.clone(), "markSetId": mark_set_id.clone() }),
    );
    assert_eq!(
        pref_false.get("hideDeleted").and_then(|v| v.as_bool()),
        Some(false)
    );

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "markset.settings.update",
        json!({
            "classId": class_id.clone(),
            "markSetId": mark_set_id.clone(),
            "patch": { "weightMethod": 1 }
        }),
    );

    let created_cat = request_ok(
        &mut stdin,
        &mut reader,
        "8",
        "categories.create",
        json!({
            "classId": class_id.clone(),
            "markSetId": mark_set_id.clone(),
            "name": "DeletedCat",
            "weight": 0
        }),
    );
    assert!(
        created_cat.get("categoryId").and_then(|v| v.as_str()).is_some(),
        "expected category creation"
    );

    let created_assessment = request_ok(
        &mut stdin,
        &mut reader,
        "9",
        "assessments.create",
        json!({
            "classId": class_id.clone(),
            "markSetId": mark_set_id.clone(),
            "title": "Deleted-Like By Category",
            "categoryName": "DeletedCat",
            "weight": 1,
            "outOf": 10
        }),
    );
    let created_assessment_id = created_assessment
        .get("assessmentId")
        .and_then(|v| v.as_str())
        .expect("assessmentId")
        .to_string();

    let list_all = request_ok(
        &mut stdin,
        &mut reader,
        "10",
        "assessments.list",
        json!({
            "classId": class_id.clone(),
            "markSetId": mark_set_id.clone(),
            "hideDeleted": false
        }),
    );
    let created_row = list_all
        .get("assessments")
        .and_then(|v| v.as_array())
        .and_then(|rows| {
            rows.iter()
                .find(|row| row.get("id").and_then(|v| v.as_str()) == Some(created_assessment_id.as_str()))
        })
        .cloned()
        .expect("created assessment row");
    assert_eq!(
        created_row.get("isDeletedLike").and_then(|v| v.as_bool()),
        Some(true)
    );

    let list_hidden = request_ok(
        &mut stdin,
        &mut reader,
        "11",
        "assessments.list",
        json!({
            "classId": class_id,
            "markSetId": mark_set_id,
            "hideDeleted": true
        }),
    );
    let still_visible = list_hidden
        .get("assessments")
        .and_then(|v| v.as_array())
        .map(|rows| {
            rows.iter()
                .any(|row| row.get("id").and_then(|v| v.as_str()) == Some(created_assessment_id.as_str()))
        })
        .unwrap_or(false);
    assert!(!still_visible, "deleted-like assessment should be hidden");
}
