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

fn request_ok(
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
    assert!(
        value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false),
        "{} failed: {}",
        method,
        value
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error")
    );
    value.get("result").cloned().unwrap_or_else(|| json!({}))
}

#[test]
fn reports_models_smoke() {
    let workspace = temp_dir("markbook-reports-smoke");
    let fixture_folder = fixture_path("fixtures/legacy/Sample25/MB8D25");

    let (mut child, mut stdin, mut reader) = spawn_sidecar();
    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );
    let import_res = request_ok(
        &mut stdin,
        &mut reader,
        "2",
        "class.importLegacy",
        json!({ "legacyClassFolderPath": fixture_folder.to_string_lossy() }),
    );
    let class_id = import_res
        .get("classId")
        .and_then(|v| v.as_str())
        .expect("classId")
        .to_string();

    let marksets = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "marksets.list",
        json!({ "classId": class_id }),
    );
    let mark_set_id = marksets
        .get("markSets")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("id"))
        .and_then(|v| v.as_str())
        .expect("mark set id")
        .to_string();

    let grid = request_ok(
        &mut stdin,
        &mut reader,
        "grid",
        "reports.markSetGridModel",
        json!({ "classId": class_id, "markSetId": mark_set_id }),
    );
    let row_count = grid.get("rowCount").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let col_count = grid.get("colCount").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let students_len = grid
        .get("students")
        .and_then(|v| v.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0);
    let assessments_len = grid
        .get("assessments")
        .and_then(|v| v.as_array())
        .map(|arr| arr.len())
        .unwrap_or(0);
    assert_eq!(row_count, students_len);
    assert_eq!(col_count, assessments_len);
    assert_eq!(
        grid.get("cells")
            .and_then(|v| v.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0),
        row_count
    );

    let class_list = request_ok(
        &mut stdin,
        &mut reader,
        "class",
        "reports.classListModel",
        json!({ "classId": class_id }),
    );
    assert_eq!(
        class_list
            .get("class")
            .and_then(|c| c.get("id"))
            .and_then(|v| v.as_str()),
        Some(class_id.as_str())
    );
    assert!(
        class_list
            .get("students")
            .and_then(|v| v.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0)
            > 0,
        "expected non-empty class list students"
    );

    let attendance = request_ok(
        &mut stdin,
        &mut reader,
        "att",
        "reports.attendanceMonthlyModel",
        json!({ "classId": class_id, "month": "9" }),
    );
    assert_eq!(
        attendance
            .get("class")
            .and_then(|c| c.get("id"))
            .and_then(|v| v.as_str()),
        Some(class_id.as_str())
    );
    assert!(
        attendance
            .get("attendance")
            .and_then(|a| a.get("daysInMonth"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            > 0,
        "expected daysInMonth > 0"
    );

    let ls = request_ok(
        &mut stdin,
        &mut reader,
        "ls",
        "reports.learningSkillsSummaryModel",
        json!({ "classId": class_id, "term": 1 }),
    );
    assert_eq!(
        ls.get("class")
            .and_then(|c| c.get("id"))
            .and_then(|v| v.as_str()),
        Some(class_id.as_str())
    );

    drop(stdin);
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(workspace);
}
