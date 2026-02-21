mod test_support;

use serde_json::json;
use test_support::{fixture_path, request, request_ok, spawn_sidecar, temp_dir};

#[test]
fn integrations_admin_transfer_apply_respects_collision_policies() {
    let workspace = temp_dir("markbook-integrations-admin-collision");
    let legacy_class_folder = fixture_path("fixtures/legacy/Sample25/MB8D25");
    let package_path = workspace.join("admin-transfer-collision.zip");
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

    let target_import = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "class.importLegacy",
        json!({ "legacyClassFolderPath": legacy_class_folder.to_string_lossy() }),
    );
    let target_class_id = target_import["classId"]
        .as_str()
        .expect("target class id")
        .to_string();

    let _ = request_ok(
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

    let stop = request(
        &mut stdin,
        &mut reader,
        "5",
        "integrations.adminTransfer.applyPackage",
        json!({
            "targetClassId": target_class_id,
            "inPath": package_path.to_string_lossy(),
            "matchMode": "student_no_then_name",
            "collisionPolicy": "stop_on_collision",
            "commentPolicy": "fill_blank"
        }),
    );
    assert_eq!(stop["ok"].as_bool(), Some(false), "expected stop policy error");
    assert_eq!(stop["error"]["code"].as_str(), Some("collision_conflict"));

    let merge = request_ok(
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
        merge["assessments"]["merged"].as_i64().unwrap_or(0) > 0,
        "expected merged assessments with merge_existing: {}",
        merge
    );

    let append = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "integrations.adminTransfer.applyPackage",
        json!({
            "targetClassId": target_class_id,
            "inPath": package_path.to_string_lossy(),
            "matchMode": "student_no_then_name",
            "collisionPolicy": "append_new",
            "commentPolicy": "fill_blank"
        }),
    );
    assert!(
        append["assessments"]["created"].as_i64().unwrap_or(0) > 0,
        "expected created assessments with append_new: {}",
        append
    );
}
