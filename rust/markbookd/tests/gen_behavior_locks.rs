use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

fn fixture_path(rel: &str) -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    base.join("../../").join(rel)
}

fn temp_dir(prefix: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "{}-{}",
        prefix,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
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
fn generate_calc_behavior_locks_when_enabled() {
    if std::env::var("MBC_GEN_LOCKS").ok().as_deref() != Some("1") {
        // Not a "skip" because this is an integration test binary; just a no-op.
        return;
    }

    let out_path = fixture_path("fixtures/legacy/Sample25/expected/calc-behavior-locks.json");

    let workspace = temp_dir("markbook-gen-calc-behavior-locks");
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

    let markset_codes = ["MAT1", "MAT2", "MAT3", "SNC1", "SNC2", "SNC3"];
    let students = [
        "O'Shanter, Tam",
        "Boame, Gerald",
        "Beach, Shelley",
        "Bell, Clarissa",
        "Stone, Edward",
        "Hughes, Amber",
        "Lowe, Glenda",
        "Wilco, Roger",
    ];
    let calc_methods = [0_i64, 1, 2, 3, 4];
    let term_keys = ["ALL", "1", "2", "3"];

    let mut marksets_obj = serde_json::Map::new();

    for code in markset_codes {
        let mark_set_id = id_by_code
            .get(code)
            .unwrap_or_else(|| panic!("mark set {} not found in import", code))
            .to_string();

        let mut methods_obj = serde_json::Map::new();
        for calc_method in calc_methods {
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

            let mut term_obj = serde_json::Map::new();
            for term_key in term_keys {
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
                let mut exp = serde_json::Map::new();
                for s in &students {
                    exp.insert(
                        s.to_string(),
                        actual_by_name
                            .get(*s)
                            .cloned()
                            .unwrap_or(None)
                            .map(|v| json!(v))
                            .unwrap_or(serde_json::Value::Null),
                    );
                }
                term_obj.insert(term_key.to_string(), serde_json::Value::Object(exp));
            }

            // One types-mask variant (type 0 only), term ALL.
            {
                let sum = request_ok(
                    &mut stdin,
                    &mut reader,
                    &format!("sum-{}-{}-ALL_T0", code, calc_method),
                    "calc.markSetSummary",
                    json!({
                        "classId": class_id,
                        "markSetId": mark_set_id,
                        "filters": { "term": serde_json::Value::Null, "categoryName": serde_json::Value::Null, "typesMask": json!(1) }
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
                let mut exp = serde_json::Map::new();
                for s in &students {
                    exp.insert(
                        s.to_string(),
                        actual_by_name
                            .get(*s)
                            .cloned()
                            .unwrap_or(None)
                            .map(|v| json!(v))
                            .unwrap_or(serde_json::Value::Null),
                    );
                }
                term_obj.insert("ALL_T0".to_string(), serde_json::Value::Object(exp));
            }

            methods_obj.insert(calc_method.to_string(), serde_json::Value::Object(term_obj));
        }

        marksets_obj.insert(
            code.to_string(),
            json!({
                "students": students,
                "methods": methods_obj
            }),
        );
    }

    let out = json!({
        "version": 2,
        "generatedAt": format!("{:?}", std::time::SystemTime::now()),
        "markSets": marksets_obj
    });
    fs::write(&out_path, serde_json::to_string_pretty(&out).expect("json")).expect("write locks");

    let _ = child.kill();
}
