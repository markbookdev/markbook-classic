mod test_support;

use serde_json::json;
use test_support::{fixture_path, request_ok, spawn_sidecar, temp_dir};

#[test]
fn classes_update_from_legacy_preserves_local_active_and_membership_mask() {
    let workspace = temp_dir("markbook-update-preserve-validity");
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

    let marksets = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "marksets.list",
        json!({ "classId": class_id }),
    );
    let mat1_id = marksets
        .get("markSets")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .find(|ms| ms.get("code").and_then(|v| v.as_str()) == Some("MAT1"))
        .and_then(|ms| ms.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .expect("MAT1 markSetId");

    let students = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "students.list",
        json!({ "classId": class_id }),
    );
    let student_id = students
        .get("students")
        .and_then(|v| v.as_array())
        .and_then(|rows| rows.first())
        .and_then(|r| r.get("id"))
        .and_then(|v| v.as_str())
        .expect("student id")
        .to_string();

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "students.update",
        json!({
            "classId": class_id,
            "studentId": student_id,
            "patch": {
                "active": false
            }
        }),
    );
    let set_res = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "students.membership.set",
        json!({
            "classId": class_id,
            "studentId": student_id,
            "markSetId": mat1_id,
            "enabled": false
        }),
    );
    let mask_before = set_res
        .get("mask")
        .and_then(|v| v.as_str())
        .expect("mask after set")
        .to_string();

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "classes.updateFromLegacy",
        json!({
            "classId": class_id,
            "legacyClassFolderPath": fixture_folder.to_string_lossy(),
            "mode": "upsert_preserve",
            "preserveLocalValidity": true
        }),
    );

    let students_after = request_ok(
        &mut stdin,
        &mut reader,
        "8",
        "students.list",
        json!({ "classId": class_id }),
    );
    let row_after = students_after
        .get("students")
        .and_then(|v| v.as_array())
        .and_then(|rows| {
            rows.iter()
                .find(|r| r.get("id").and_then(|v| v.as_str()) == Some(student_id.as_str()))
        })
        .cloned()
        .expect("student row after update");
    assert_eq!(
        row_after.get("active").and_then(|v| v.as_bool()),
        Some(false)
    );

    let membership_after = request_ok(
        &mut stdin,
        &mut reader,
        "9",
        "students.membership.get",
        json!({ "classId": class_id }),
    );
    let student_after = membership_after
        .get("students")
        .and_then(|v| v.as_array())
        .and_then(|rows| {
            rows.iter()
                .find(|r| r.get("id").and_then(|v| v.as_str()) == Some(student_id.as_str()))
        })
        .cloned()
        .expect("membership student row");
    let mask_after = student_after
        .get("mask")
        .and_then(|v| v.as_str())
        .expect("mask after update");
    assert_eq!(mask_after, mask_before);
}
