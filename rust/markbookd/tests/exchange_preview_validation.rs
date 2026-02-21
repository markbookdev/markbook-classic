mod test_support;

use serde_json::json;
use std::fs;
use std::path::PathBuf;
use test_support::{fixture_path, request_ok, spawn_sidecar, temp_dir};

#[test]
fn exchange_preview_and_apply_report_deterministic_counts() {
    let workspace = temp_dir("markbook-exchange-preview");
    let out_dir = temp_dir("markbook-exchange-preview-out");
    let csv_path: PathBuf = out_dir.join("exchange.csv");
    let legacy_folder = fixture_path("fixtures/legacy/Sample25/MB8D25");
    let (_child, mut stdin, mut reader) = spawn_sidecar();

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );

    let imported = request_ok(
        &mut stdin,
        &mut reader,
        "2",
        "class.importLegacy",
        json!({ "legacyClassFolderPath": legacy_folder.to_string_lossy() }),
    );
    let class_id = imported
        .get("classId")
        .and_then(|v| v.as_str())
        .expect("classId")
        .to_string();

    let exported = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "exchange.exportClassCsv",
        json!({
            "classId": class_id,
            "outPath": csv_path.to_string_lossy()
        }),
    );
    assert!(
        exported
            .get("rowsExported")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            > 0
    );

    // Inject one bad student row for deterministic unmatched/warning behavior.
    let mut csv_text = fs::read_to_string(&csv_path).expect("read csv");
    csv_text.push_str("missing-student,\"Missing, Student\",MAT1,0,\"Injected\",scored,75\n");
    fs::write(&csv_path, csv_text).expect("write csv");

    let preview = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "exchange.previewClassCsv",
        json!({
            "classId": class_id,
            "inPath": csv_path.to_string_lossy(),
            "mode": "upsert"
        }),
    );
    assert!(
        preview
            .get("rowsTotal")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            >= preview
                .get("rowsParsed")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
    );
    assert!(
        preview
            .get("rowsMatched")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            > 0
    );
    assert!(
        preview
            .get("rowsUnmatched")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            > 0
    );
    assert!(
        preview
            .get("warningsCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            > 0
    );

    let applied = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "exchange.applyClassCsv",
        json!({
            "classId": class_id,
            "inPath": csv_path.to_string_lossy(),
            "mode": "upsert"
        }),
    );
    assert!(applied.get("updated").and_then(|v| v.as_u64()).unwrap_or(0) > 0);
    assert!(applied.get("skipped").and_then(|v| v.as_u64()).unwrap_or(0) > 0);
}
