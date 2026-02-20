mod test_support;

use serde_json::json;
use test_support::{fixture_path, request, request_ok, spawn_sidecar, temp_dir};

#[test]
fn classes_update_from_legacy_respects_collision_policy_modes() {
    let workspace = temp_dir("markbook-update-collision-policy");
    let fixture_folder = fixture_path("fixtures/legacy/Sample25/MB8D25");
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
        json!({ "legacyClassFolderPath": fixture_folder.to_string_lossy() }),
    );
    let class_id = imported
        .get("classId")
        .and_then(|v| v.as_str())
        .expect("classId")
        .to_string();

    let stop = request(
        &mut stdin,
        &mut reader,
        "3",
        "classes.updateFromLegacy",
        json!({
            "classId": class_id,
            "legacyClassFolderPath": fixture_folder.to_string_lossy(),
            "collisionPolicy": "stop_on_collision"
        }),
    );
    assert_eq!(stop.get("ok").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(
        stop.get("error")
            .and_then(|v| v.get("code"))
            .and_then(|v| v.as_str()),
        Some("collision_conflict")
    );

    let merged = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "classes.updateFromLegacy",
        json!({
            "classId": class_id,
            "legacyClassFolderPath": fixture_folder.to_string_lossy(),
            "collisionPolicy": "merge_existing"
        }),
    );
    assert!(
        merged
            .get("assessments")
            .and_then(|v| v.get("matched"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
            > 0
    );

    let appended = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "classes.updateFromLegacy",
        json!({
            "classId": class_id,
            "legacyClassFolderPath": fixture_folder.to_string_lossy(),
            "collisionPolicy": "append_new"
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
