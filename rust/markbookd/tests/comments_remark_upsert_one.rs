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
fn comments_remarks_upsert_one_saves_and_clears_single_student_remark() {
    let workspace = temp_dir("markbook-comments-remark-upsert");
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

    let students = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "students.list",
        json!({ "classId": class_id.clone() }),
    );
    let student_id = students
        .get("students")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("id"))
        .and_then(|v| v.as_str())
        .expect("student id")
        .to_string();

    let sets = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "comments.sets.list",
        json!({ "classId": class_id.clone(), "markSetId": mark_set_id.clone() }),
    );
    let set_number = sets
        .get("sets")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("setNumber"))
        .and_then(|v| v.as_i64())
        .unwrap_or(1);

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "comments.remarks.upsertOne",
        json!({
            "classId": class_id.clone(),
            "markSetId": mark_set_id.clone(),
            "setNumber": set_number,
            "studentId": student_id.clone(),
            "remark": "Great improvement on chapter review."
        }),
    );

    let open = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "comments.sets.open",
        json!({
            "classId": class_id.clone(),
            "markSetId": mark_set_id.clone(),
            "setNumber": set_number
        }),
    );
    let remark = open
        .get("remarksByStudent")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|row| row.get("studentId").and_then(|v| v.as_str()) == Some(student_id.as_str()))
        })
        .and_then(|row| row.get("remark"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert_eq!(remark, "Great improvement on chapter review.");

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "8",
        "comments.remarks.upsertOne",
        json!({
            "classId": class_id,
            "markSetId": mark_set_id,
            "setNumber": set_number,
            "studentId": student_id.clone(),
            "remark": ""
        }),
    );

    let open_after_clear = request_ok(
        &mut stdin,
        &mut reader,
        "9",
        "comments.sets.open",
        json!({
            "classId": class_id,
            "markSetId": mark_set_id,
            "setNumber": set_number
        }),
    );
    let cleared = open_after_clear
        .get("remarksByStudent")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|row| row.get("studentId").and_then(|v| v.as_str()) == Some(student_id.as_str()))
        })
        .and_then(|row| row.get("remark"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert_eq!(cleared, "");
}
