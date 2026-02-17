#[path = "../src/legacy.rs"]
mod legacy;

use serde_json::json;
use std::collections::HashMap;
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

fn round_off_1_decimal(x: f64) -> f64 {
    ((10.0 * x) + 0.5).floor() / 10.0
}

#[test]
fn assessment_stats_match_legacy_mark_file_summaries_sample25() {
    let workspace = temp_dir("markbook-assessment-stats-parity");
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

    let cases: Vec<(&str, &str)> = vec![
        ("MAT1", "MAT18D.Y25"),
        ("MAT2", "MAT28D.Y25"),
        ("MAT3", "MAT38D.Y25"),
        ("SNC1", "SNC18D.Y25"),
        ("SNC2", "SNC28D.Y25"),
        ("SNC3", "SNC38D.Y25"),
    ];

    for (set_code, file_name) in cases {
        let mark_set_id = ids_by_code
            .get(set_code)
            .unwrap_or_else(|| panic!("missing mark set id for {}", set_code))
            .to_string();

        let open = request_ok(
            &mut stdin,
            &mut reader,
            &format!("open-{}", set_code),
            "markset.open",
            json!({ "classId": class_id, "markSetId": mark_set_id }),
        );
        let active_by_order: Vec<bool> = open
            .get("students")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .iter()
            .map(|s| s.get("active").and_then(|v| v.as_bool()).unwrap_or(true))
            .collect();

        let stats = request_ok(
            &mut stdin,
            &mut reader,
            &format!("stats-{}", set_code),
            "calc.assessmentStats",
            json!({ "classId": class_id, "markSetId": mark_set_id }),
        );
        let actual = stats
            .get("assessments")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let mut actual_by_idx: HashMap<i64, serde_json::Value> = HashMap::new();
        for a in actual {
            let idx = a.get("idx").and_then(|v| v.as_i64()).unwrap_or(-1);
            actual_by_idx.insert(idx, a);
        }

        let mark_file_path = fixture_folder.join(file_name);
        let legacy_file = legacy::parse_legacy_mark_file(&mark_file_path)
            .unwrap_or_else(|e| panic!("parse {}: {:?}", file_name, e));
        assert_eq!(
            legacy_file.last_student,
            active_by_order.len(),
            "{}: expected last_student to match students length",
            file_name
        );

        for legacy_a in legacy_file.assessments {
            let idx = legacy_a.idx as i64;
            let actual = actual_by_idx
                .get(&idx)
                .unwrap_or_else(|| panic!("{} missing assessment idx {}", set_code, idx));

            // Legacy mark files store per-assessment averages in the summary line, but those values
            // can become stale if the class list validity flags change after a calculation pass.
            // For parity with VB6 `Calculate`, recompute expected averages from raw marks plus the
            // current active mask (valid_kid) used by the sidecar.
            let mut denom = 0usize;
            let mut sum_raw = 0.0_f64;
            for (i, score) in legacy_a.raw_scores.iter().enumerate() {
                if !*active_by_order.get(i).unwrap_or(&true) {
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
            let expected_avg_percent_unrounded = if legacy_a.out_of > 0.0 {
                100.0 * expected_avg_raw_unrounded / legacy_a.out_of
            } else {
                0.0
            };
            let expected_avg_raw = round_off_1_decimal(expected_avg_raw_unrounded);
            let expected_avg_percent = round_off_1_decimal(expected_avg_percent_unrounded);
            let actual_avg_raw = actual.get("avgRaw").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let actual_avg_percent = actual
                .get("avgPercent")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            let raw_diff = (actual_avg_raw - expected_avg_raw).abs();
            let pct_diff = (actual_avg_percent - expected_avg_percent).abs();
            assert!(
                raw_diff <= 0.05,
                "{} idx {} avgRaw mismatch: expected {}, got {}",
                set_code,
                idx,
                expected_avg_raw,
                actual_avg_raw
            );
            assert!(
                pct_diff <= 0.05,
                "{} idx {} avgPercent mismatch: expected {}, got {}",
                set_code,
                idx,
                expected_avg_percent,
                actual_avg_percent
            );

            let mut expected_scored = 0usize;
            let mut expected_zero = 0usize;
            let mut expected_no_mark = 0usize;
            for (i, score) in legacy_a.raw_scores.iter().enumerate() {
                if !*active_by_order.get(i).unwrap_or(&true) {
                    continue;
                }
                match score {
                    legacy::LegacyScore::NoMark => expected_no_mark += 1,
                    legacy::LegacyScore::Zero => expected_zero += 1,
                    legacy::LegacyScore::Scored(_) => expected_scored += 1,
                }
            }

            let actual_scored = actual
                .get("scoredCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            let actual_zero = actual
                .get("zeroCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            let actual_no_mark = actual
                .get("noMarkCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;

            assert_eq!(
                actual_scored, expected_scored,
                "{} idx {} scoredCount mismatch",
                set_code, idx
            );
            assert_eq!(
                actual_zero, expected_zero,
                "{} idx {} zeroCount mismatch",
                set_code, idx
            );
            assert_eq!(
                actual_no_mark, expected_no_mark,
                "{} idx {} noMarkCount mismatch",
                set_code, idx
            );
        }
    }

    drop(stdin);
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(workspace);
}
