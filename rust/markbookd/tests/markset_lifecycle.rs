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
fn markset_create_clone_delete_undelete_default_flow() {
    let workspace = temp_dir("markbook-markset-lifecycle");
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

    let created = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "marksets.create",
        json!({
            "classId": class_id,
            "code": "NEW1",
            "description": "New Mark Set 1",
            "blockTitle": "Term",
            "weightMethod": 1,
            "calcMethod": 0,
            "makeDefault": true,
            "starterCategories": [{ "name": "Knowledge", "weight": 100 }]
        }),
    );
    let new_mark_set_id = created
        .get("markSetId")
        .and_then(|v| v.as_str())
        .expect("markSetId")
        .to_string();

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "marksets.clone",
        json!({
            "classId": class_id,
            "markSetId": new_mark_set_id,
            "code": "NEW2",
            "description": "New Mark Set 2",
            "cloneAssessments": true,
            "cloneScores": false
        }),
    );

    let list1 = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "marksets.list",
        json!({ "classId": class_id, "includeDeleted": true }),
    );
    let has_new2 = list1
        .get("markSets")
        .and_then(|v| v.as_array())
        .map(|rows| {
            rows.iter()
                .any(|r| r.get("code").and_then(|v| v.as_str()) == Some("NEW2"))
        })
        .unwrap_or(false);
    assert!(has_new2, "expected cloned mark set NEW2 in list");

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "marksets.delete",
        json!({ "classId": class_id, "markSetId": new_mark_set_id }),
    );

    let list2 = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "marksets.list",
        json!({ "classId": class_id, "includeDeleted": true }),
    );
    let deleted_row = list2
        .get("markSets")
        .and_then(|v| v.as_array())
        .and_then(|rows| {
            rows.iter().find(|r| {
                r.get("id").and_then(|v| v.as_str()) == Some(new_mark_set_id.as_str())
            })
        })
        .cloned()
        .expect("deleted row");
    assert!(
        deleted_row.get("deletedAt").and_then(|v| v.as_str()).is_some(),
        "expected deletedAt after marksets.delete"
    );

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "8",
        "marksets.undelete",
        json!({ "classId": class_id, "markSetId": new_mark_set_id }),
    );
    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "9",
        "marksets.setDefault",
        json!({ "classId": class_id, "markSetId": new_mark_set_id }),
    );

    let list3 = request_ok(
        &mut stdin,
        &mut reader,
        "10",
        "marksets.list",
        json!({ "classId": class_id, "includeDeleted": true }),
    );
    let default_row = list3
        .get("markSets")
        .and_then(|v| v.as_array())
        .and_then(|rows| {
            rows.iter().find(|r| {
                r.get("id").and_then(|v| v.as_str()) == Some(new_mark_set_id.as_str())
            })
        })
        .cloned()
        .expect("default row");
    assert_eq!(
        default_row.get("isDefault").and_then(|v| v.as_bool()),
        Some(true)
    );
}
