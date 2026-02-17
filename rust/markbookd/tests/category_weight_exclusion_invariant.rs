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
fn category_weight_zero_excludes_that_category_from_final_mark() {
    let workspace = temp_dir("markbook-catwt0");
    let (mut child, mut stdin, mut reader) = spawn_sidecar();
    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );

    // Insert a tiny synthetic class/mark set directly into the workspace DB.
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
            "SYN1",
            "SYN1",
            "Synthetic 1",
            0_i64,
            1_i64, // category weighting
            0_i64,
        ),
    )
    .expect("insert mark set");

    let cat_a_id = Uuid::new_v4().to_string();
    let cat_b_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO categories(id, mark_set_id, name, weight, sort_order) VALUES(?, ?, ?, ?, ?)",
        (&cat_a_id, &mark_set_id, "CatA", 100.0_f64, 0_i64),
    )
    .expect("insert cat A");
    conn.execute(
        "INSERT INTO categories(id, mark_set_id, name, weight, sort_order) VALUES(?, ?, ?, ?, ?)",
        (&cat_b_id, &mark_set_id, "CatB", 0.0_f64, 1_i64),
    )
    .expect("insert cat B");

    let a1_id = Uuid::new_v4().to_string();
    let a2_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO assessments(id, mark_set_id, idx, category_name, title, weight, out_of)
         VALUES(?, ?, ?, ?, ?, ?, ?)",
        (&a1_id, &mark_set_id, 0_i64, "CatA", "A1", 1.0_f64, 10.0_f64),
    )
    .expect("insert assessment A1");
    conn.execute(
        "INSERT INTO assessments(id, mark_set_id, idx, category_name, title, weight, out_of)
         VALUES(?, ?, ?, ?, ?, ?, ?)",
        (&a2_id, &mark_set_id, 1_i64, "CatB", "B1", 1.0_f64, 10.0_f64),
    )
    .expect("insert assessment B1");

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

    // CatA scored 100%, CatB scored 50% but CatB has weight 0, so final should be 100%.
    let s1_id = Uuid::new_v4().to_string();
    let s2_id = Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO scores(id, assessment_id, student_id, raw_value, status) VALUES(?, ?, ?, ?, ?)",
        (&s1_id, &a1_id, &student_id, 10.0_f64, "scored"),
    )
    .expect("insert score A1");
    conn.execute(
        "INSERT INTO scores(id, assessment_id, student_id, raw_value, status) VALUES(?, ?, ?, ?, ?)",
        (&s2_id, &a2_id, &student_id, 5.0_f64, "scored"),
    )
    .expect("insert score B1");

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
        (m1 - 100.0).abs() <= 0.05,
        "expected CatB excluded (weight 0), got {}",
        m1
    );

    // Change CatB drastically; final should remain unchanged.
    conn.execute(
        "UPDATE scores SET raw_value = ? WHERE assessment_id = ? AND student_id = ?",
        (1.0_f64, &a2_id, &student_id),
    )
    .expect("update B1 score");

    let summary2 = request_ok(
        &mut stdin,
        &mut reader,
        "sum2",
        "calc.markSetSummary",
        json!({ "classId": class_id, "markSetId": mark_set_id }),
    );
    let m2 = get_final_mark(&summary2, display_name);
    assert!(
        (m2 - m1).abs() <= 0.05,
        "CatB change affected final mark unexpectedly: {} -> {}",
        m1,
        m2
    );

    drop(stdin);
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(workspace);
}

