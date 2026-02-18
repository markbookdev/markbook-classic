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

fn db_path(workspace: &PathBuf) -> PathBuf {
    workspace.join("markbook.sqlite3")
}

fn setup_one_student_markset(
    workspace: &PathBuf,
    class_id: &str,
    student_id: &str,
    mark_set_id: &str,
    calc_method: i64,
    weight_method: i64,
    categories: &[(&str, f64, i64)],
    assessments: &[(&str, i64, &str, f64, f64)],
    // (assessment_id, raw_value, status)
    scores: &[(&str, Option<f64>, &str)],
) {
    use rusqlite::Connection;
    let conn = Connection::open(db_path(workspace)).expect("open db");

    conn.execute(
        "INSERT INTO classes(id, name) VALUES(?, ?)",
        (class_id, "Test Class"),
    )
    .expect("insert class");
    conn.execute(
        "INSERT INTO students(id, class_id, last_name, first_name, student_no, birth_date, active, sort_order, raw_line, mark_set_mask, updated_at)
         VALUES(?, ?, ?, ?, NULL, NULL, 1, 0, 'RAW', 'TBA', NULL)",
        (student_id, class_id, "Student", "One"),
    )
    .expect("insert student");
    conn.execute(
        "INSERT INTO mark_sets(id, class_id, code, file_prefix, description, weight, source_filename, sort_order, full_code, room, day, period, weight_method, calc_method)
         VALUES(?, ?, 'TST', 'TST', 'Test', 1.0, NULL, 0, NULL, NULL, NULL, NULL, ?, ?)",
        (mark_set_id, class_id, weight_method, calc_method),
    )
    .expect("insert mark set");

    for (name, weight, sort_order) in categories {
        conn.execute(
            "INSERT INTO categories(id, mark_set_id, name, weight, sort_order)
             VALUES(lower(hex(randomblob(16))), ?, ?, ?, ?)",
            (mark_set_id, *name, *weight, *sort_order),
        )
        .expect("insert category");
    }

    for (aid, idx, cat_name, weight, out_of) in assessments {
        conn.execute(
            "INSERT INTO assessments(id, mark_set_id, idx, date, category_name, title, term, legacy_type, weight, out_of, avg_percent, avg_raw)
             VALUES(?, ?, ?, NULL, ?, ?, 1, 0, ?, ?, 0, 0)",
            (*aid, mark_set_id, *idx, *cat_name, format!("A{}", idx), *weight, *out_of),
        )
        .expect("insert assessment");
    }

    for (aid, raw, status) in scores {
        conn.execute(
            "INSERT INTO scores(id, assessment_id, student_id, raw_value, status)
             VALUES(lower(hex(randomblob(16))), ?, ?, ?, ?)",
            (*aid, student_id, *raw, *status),
        )
        .expect("insert score");
    }
}

#[test]
fn bonus_add_applies_only_for_average_not_median() {
    let workspace = temp_dir("markbook-calc-bonus");
    let (mut child, mut stdin, mut reader) = spawn_sidecar();
    let _ = request(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );

    // Average case: A=80, BONUS=100 with BONUS weight 20 => 80 + 100*0.2 = 100.
    setup_one_student_markset(
        &workspace,
        "c1",
        "s1",
        "m_avg",
        0,
        1,
        &[("A", 100.0, 0), ("BONUS", 20.0, 1)],
        &[
            ("a1", 0, "A", 1.0, 100.0),
            ("a2", 1, "BONUS", 1.0, 100.0),
        ],
        &[("a1", Some(80.0), "scored"), ("a2", Some(100.0), "scored")],
    );
    let avg_summary = request(
        &mut stdin,
        &mut reader,
        "2",
        "calc.markSetSummary",
        json!({ "classId": "c1", "markSetId": "m_avg", "filters": {} }),
    );
    let avg_mark = avg_summary["perStudent"][0]["finalMark"]
        .as_f64()
        .expect("avg finalMark");
    assert!((avg_mark - 100.0).abs() < 1e-6);

    // Median case: same marks but calcMethod=Median => median(80,100)=90 (no BONUS add-on).
    setup_one_student_markset(
        &workspace,
        "c2",
        "s2",
        "m_med",
        1,
        1,
        &[("A", 100.0, 0), ("BONUS", 20.0, 1)],
        &[
            ("b1", 0, "A", 1.0, 100.0),
            ("b2", 1, "BONUS", 1.0, 100.0),
        ],
        &[("b1", Some(80.0), "scored"), ("b2", Some(100.0), "scored")],
    );
    let med_summary = request(
        &mut stdin,
        &mut reader,
        "3",
        "calc.markSetSummary",
        json!({ "classId": "c2", "markSetId": "m_med", "filters": {} }),
    );
    let med_mark = med_summary["perStudent"][0]["finalMark"]
        .as_f64()
        .expect("median finalMark");
    assert!((med_mark - 90.0).abs() < 1e-6);

    let _ = child.kill();
}

#[test]
fn mode_levels_in_workspace_settings_affect_mode_result() {
    let workspace = temp_dir("markbook-calc-mode-levels");
    let (mut child, mut stdin, mut reader) = spawn_sidecar();
    let _ = request(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );

    // Setup a simple markset: one entry worth 62%.
    setup_one_student_markset(
        &workspace,
        "c1",
        "s1",
        "m1",
        2,
        0,
        &[("A", 100.0, 0)],
        &[("a1", 0, "A", 1.0, 100.0)],
        &[("a1", Some(62.0), "scored")],
    );

    // Default thresholds (0/50/60/70/80): 62 -> level2 midrange (60..70) = 65.
    let s1 = request(
        &mut stdin,
        &mut reader,
        "2",
        "calc.markSetSummary",
        json!({ "classId": "c1", "markSetId": "m1", "filters": {} }),
    );
    let m1 = s1["perStudent"][0]["finalMark"].as_f64().expect("finalMark");
    assert!((m1 - 65.0).abs() < 1e-6);

    // Override mode levels: make the second threshold 50 and third 70.
    // Then 62 maps to level1 midrange (50..70) = 60.
    {
        use rusqlite::Connection;
        let conn = Connection::open(db_path(&workspace)).expect("open db");
        conn.execute(
            "INSERT INTO workspace_settings(key, value_json) VALUES('user_cfg.mode_levels', ?)
             ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json",
            [json!({
                "activeLevels": 4,
                "vals": [0, 50, 70, 80, 90, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
                "symbols": []
            })
            .to_string()],
        )
        .expect("set mode levels");
        conn.execute(
            "INSERT INTO workspace_settings(key, value_json) VALUES('user_cfg.roff', '{\"roff\":false}')
             ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json",
            [],
        )
        .expect("set roff");
    }

    let s2 = request(
        &mut stdin,
        &mut reader,
        "3",
        "calc.markSetSummary",
        json!({ "classId": "c1", "markSetId": "m1", "filters": {} }),
    );
    let m2 = s2["perStudent"][0]["finalMark"].as_f64().expect("finalMark");
    assert!((m2 - 60.0).abs() < 1e-6);

    let _ = child.kill();
}

#[test]
fn mode_tie_breaks_to_higher_level() {
    let workspace = temp_dir("markbook-calc-mode-tie");
    let (mut child, mut stdin, mut reader) = spawn_sidecar();
    let _ = request(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );

    // Two entries: 55% (level1 midrange=55) and 65% (level2 midrange=65), equal weights.
    setup_one_student_markset(
        &workspace,
        "c1",
        "s1",
        "m1",
        2,
        0,
        &[("A", 100.0, 0)],
        &[
            ("a1", 0, "A", 1.0, 100.0),
            ("a2", 1, "A", 1.0, 100.0),
        ],
        &[("a1", Some(55.0), "scored"), ("a2", Some(65.0), "scored")],
    );

    let s1 = request(
        &mut stdin,
        &mut reader,
        "2",
        "calc.markSetSummary",
        json!({ "classId": "c1", "markSetId": "m1", "filters": {} }),
    );
    let mark = s1["perStudent"][0]["finalMark"].as_f64().expect("finalMark");
    assert!((mark - 65.0).abs() < 1e-6);

    let _ = child.kill();
}

#[test]
fn blended_methods_ignore_category_filter_and_force_category_weighting() {
    let workspace = temp_dir("markbook-calc-blend-filter");
    let (mut child, mut stdin, mut reader) = spawn_sidecar();
    let _ = request(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );

    // Two categories with equal weights. Two entries in different cats.
    setup_one_student_markset(
        &workspace,
        "c1",
        "s1",
        "m1",
        4, // blended median
        0, // user asked entry weighting, but blend should force category
        &[("A", 50.0, 0), ("B", 50.0, 1)],
        &[
            ("a1", 0, "A", 1.0, 100.0),
            ("a2", 1, "B", 1.0, 100.0),
        ],
        &[("a1", Some(40.0), "scored"), ("a2", Some(80.0), "scored")],
    );

    let s_all = request(
        &mut stdin,
        &mut reader,
        "2",
        "calc.markSetSummary",
        json!({ "classId": "c1", "markSetId": "m1", "filters": { "categoryName": "A" } }),
    );
    // VB6 ignores category filter for blended methods, so applied filter should be null.
    assert!(s_all["filters"]["categoryName"].is_null());
    // And settingsApplied should show category weighting applied.
    assert_eq!(
        s_all["settingsApplied"]["weightMethodApplied"].as_i64(),
        Some(1)
    );

    let mark = s_all["perStudent"][0]["finalMark"].as_f64().expect("finalMark");
    // Category medians are 40 and 80; equal weights => 60 overall.
    assert!((mark - 60.0).abs() < 1e-6);

    let _ = child.kill();
}

