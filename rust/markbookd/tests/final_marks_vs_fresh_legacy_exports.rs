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

#[test]
fn final_marks_match_fresh_legacy_exports_when_available() {
    let strict = std::env::var("MBC_STRICT_FRESH_SUMMARIES")
        .ok()
        .map(|v| matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false);

    let expected_path = fixture_path("fixtures/legacy/Sample25/expected/fresh-final-marks.json");
    if !expected_path.exists() {
        if strict {
            panic!(
                "strict final-mark parity requested but fresh export file is missing: {}",
                expected_path.display()
            );
        }
        eprintln!(
            "skipping strict final-mark parity: missing {}",
            expected_path.display()
        );
        return;
    }

    let expected_text =
        std::fs::read_to_string(&expected_path).expect("read fresh final marks export json");
    let expected: serde_json::Value =
        serde_json::from_str(&expected_text).expect("parse fresh final marks json");
    let expected_sets = expected
        .as_object()
        .expect("fresh-final-marks.json must be object keyed by mark set code");

    let workspace = temp_dir("markbook-fresh-final-marks-parity");
    let fixture_folder = fixture_path("fixtures/legacy/Sample25/MB8D25");
    let (mut child, mut stdin, mut reader) = spawn_sidecar();

    let _ = request(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );
    let import_res = request(
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

    let marksets = request(
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

    for (set_code, set_expected_val) in expected_sets {
        let mark_set_id = ids_by_code
            .get(set_code)
            .unwrap_or_else(|| panic!("missing mark set id for {}", set_code))
            .to_string();
        let summary = request(
            &mut stdin,
            &mut reader,
            &format!("sum-{}", set_code),
            "calc.markSetSummary",
            json!({
                "classId": class_id,
                "markSetId": mark_set_id
            }),
        );
        let mut actual_map: HashMap<String, Option<f64>> = HashMap::new();
        for student in summary
            .get("perStudent")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
        {
            let Some(name) = student.get("displayName").and_then(|v| v.as_str()) else {
                continue;
            };
            let mark = student.get("finalMark").and_then(|v| v.as_f64());
            actual_map.insert(name.to_string(), mark);
        }

        let expected_set = set_expected_val
            .as_object()
            .unwrap_or_else(|| panic!("expected set object for {}", set_code));
        for (name, expected_mark) in expected_set {
            let actual_mark = actual_map
                .get(name)
                .copied()
                .unwrap_or_else(|| panic!("missing final mark for {} in {}", name, set_code));

            if expected_mark.is_null() {
                assert!(
                    actual_mark.is_none(),
                    "expected null final mark for {} {}, got {:?}",
                    set_code,
                    name,
                    actual_mark
                );
                continue;
            }
            let expected_mark = expected_mark
                .as_f64()
                .unwrap_or_else(|| panic!("expected numeric mark for {} {}", set_code, name));
            let actual_mark = actual_mark.unwrap_or_else(|| {
                panic!(
                    "expected numeric final mark for {} {}, got null",
                    set_code, name
                )
            });
            let diff = (actual_mark - expected_mark).abs();
            assert!(
                diff <= 0.05,
                "final mark mismatch {} {}: expected {}, got {}",
                set_code,
                name,
                expected_mark,
                actual_mark
            );
        }
    }

    drop(stdin);
    let _ = child.wait();
    let _ = std::fs::remove_dir_all(workspace);
}
