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
    if value.get("ok").and_then(|v| v.as_bool()) == Some(false) {
        let code = value
            .get("error")
            .and_then(|e| e.get("code"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        assert_ne!(
            code, "not_implemented",
            "unexpected unknown method for {}",
            method
        );
    }
    value
}

#[test]
fn router_dispatch_smoke_covers_handler_families() {
    let workspace = temp_dir("markbook-router-smoke");
    let bundle_out = workspace.join("smoke-backup.mbcbackup.zip");
    let csv_out = workspace.join("smoke-export.csv");

    let (mut child, mut stdin, mut reader) = spawn_sidecar();

    let _ = request(&mut stdin, &mut reader, "1", "health", json!({}));
    let _ = request(
        &mut stdin,
        &mut reader,
        "2",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );
    let created = request(
        &mut stdin,
        &mut reader,
        "3",
        "classes.create",
        json!({ "name": "Smoke Class" }),
    );
    let class_id = created
        .get("result")
        .and_then(|v| v.get("classId"))
        .and_then(|v| v.as_str())
        .expect("classId")
        .to_string();

    let _ = request(&mut stdin, &mut reader, "4", "classes.list", json!({}));
    let _ = request(
        &mut stdin,
        &mut reader,
        "5",
        "class.importLegacy",
        json!({}),
    );
    let _ = request(
        &mut stdin,
        &mut reader,
        "6",
        "marksets.list",
        json!({ "classId": class_id }),
    );
    let _ = request(
        &mut stdin,
        &mut reader,
        "7",
        "students.list",
        json!({ "classId": class_id }),
    );
    let created_student = request(
        &mut stdin,
        &mut reader,
        "7a",
        "students.create",
        json!({
            "classId": class_id,
            "lastName": "Smoke",
            "firstName": "Student",
            "active": true
        }),
    );
    let student_id = created_student
        .get("result")
        .and_then(|v| v.get("studentId"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if !student_id.is_empty() {
        let _ = request(
            &mut stdin,
            &mut reader,
            "7b",
            "students.update",
            json!({
                "classId": class_id,
                "studentId": student_id,
                "patch": { "firstName": "Updated" }
            }),
        );
        let _ = request(
            &mut stdin,
            &mut reader,
            "7c",
            "notes.update",
            json!({
                "classId": class_id,
                "studentId": student_id,
                "note": "router smoke note"
            }),
        );
    }
    let _ = request(
        &mut stdin,
        &mut reader,
        "8",
        "notes.get",
        json!({ "classId": class_id }),
    );
    let _ = request(
        &mut stdin,
        &mut reader,
        "8b",
        "grid.get",
        json!({
            "classId": class_id,
            "markSetId": "missing",
            "rowStart": 0,
            "rowCount": 1,
            "colStart": 0,
            "colCount": 1
        }),
    );
    let _ = request(
        &mut stdin,
        &mut reader,
        "8c",
        "categories.list",
        json!({ "classId": class_id, "markSetId": "missing" }),
    );
    let _ = request(
        &mut stdin,
        &mut reader,
        "9",
        "attendance.monthOpen",
        json!({ "classId": class_id, "month": "9" }),
    );
    let _ = request(
        &mut stdin,
        &mut reader,
        "10",
        "seating.get",
        json!({ "classId": class_id }),
    );
    let _ = request(
        &mut stdin,
        &mut reader,
        "11",
        "comments.banks.list",
        json!({}),
    );
    let _ = request(
        &mut stdin,
        &mut reader,
        "12",
        "calc.assessmentStats",
        json!({ "classId": class_id, "markSetId": "missing" }),
    );
    let _ = request(
        &mut stdin,
        &mut reader,
        "13",
        "reports.classListModel",
        json!({ "classId": class_id }),
    );
    let _ = request(
        &mut stdin,
        &mut reader,
        "14",
        "backup.exportWorkspaceBundle",
        json!({
            "workspacePath": workspace.to_string_lossy(),
            "outPath": bundle_out.to_string_lossy()
        }),
    );
    let _ = request(
        &mut stdin,
        &mut reader,
        "15",
        "backup.importWorkspaceBundle",
        json!({
            "workspacePath": workspace.to_string_lossy(),
            "inPath": bundle_out.to_string_lossy()
        }),
    );
    let _ = request(
        &mut stdin,
        &mut reader,
        "16",
        "exchange.exportClassCsv",
        json!({ "classId": class_id, "outPath": csv_out.to_string_lossy() }),
    );
    let _ = request(
        &mut stdin,
        &mut reader,
        "17",
        "loaned.list",
        json!({ "classId": class_id }),
    );
    let _ = request(
        &mut stdin,
        &mut reader,
        "18",
        "devices.list",
        json!({ "classId": class_id }),
    );
    let _ = request(
        &mut stdin,
        &mut reader,
        "19",
        "learningSkills.open",
        json!({ "classId": class_id }),
    );
    let _ = request(
        &mut stdin,
        &mut reader,
        "20",
        "classes.delete",
        json!({ "classId": class_id }),
    );

    drop(stdin);
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(workspace);
}
