mod test_support;

use serde_json::json;
use test_support::{fixture_path, request_ok, spawn_sidecar, temp_dir};

fn find_mark_set_id_by_code(marksets: &serde_json::Value, code: &str) -> Option<String> {
    marksets
        .get("markSets")
        .and_then(|v| v.as_array())
        .and_then(|rows| {
            rows.iter()
                .find(|m| m.get("code").and_then(|v| v.as_str()) == Some(code))
        })
        .and_then(|m| m.get("id").and_then(|v| v.as_str()))
        .map(|s| s.to_string())
}

#[test]
fn markset_transfer_preview_and_apply_merges_or_appends() {
    let workspace = temp_dir("markbook-markset-transfer");
    let fixture_folder = fixture_path("fixtures/legacy/Sample25/MB8D25");
    let (_child, mut stdin, mut reader) = spawn_sidecar();

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );

    let import_a = request_ok(
        &mut stdin,
        &mut reader,
        "2",
        "class.importLegacy",
        json!({ "legacyClassFolderPath": fixture_folder.to_string_lossy() }),
    );
    let import_b = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "class.importLegacy",
        json!({ "legacyClassFolderPath": fixture_folder.to_string_lossy() }),
    );

    let source_class_id = import_a
        .get("classId")
        .and_then(|v| v.as_str())
        .expect("source class")
        .to_string();
    let target_class_id = import_b
        .get("classId")
        .and_then(|v| v.as_str())
        .expect("target class")
        .to_string();

    let source_marksets = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "marksets.list",
        json!({ "classId": source_class_id }),
    );
    let target_marksets = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "marksets.list",
        json!({ "classId": target_class_id }),
    );

    let source_mark_set_id =
        find_mark_set_id_by_code(&source_marksets, "MAT1").expect("source MAT1");
    let target_mark_set_id =
        find_mark_set_id_by_code(&target_marksets, "MAT1").expect("target MAT1");

    let preview = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "marksets.transfer.preview",
        json!({
            "sourceClassId": source_class_id,
            "sourceMarkSetId": source_mark_set_id,
            "targetClassId": target_class_id,
            "targetMarkSetId": target_mark_set_id
        }),
    );
    assert!(
        preview
            .get("sourceAssessmentCount")
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
            > 0
    );

    let merged = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "marksets.transfer.apply",
        json!({
            "sourceClassId": source_class_id,
            "sourceMarkSetId": source_mark_set_id,
            "targetClassId": target_class_id,
            "targetMarkSetId": target_mark_set_id,
            "collisionPolicy": "merge_existing"
        }),
    );
    assert!(
        merged
            .get("assessments")
            .and_then(|v| v.get("merged"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
            >= 1
    );
    assert!(
        merged
            .get("scores")
            .and_then(|v| v.get("upserted"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
            > 0
    );

    let appended = request_ok(
        &mut stdin,
        &mut reader,
        "8",
        "marksets.transfer.apply",
        json!({
            "sourceClassId": source_class_id,
            "sourceMarkSetId": source_mark_set_id,
            "targetClassId": target_class_id,
            "targetMarkSetId": target_mark_set_id,
            "collisionPolicy": "append_new",
            "titleMode": "appendTransfer"
        }),
    );
    assert!(
        appended
            .get("assessments")
            .and_then(|v| v.get("created"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
            > 0
    );
}
