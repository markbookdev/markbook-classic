mod test_support;

use serde_json::json;
use test_support::{fixture_path, request_ok, spawn_sidecar, temp_dir};

#[test]
fn classes_update_from_legacy_upsert_preserves_local_only_and_appends_sort_order() {
    let workspace = temp_dir("markbook-update-upsert");
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

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "students.create",
        json!({
            "classId": class_id,
            "lastName": "LocalOnly",
            "firstName": "Student",
            "studentNo": "LOCAL-ONLY-1",
            "active": true
        }),
    );

    let preview = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "classes.legacyPreview",
        json!({
            "classId": class_id,
            "legacyClassFolderPath": fixture_folder.to_string_lossy()
        }),
    );
    assert!(
        preview
            .get("students")
            .and_then(|v| v.get("matched"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
            > 0
    );
    assert!(
        preview
            .get("students")
            .and_then(|v| v.get("localOnly"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
            >= 1
    );

    let update = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "classes.updateFromLegacy",
        json!({
            "classId": class_id,
            "legacyClassFolderPath": fixture_folder.to_string_lossy()
        }),
    );
    assert_eq!(update.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert!(
        update
            .get("students")
            .and_then(|v| v.get("updated"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
            > 0
    );
    assert!(
        update
            .get("students")
            .and_then(|v| v.get("localOnly"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
            >= 1
    );

    let students = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "students.list",
        json!({ "classId": class_id }),
    );
    let rows = students
        .get("students")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let local_only = rows
        .iter()
        .find(|s| {
            s.get("lastName").and_then(|v| v.as_str()) == Some("LocalOnly")
                && s.get("firstName").and_then(|v| v.as_str()) == Some("Student")
        })
        .cloned()
        .expect("local-only student present after update");
    let max_sort = rows
        .iter()
        .filter_map(|s| s.get("sortOrder").and_then(|v| v.as_i64()))
        .max()
        .unwrap_or(-1);
    assert_eq!(
        local_only.get("sortOrder").and_then(|v| v.as_i64()),
        Some(max_sort),
        "local-only student should remain appended after incoming rows"
    );
}
