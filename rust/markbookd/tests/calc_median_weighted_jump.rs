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

fn request_ok(
    stdin: &mut ChildStdin,
    reader: &mut BufReader<ChildStdout>,
    id: &str,
    method: &str,
    params: serde_json::Value,
) -> serde_json::Value {
    let payload = json!({ "id": id, "method": method, "params": params });
    writeln!(stdin, "{}", payload).expect("write request");
    stdin.flush().expect("flush request");
    let mut line = String::new();
    reader.read_line(&mut line).expect("read response line");
    let value: serde_json::Value = serde_json::from_str(line.trim()).expect("parse response json");
    assert!(
        value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false),
        "{} failed: {}",
        method,
        value
    );
    value.get("result").cloned().unwrap_or_else(|| json!({}))
}

fn db_path(workspace: &PathBuf) -> PathBuf {
    workspace.join("markbook.sqlite3")
}

fn setup_one_student_median_markset(workspace: &PathBuf, w1: f64, p1: f64, w2: f64, p2: f64) {
    use rusqlite::Connection;
    let conn = Connection::open(db_path(workspace)).expect("open db");
    conn.execute("INSERT INTO classes(id, name) VALUES('c1','Test')", [])
        .expect("class");
    conn.execute(
        "INSERT INTO students(id, class_id, last_name, first_name, student_no, birth_date, active, sort_order, raw_line, mark_set_mask, updated_at)
         VALUES('s1','c1','Student','One',NULL,NULL,1,0,'RAW','TBA',NULL)",
        [],
    )
    .expect("student");
    conn.execute(
        "INSERT INTO mark_sets(id, class_id, code, file_prefix, description, weight, source_filename, sort_order, full_code, room, day, period, weight_method, calc_method)
         VALUES('m1','c1','TST','TST','Test',1.0,NULL,0,NULL,NULL,NULL,NULL,0,1)",
        [],
    )
    .expect("mark set");
    conn.execute(
        "INSERT INTO categories(id, mark_set_id, name, weight, sort_order)
         VALUES('cat1','m1','A',100.0,0)",
        [],
    )
    .expect("category");
    conn.execute(
        "INSERT INTO assessments(id, mark_set_id, idx, date, category_name, title, term, legacy_type, weight, out_of, avg_percent, avg_raw)
         VALUES('a1','m1',0,NULL,'A','A1',1,0,?,100.0,0,0)",
        [w1],
    )
    .expect("assessment1");
    conn.execute(
        "INSERT INTO assessments(id, mark_set_id, idx, date, category_name, title, term, legacy_type, weight, out_of, avg_percent, avg_raw)
         VALUES('a2','m1',1,NULL,'A','A2',1,0,?,100.0,0,0)",
        [w2],
    )
    .expect("assessment2");
    conn.execute(
        "INSERT INTO scores(id, assessment_id, student_id, raw_value, status)
         VALUES('sc1','a1','s1',?,'scored')",
        [p1],
    )
    .expect("score1");
    conn.execute(
        "INSERT INTO scores(id, assessment_id, student_id, raw_value, status)
         VALUES('sc2','a2','s1',?,'scored')",
        [p2],
    )
    .expect("score2");
}

#[test]
fn median_respects_weights_when_not_equal() {
    let workspace = temp_dir("markbook-calc-median-weighted");
    let (mut child, mut stdin, mut reader) = spawn_sidecar();
    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );

    // Two entries: 20% with weight 9, 90% with weight 1 => weighted median should be 20.
    setup_one_student_median_markset(&workspace, 9.0, 20.0, 1.0, 90.0);
    let s1 = request_ok(
        &mut stdin,
        &mut reader,
        "2",
        "calc.markSetSummary",
        json!({ "classId": "c1", "markSetId": "m1", "filters": {} }),
    );
    let m1 = s1["perStudent"][0]["finalMark"].as_f64().expect("finalMark");
    assert!(
        (m1 - 20.0).abs() < 1e-6,
        "expected weighted median 20.0, got {}",
        m1
    );

    let _ = child.kill();
}

#[test]
fn median_averages_middle_two_when_weights_equal() {
    let workspace = temp_dir("markbook-calc-median-equal");
    let (mut child, mut stdin, mut reader) = spawn_sidecar();
    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );

    // Two entries with equal weights => average.
    setup_one_student_median_markset(&workspace, 1.0, 20.0, 1.0, 90.0);
    let s1 = request_ok(
        &mut stdin,
        &mut reader,
        "2",
        "calc.markSetSummary",
        json!({ "classId": "c1", "markSetId": "m1", "filters": {} }),
    );
    let m1 = s1["perStudent"][0]["finalMark"].as_f64().expect("finalMark");
    assert!(
        (m1 - 55.0).abs() < 1e-6,
        "expected median average 55.0, got {}",
        m1
    );

    let _ = child.kill();
}

