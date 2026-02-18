use serde_json::json;
use std::collections::HashMap;
use std::fs;
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
    );
    value.get("result").cloned().unwrap_or_else(|| json!({}))
}

#[test]
fn sample25_calc_behavior_locks_hold() {
    let locks_path = fixture_path("fixtures/legacy/Sample25/expected/calc-behavior-locks.json");
    let text = fs::read_to_string(&locks_path).expect("read calc-behavior-locks.json");
    let locks: serde_json::Value = serde_json::from_str(&text).expect("parse json");

    let marksets_obj = locks
        .get("markSets")
        .and_then(|v| v.as_object())
        .expect("markSets object");

    let workspace = temp_dir("markbook-calc-behavior-locks");
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

    let ms_list = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "marksets.list",
        json!({ "classId": class_id }),
    );
    let mut id_by_code: HashMap<String, String> = HashMap::new();
    for ms in ms_list
        .get("markSets")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
    {
        if let (Some(code), Some(id)) = (
            ms.get("code").and_then(|v| v.as_str()),
            ms.get("id").and_then(|v| v.as_str()),
        ) {
            id_by_code.insert(code.to_string(), id.to_string());
        }
    }

    for (code, cfg) in marksets_obj {
        let mark_set_id = id_by_code
            .get(code)
            .unwrap_or_else(|| panic!("mark set {} not found in import", code))
            .to_string();

        let students: Vec<String> = cfg
            .get("students")
            .and_then(|v| v.as_array())
            .expect("students array")
            .iter()
            .map(|v| v.as_str().expect("student name").to_string())
            .collect();

        let methods = cfg
            .get("methods")
            .and_then(|v| v.as_object())
            .expect("methods object");

        for (method_s, method_cfg) in methods {
            let calc_method: i64 = method_s.parse().expect("calc method int key");
            let _ = request_ok(
                &mut stdin,
                &mut reader,
                &format!("set-{}-{}", code, calc_method),
                "markset.settings.update",
                json!({
                    "classId": class_id,
                    "markSetId": mark_set_id,
                    "patch": { "calcMethod": calc_method }
                }),
            );

            let terms = method_cfg
                .as_object()
                .expect("method terms object");
            for (term_key, expected_map) in terms {
                let term = if term_key == "ALL" {
                    serde_json::Value::Null
                } else {
                    json!(term_key.parse::<i64>().expect("term int key"))
                };
                let sum = request_ok(
                    &mut stdin,
                    &mut reader,
                    &format!("sum-{}-{}-{}", code, calc_method, term_key),
                    "calc.markSetSummary",
                    json!({
                        "classId": class_id,
                        "markSetId": mark_set_id,
                        "filters": { "term": term, "categoryName": serde_json::Value::Null, "typesMask": serde_json::Value::Null }
                    }),
                );

                let mut actual_by_name: HashMap<String, Option<f64>> = HashMap::new();
                for row in sum
                    .get("perStudent")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default()
                {
                    let Some(name) = row.get("displayName").and_then(|v| v.as_str()) else {
                        continue;
                    };
                    let fm = row.get("finalMark").and_then(|v| v.as_f64());
                    actual_by_name.insert(name.to_string(), fm);
                }

                let expected_obj = expected_map
                    .as_object()
                    .expect("expected student map object");
                for student_name in &students {
                    let expected = expected_obj
                        .get(student_name)
                        .unwrap_or_else(|| panic!("missing expected value for {}", student_name));
                    let expected_val = expected.as_f64();
                    let actual_val = actual_by_name.get(student_name).cloned().unwrap_or(None);
                    match (expected_val, actual_val) {
                        (None, None) => {}
                        (Some(e), Some(a)) => {
                            assert!(
                                (a - e).abs() <= 0.05,
                                "{} {} term {}: expected {}, got {}",
                                code,
                                calc_method,
                                term_key,
                                e,
                                a
                            );
                        }
                        (None, Some(a)) => panic!(
                            "{} {} term {}: expected null, got {}",
                            code, calc_method, term_key, a
                        ),
                        (Some(e), None) => panic!(
                            "{} {} term {}: expected {}, got null",
                            code, calc_method, term_key, e
                        ),
                    }
                }
            }
        }
    }

    let _ = child.kill();
}

