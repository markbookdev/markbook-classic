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
fn reports_models_honor_filters_and_student_scope() {
    let workspace = temp_dir("markbook-reports-filter-alignment");
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

    let filters = json!({
        "term": 1,
        "categoryName": serde_json::Value::Null,
        "typesMask": serde_json::Value::Null
    });

    let calc_summary = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "calc.markSetSummary",
        json!({
            "classId": class_id.clone(),
            "markSetId": mark_set_id.clone(),
            "filters": filters
        }),
    );
    let report_summary = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "reports.markSetSummaryModel",
        json!({
            "classId": class_id.clone(),
            "markSetId": mark_set_id.clone(),
            "filters": filters,
            "studentScope": "valid"
        }),
    );

    assert_eq!(
        report_summary
            .get("filters")
            .and_then(|v| v.get("term"))
            .and_then(|v| v.as_i64()),
        Some(1)
    );
    let calc_students = calc_summary
        .get("perStudent")
        .and_then(|v| v.as_array())
        .map(|v| v.len())
        .unwrap_or(0);
    let report_students = report_summary
        .get("perStudent")
        .and_then(|v| v.as_array())
        .map(|v| v.len())
        .unwrap_or(0);
    assert!(report_students <= calc_students);

    let grid_all = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "reports.markSetGridModel",
        json!({
            "classId": class_id.clone(),
            "markSetId": mark_set_id.clone(),
            "filters": filters,
            "studentScope": "all"
        }),
    );
    let grid_valid = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "reports.markSetGridModel",
        json!({
            "classId": class_id,
            "markSetId": mark_set_id,
            "filters": filters,
            "studentScope": "valid"
        }),
    );

    assert_eq!(
        grid_valid
            .get("filters")
            .and_then(|v| v.get("term"))
            .and_then(|v| v.as_i64()),
        Some(1)
    );
    let rows_all = grid_all
        .get("rowCount")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let rows_valid = grid_valid
        .get("rowCount")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    assert!(rows_valid <= rows_all);
}

