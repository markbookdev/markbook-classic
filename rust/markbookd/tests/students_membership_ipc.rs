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
fn students_membership_get_and_set_affects_calc_validity() {
    let workspace = temp_dir("markbook-students-membership");
    let fixture_folder = fixture_path("fixtures/legacy/Sample25/MB8D25");

    let (_child, mut stdin, mut reader) = spawn_sidecar();
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
    let mat1_id = marksets
        .get("markSets")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .find(|ms| ms.get("code").and_then(|v| v.as_str()) == Some("MAT1"))
        .and_then(|ms| ms.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .expect("MAT1 markSetId");

    // Pick a student with a computed (non-null) final mark, so the effect is meaningful.
    let sum_all = request_ok(
        &mut stdin,
        &mut reader,
        "sum_all",
        "calc.markSetSummary",
        json!({ "classId": class_id, "markSetId": mat1_id }),
    );
    let (student_id, baseline_final): (String, f64) = sum_all
        .get("perStudent")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .find_map(|s| {
            let sid = s.get("studentId").and_then(|v| v.as_str())?.to_string();
            let fm = s.get("finalMark").and_then(|v| v.as_f64())?;
            Some((sid, fm))
        })
        .expect("expected at least one student with final mark");

    // Discover markSet sort order via membership.get and ensure mask bit toggles.
    let mem = request_ok(
        &mut stdin,
        &mut reader,
        "mem",
        "students.membership.get",
        json!({ "classId": class_id }),
    );
    let mat1 = mem
        .get("markSets")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .find(|ms| ms.get("id").and_then(|v| v.as_str()) == Some(&mat1_id))
        .expect("MAT1 in membership markSets");
    let mat1_sort: usize = mat1
        .get("sortOrder")
        .and_then(|v| v.as_u64())
        .expect("sortOrder") as usize;

    // Disable membership for MAT1.
    let set_res = request_ok(
        &mut stdin,
        &mut reader,
        "set0",
        "students.membership.set",
        json!({ "classId": class_id, "studentId": student_id, "markSetId": mat1_id, "enabled": false }),
    );
    let mask = set_res.get("mask").and_then(|v| v.as_str()).unwrap_or("");
    assert!(mask.len() > mat1_sort, "mask should cover mark set index");
    assert_eq!(mask.as_bytes()[mat1_sort], b'0');

    // Student should now be invalid for this mark set => final mark must become null.
    let sum2 = request_ok(
        &mut stdin,
        &mut reader,
        "sum2",
        "calc.markSetSummary",
        json!({ "classId": class_id, "markSetId": mat1_id }),
    );
    let after = sum2
        .get("perStudent")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .find(|s| s.get("studentId").and_then(|v| v.as_str()) == Some(&student_id))
        .expect("student present in perStudent");
    assert!(
        after.get("finalMark").is_none() || after.get("finalMark") == Some(&serde_json::Value::Null),
        "finalMark should be null when membership disabled; got {}",
        after
    );

    // Re-enable and ensure final mark returns (close to baseline).
    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "set1",
        "students.membership.set",
        json!({ "classId": class_id, "studentId": student_id, "markSetId": mat1_id, "enabled": true }),
    );
    let sum3 = request_ok(
        &mut stdin,
        &mut reader,
        "sum3",
        "calc.markSetSummary",
        json!({ "classId": class_id, "markSetId": mat1_id }),
    );
    let after2 = sum3
        .get("perStudent")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .find(|s| s.get("studentId").and_then(|v| v.as_str()) == Some(&student_id))
        .expect("student present in perStudent");
    let fm = after2
        .get("finalMark")
        .and_then(|v| v.as_f64())
        .expect("finalMark should return after re-enable");
    assert!(
        (fm - baseline_final).abs() < 0.06,
        "finalMark should return to baseline: {} vs {}",
        fm,
        baseline_final
    );
}

