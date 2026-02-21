mod test_support;

use serde_json::json;
use test_support::{fixture_path, request, request_ok, spawn_sidecar, temp_dir};

#[test]
fn integrations_sis_preview_and_apply_import_updates_and_creates_students() {
    let workspace = temp_dir("markbook-integrations-sis-preview-apply");
    let legacy_class_folder = fixture_path("fixtures/legacy/Sample25/MB8D25");
    let csv_path = workspace.join("sis-import.csv");
    let (_child, mut stdin, mut reader) = spawn_sidecar();

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );
    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "2",
        "class.importLegacy",
        json!({ "legacyClassFolderPath": legacy_class_folder.to_string_lossy() }),
    );
    let classes = request_ok(&mut stdin, &mut reader, "3", "classes.list", json!({}));
    let class_id = classes["classes"][0]["id"]
        .as_str()
        .expect("class id")
        .to_string();
    let students_before = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "students.list",
        json!({ "classId": class_id }),
    );
    let first = students_before["students"][0].clone();
    let student_no = first["studentNo"].as_str().unwrap_or("");
    let last_name = first["lastName"].as_str().unwrap_or("Last");
    let first_name = first["firstName"].as_str().unwrap_or("First");
    let import_csv = format!(
        "student_no,last_name,first_name,active,birth_date\n{},{},{},1,2008-01-01\n900001,Newman,Sam,1,2009-02-02\n",
        student_no,
        last_name,
        first_name
    );
    std::fs::write(&csv_path, import_csv).expect("write sis csv");

    let preview = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "integrations.sis.previewImport",
        json!({
            "classId": class_id,
            "inPath": csv_path.to_string_lossy(),
            "profile": "sis_roster_v1",
            "matchMode": "student_no_then_name",
            "mode": "upsert_preserve"
        }),
    );
    assert!(
        preview["matched"].as_i64().unwrap_or(0) >= 1,
        "expected at least one matched row: {}",
        preview
    );
    assert!(
        preview["new"].as_i64().unwrap_or(0) >= 1,
        "expected at least one new row: {}",
        preview
    );

    let apply = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "integrations.sis.applyImport",
        json!({
            "classId": class_id,
            "inPath": csv_path.to_string_lossy(),
            "profile": "sis_roster_v1",
            "matchMode": "student_no_then_name",
            "mode": "upsert_preserve",
            "collisionPolicy": "merge_existing"
        }),
    );
    assert!(
        apply["created"].as_i64().unwrap_or(0) >= 1,
        "expected at least one created student: {}",
        apply
    );

    let students_after = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "students.list",
        json!({ "classId": class_id }),
    );
    let has_new = students_after["students"]
        .as_array()
        .unwrap_or(&Vec::new())
        .iter()
        .any(|s| s["lastName"].as_str() == Some("Newman") && s["firstName"].as_str() == Some("Sam"));
    assert!(has_new, "expected imported new student in class roster");

    // invalid match mode should return bad_params
    let invalid = request(
        &mut stdin,
        &mut reader,
        "8",
        "integrations.sis.previewImport",
        json!({
            "classId": class_id,
            "inPath": csv_path.to_string_lossy(),
            "matchMode": "unknown_mode"
        }),
    );
    assert_eq!(
        invalid.pointer("/error/code").and_then(|v| v.as_str()),
        Some("bad_params")
    );
}
