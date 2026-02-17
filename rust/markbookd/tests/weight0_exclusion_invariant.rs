use rusqlite::Connection;
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

fn is_valid_kid(active: bool, mark_set_mask: &str, mark_set_sort_order: i64) -> bool {
    if !active {
        return false;
    }
    let t = mark_set_mask.trim();
    if t.is_empty() {
        return true;
    }
    if t.eq_ignore_ascii_case("TBA") {
        return true;
    }
    let Ok(idx) = usize::try_from(mark_set_sort_order) else {
        return true;
    };
    let up = t.to_ascii_uppercase();
    if !up.chars().all(|ch| ch == '0' || ch == '1') {
        return true;
    }
    if idx >= up.len() {
        return true;
    }
    up.as_bytes()[idx] == b'1'
}

#[test]
fn weight_zero_assessment_does_not_affect_final_marks_but_is_still_reported_in_stats() {
    let workspace = temp_dir("markbook-weight0");
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

    let db_path = workspace.join("markbook.sqlite3");
    let conn = Connection::open(&db_path).expect("open db");

    let (mat1_id, mat1_sort_order): (String, i64) = conn
        .query_row(
            "SELECT id, sort_order FROM mark_sets WHERE class_id = ? AND code = ?",
            (&class_id, "MAT1"),
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("MAT1 mark set");
    assert_eq!(mat1_sort_order, 0, "fixture assumes MAT1 sort_order=0");

    let assessment_id: String = conn
        .query_row(
            "SELECT id FROM assessments WHERE mark_set_id = ? AND idx = 0",
            [&mat1_id],
            |r| r.get(0),
        )
        .expect("assessment idx 0");
    let assessment_weight: f64 = conn
        .query_row(
            "SELECT COALESCE(weight, 0) FROM assessments WHERE id = ?",
            [&assessment_id],
            |r| r.get(0),
        )
        .expect("assessment weight");
    assert!(
        assessment_weight <= 0.0,
        "fixture expects MAT1 assessment idx0 to have weight 0"
    );

    // Find a valid student with a scored value (>0) in this assessment.
    let mut picked: Option<(String, String)> = None; // (student_id, display_name)
    let mut stmt = conn
        .prepare(
            "SELECT id, last_name, first_name, active, COALESCE(mark_set_mask, 'TBA')
             FROM students
             WHERE class_id = ?
             ORDER BY sort_order",
        )
        .expect("prepare students");
    let rows = stmt
        .query_map([&class_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
                r.get::<_, String>(4)?,
            ))
        })
        .expect("query students");
    for row in rows.flatten() {
        let (student_id, last, first, active, mask) = row;
        if !is_valid_kid(active != 0, &mask, mat1_sort_order) {
            continue;
        }
        let (status, raw): (String, Option<f64>) = conn
            .query_row(
                "SELECT status, raw_value FROM scores WHERE assessment_id = ? AND student_id = ?",
                (&assessment_id, &student_id),
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .expect("score row");
        if status == "scored" && raw.unwrap_or(0.0) > 0.0 {
            picked = Some((student_id, format!("{}, {}", last, first)));
            break;
        }
    }
    let (student_id, display_name) = picked.expect("picked a scored student for idx0");

    // Baseline final mark.
    let summary1 = request_ok(
        &mut stdin,
        &mut reader,
        "sum1",
        "calc.markSetSummary",
        json!({ "classId": class_id, "markSetId": mat1_id }),
    );
    let baseline_mark = summary1
        .get("perStudent")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|s| s.get("displayName").and_then(|v| v.as_str()) == Some(&display_name))
        })
        .and_then(|s| s.get("finalMark"))
        .and_then(|v| v.as_f64())
        .expect("baseline finalMark");

    // Mutate the weight-0 assessment to a huge score; final mark must not change.
    let changed = conn
        .execute(
            "UPDATE scores SET raw_value = ?, status = 'scored' WHERE assessment_id = ? AND student_id = ?",
            (999.0_f64, &assessment_id, &student_id),
        )
        .expect("update score");
    assert_eq!(changed, 1, "expected to update existing score row");

    let summary2 = request_ok(
        &mut stdin,
        &mut reader,
        "sum2",
        "calc.markSetSummary",
        json!({ "classId": class_id, "markSetId": mat1_id }),
    );
    let next_mark = summary2
        .get("perStudent")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|s| s.get("displayName").and_then(|v| v.as_str()) == Some(&display_name))
        })
        .and_then(|s| s.get("finalMark"))
        .and_then(|v| v.as_f64())
        .expect("next finalMark");
    assert!(
        (next_mark - baseline_mark).abs() <= 0.05,
        "weight-0 assessment changed finalMark unexpectedly: baseline {} next {}",
        baseline_mark,
        next_mark
    );

    // Assessment stats still include idx=0 (visible even if excluded from final marks).
    let stats = request_ok(
        &mut stdin,
        &mut reader,
        "st1",
        "calc.assessmentStats",
        json!({ "classId": class_id, "markSetId": mat1_id, "filters": { "term": "ALL", "categoryName": "ALL", "typesMask": null } }),
    );
    let has_idx0 = stats
        .get("assessments")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().any(|a| a.get("idx").and_then(|v| v.as_i64()) == Some(0)))
        .unwrap_or(false);
    assert!(has_idx0, "expected assessment stats to include idx=0");

    drop(stdin);
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(workspace);
}

