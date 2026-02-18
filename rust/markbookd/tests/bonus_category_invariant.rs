use rusqlite::Connection;
use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

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

fn get_final_mark(summary: &serde_json::Value, display_name: &str) -> f64 {
    summary
        .get("perStudent")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|s| s.get("displayName").and_then(|v| v.as_str()) == Some(display_name))
        })
        .and_then(|s| s.get("finalMark"))
        .and_then(|v| v.as_f64())
        .expect("finalMark")
}

#[test]
fn bonus_category_is_added_outside_denominator() {
    let workspace = temp_dir("markbook-bonus");
    let (mut child, mut stdin, mut reader) = spawn_sidecar();
    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );

    let db_path = workspace.join("markbook.sqlite3");
    let conn = Connection::open(&db_path).expect("open db");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("fk on");

    let class_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO classes(id, name) VALUES(?, ?)",
        (&class_id, "Synthetic"),
    )
    .expect("insert class");

    let mark_set_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO mark_sets(id, class_id, code, file_prefix, description, sort_order, weight_method, calc_method)
         VALUES(?, ?, ?, ?, ?, ?, ?, ?)",
        (
            &mark_set_id,
            &class_id,
            "SYNB",
            "SYNB",
            "Synthetic Bonus",
            0_i64,
            1_i64, // category weighting
            0_i64,
        ),
    )
    .expect("insert mark set");

    let cat_main_id = Uuid::new_v4().to_string();
    let cat_bonus_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO categories(id, mark_set_id, name, weight, sort_order) VALUES(?, ?, ?, ?, ?)",
        (&cat_main_id, &mark_set_id, "Main", 100.0_f64, 0_i64),
    )
    .expect("insert main cat");
    conn.execute(
        "INSERT INTO categories(id, mark_set_id, name, weight, sort_order) VALUES(?, ?, ?, ?, ?)",
        (&cat_bonus_id, &mark_set_id, "BONUS", 10.0_f64, 1_i64),
    )
    .expect("insert bonus cat");

    let a_main_id = Uuid::new_v4().to_string();
    let a_bonus_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO assessments(id, mark_set_id, idx, category_name, title, weight, out_of)
         VALUES(?, ?, ?, ?, ?, ?, ?)",
        (&a_main_id, &mark_set_id, 0_i64, "Main", "Main1", 1.0_f64, 10.0_f64),
    )
    .expect("insert main assessment");
    conn.execute(
        "INSERT INTO assessments(id, mark_set_id, idx, category_name, title, weight, out_of)
         VALUES(?, ?, ?, ?, ?, ?, ?)",
        (&a_bonus_id, &mark_set_id, 1_i64, "BONUS", "Bonus1", 1.0_f64, 10.0_f64),
    )
    .expect("insert bonus assessment");

    let student_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO students(id, class_id, last_name, first_name, active, sort_order, raw_line, mark_set_mask)
         VALUES(?, ?, ?, ?, ?, ?, ?, ?)",
        (
            &student_id,
            &class_id,
            "Student",
            "One",
            1_i64,
            0_i64,
            "",
            "TBA",
        ),
    )
    .expect("insert student");

    // Base = 50% (5/10 in Main), Bonus = 100% (10/10), bonus weight 10 => +10. Final = 60.
    conn.execute(
        "INSERT INTO scores(id, assessment_id, student_id, raw_value, status) VALUES(?, ?, ?, ?, ?)",
        (Uuid::new_v4().to_string(), &a_main_id, &student_id, 5.0_f64, "scored"),
    )
    .expect("insert main score");
    conn.execute(
        "INSERT INTO scores(id, assessment_id, student_id, raw_value, status) VALUES(?, ?, ?, ?, ?)",
        (
            Uuid::new_v4().to_string(),
            &a_bonus_id,
            &student_id,
            10.0_f64,
            "scored",
        ),
    )
    .expect("insert bonus score");

    let display_name = "Student, One";
    let summary1 = request_ok(
        &mut stdin,
        &mut reader,
        "sum1",
        "calc.markSetSummary",
        json!({ "classId": class_id, "markSetId": mark_set_id }),
    );
    let m1 = get_final_mark(&summary1, display_name);
    assert!(
        (m1 - 60.0).abs() <= 0.05,
        "expected bonus add-on to yield 60.0, got {}",
        m1
    );

    // Change bonus to 80% (8/10) => add 8. Final = 58.
    conn.execute(
        "UPDATE scores SET raw_value = ? WHERE assessment_id = ? AND student_id = ?",
        (8.0_f64, &a_bonus_id, &student_id),
    )
    .expect("update bonus score");

    let summary2 = request_ok(
        &mut stdin,
        &mut reader,
        "sum2",
        "calc.markSetSummary",
        json!({ "classId": class_id, "markSetId": mark_set_id }),
    );
    let m2 = get_final_mark(&summary2, display_name);
    assert!(
        (m2 - 58.0).abs() <= 0.05,
        "expected bonus add-on to yield 58.0, got {}",
        m2
    );

    drop(stdin);
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(workspace);
}

