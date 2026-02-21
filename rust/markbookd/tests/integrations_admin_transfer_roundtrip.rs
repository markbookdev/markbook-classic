mod test_support;

use serde_json::json;
use test_support::{fixture_path, request_ok, spawn_sidecar, temp_dir};

fn find_mark_set_by_code(marksets: &serde_json::Value, code: &str) -> Option<serde_json::Value> {
    marksets
        .get("markSets")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|m| m.get("code").and_then(|v| v.as_str()) == Some(code))
        })
        .cloned()
}

#[test]
fn integrations_admin_transfer_roundtrip_exports_previews_and_applies() {
    let workspace = temp_dir("markbook-integrations-admin-roundtrip");
    let legacy_class_folder = fixture_path("fixtures/legacy/Sample25/MB8D25");
    let package_path = workspace.join("admin-transfer.zip");
    let (_child, mut stdin, mut reader) = spawn_sidecar();

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );
    let source_import = request_ok(
        &mut stdin,
        &mut reader,
        "2",
        "class.importLegacy",
        json!({ "legacyClassFolderPath": legacy_class_folder.to_string_lossy() }),
    );
    let source_class_id = source_import["classId"]
        .as_str()
        .expect("source class id")
        .to_string();

    let target = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "classes.create",
        json!({ "name": "Admin Transfer Target" }),
    );
    let target_class_id = target["classId"]
        .as_str()
        .expect("target class id")
        .to_string();

    let export_res = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "integrations.adminTransfer.exportPackage",
        json!({
            "classId": source_class_id,
            "outPath": package_path.to_string_lossy(),
            "includeComments": true,
            "includeLearningSkills": true
        }),
    );
    assert_eq!(
        export_res["format"].as_str(),
        Some("mb-admin-transfer-v1"),
        "unexpected format: {}",
        export_res
    );
    assert!(
        export_res["entriesWritten"].as_i64().unwrap_or(0) > 0,
        "expected entries written: {}",
        export_res
    );
    assert!(package_path.is_file(), "package not written");

    let preview_res = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "integrations.adminTransfer.previewPackage",
        json!({
            "targetClassId": target_class_id,
            "inPath": package_path.to_string_lossy(),
            "matchMode": "student_no_then_name"
        }),
    );
    assert!(
        preview_res["markSetCount"].as_i64().unwrap_or(0) > 0,
        "expected mark sets in package: {}",
        preview_res
    );

    let apply_res = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "integrations.adminTransfer.applyPackage",
        json!({
            "targetClassId": target_class_id,
            "inPath": package_path.to_string_lossy(),
            "matchMode": "student_no_then_name",
            "collisionPolicy": "merge_existing",
            "commentPolicy": "fill_blank"
        }),
    );
    assert!(
        apply_res["assessments"]["created"].as_i64().unwrap_or(0) > 0,
        "expected created assessments: {}",
        apply_res
    );
    assert!(
        apply_res["scores"]["upserted"].as_i64().unwrap_or(0) > 0,
        "expected upserted scores: {}",
        apply_res
    );

    let target_marksets = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "marksets.list",
        json!({ "classId": target_class_id }),
    );
    assert!(
        find_mark_set_by_code(&target_marksets, "MAT1").is_some(),
        "expected MAT1 in target class mark sets: {}",
        target_marksets
    );
}
