#[path = "../src/legacy.rs"]
mod legacy;

use rusqlite::Connection;
use serde_json::json;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::Duration;
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

fn round_off_1_decimal(x: f64) -> f64 {
    ((10.0 * x) + 0.5).floor() / 10.0
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
fn membership_mask_affects_assessment_averages_and_final_marks() {
    let workspace = temp_dir("markbook-valid-kid-mask");
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
    let mut ids_by_code: HashMap<String, String> = HashMap::new();
    for ms in marksets
        .get("markSets")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
    {
        if let (Some(code), Some(id)) = (
            ms.get("code").and_then(|v| v.as_str()),
            ms.get("id").and_then(|v| v.as_str()),
        ) {
            ids_by_code.insert(code.to_string(), id.to_string());
        }
    }
    let mat1_id = ids_by_code.get("MAT1").expect("MAT1 id").to_string();

    // Flip Tam's membership bit for MAT1 off.
    let db_path = workspace.join("markbook.sqlite3");
    let conn = Connection::open(&db_path).expect("open db");
    conn.busy_timeout(Duration::from_secs(2))
        .expect("busy timeout");

    let mark_set_sort_order: i64 = conn
        .query_row("SELECT sort_order FROM mark_sets WHERE id = ?", [&mat1_id], |r| r.get(0))
        .expect("mark set sort order");
    assert_eq!(mark_set_sort_order, 0, "fixture assumes MAT1 sort_order=0");

    let (tam_id, tam_mask, tam_active): (String, String, i64) = conn
        .query_row(
            "SELECT id, COALESCE(mark_set_mask, 'TBA'), active
             FROM students
             WHERE class_id = ? AND last_name = ? AND first_name = ?",
            (&class_id, "O'Shanter", "Tam"),
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .expect("Tam row");
    assert_eq!(tam_active, 1, "Tam should be active in fixture");
    assert!(
        tam_mask.eq_ignore_ascii_case("TBA")
            || tam_mask.chars().all(|ch| ch == '0' || ch == '1'),
        "unexpected mask value: {}",
        tam_mask
    );

    let mut new_mask = tam_mask.trim().to_ascii_uppercase();
    if new_mask.eq_ignore_ascii_case("TBA") {
        new_mask = "0".to_string();
    } else if !new_mask.is_empty() {
        let mut chars: Vec<char> = new_mask.chars().collect();
        chars[0] = '0';
        new_mask = chars.into_iter().collect();
    }

    conn.execute(
        "UPDATE students SET mark_set_mask = ? WHERE id = ?",
        (&new_mask, &tam_id),
    )
    .expect("update mask");

    // Recompute expected assessment-0 avg_raw from the legacy mark file raw values + valid_kid mask.
    let mut valid_by_order: Vec<bool> = Vec::new();
    let mut stmt = conn
        .prepare(
            "SELECT active, COALESCE(mark_set_mask, 'TBA')
             FROM students
             WHERE class_id = ?
             ORDER BY sort_order",
        )
        .expect("prepare students");
    let rows = stmt
        .query_map([&class_id], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
        .expect("query students");
    for row in rows {
        let (active, mask) = row.expect("row");
        valid_by_order.push(is_valid_kid(active != 0, &mask, mark_set_sort_order));
    }

    let mark_file = fixture_folder.join("MAT18D.Y25");
    let legacy_file = legacy::parse_legacy_mark_file(&mark_file).expect("parse mark file");
    let a0 = legacy_file.assessments.get(0).expect("assessment 0");
    assert_eq!(
        a0.raw_scores.len(),
        valid_by_order.len(),
        "raw score row count must match students"
    );

    let mut denom = 0usize;
    let mut sum_raw = 0.0_f64;
    for (i, score) in a0.raw_scores.iter().enumerate() {
        if !*valid_by_order.get(i).unwrap_or(&true) {
            continue;
        }
        match score {
            legacy::LegacyScore::NoMark => {}
            legacy::LegacyScore::Zero => denom += 1,
            legacy::LegacyScore::Scored(v) => {
                denom += 1;
                sum_raw += *v;
            }
        }
    }
    let expected_avg_raw_unrounded = if denom > 0 {
        sum_raw / (denom as f64)
    } else {
        0.0
    };
    let expected_avg_raw = round_off_1_decimal(expected_avg_raw_unrounded);

    // Sidecar should reflect the changed membership in calc.assessmentStats.
    let stats = request_ok(
        &mut stdin,
        &mut reader,
        "stats",
        "calc.assessmentStats",
        json!({ "classId": class_id, "markSetId": mat1_id }),
    );
    let actual = stats
        .get("assessments")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .find(|a| a.get("idx").and_then(|v| v.as_i64()) == Some(0))
        .expect("idx 0 stats");
    let actual_avg_raw = actual.get("avgRaw").and_then(|v| v.as_f64()).unwrap_or(0.0);
    assert!(
        (actual_avg_raw - expected_avg_raw).abs() <= 0.05,
        "avgRaw mismatch after membership change: expected {}, got {}",
        expected_avg_raw,
        actual_avg_raw
    );

    // And Tam should have no final mark for MAT1 since they are no longer valid_kid for this set.
    let summary = request_ok(
        &mut stdin,
        &mut reader,
        "sum",
        "calc.markSetSummary",
        json!({ "classId": class_id, "markSetId": mat1_id }),
    );
    let tam_row = summary
        .get("perStudent")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .find(|s| s.get("displayName").and_then(|v| v.as_str()) == Some("O'Shanter, Tam"))
        .expect("Tam in perStudent");
    assert!(
        tam_row.get("finalMark").is_none() || tam_row.get("finalMark").unwrap().is_null(),
        "expected finalMark to be null for Tam when not valid_kid"
    );

    drop(stdin);
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(workspace);
}
