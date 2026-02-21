mod test_support;

use serde_json::json;
use test_support::{fixture_path, request_ok, spawn_sidecar, temp_dir};

#[test]
fn classes_update_from_attached_legacy_uses_linked_folder_and_preserves_membership() {
    let workspace = temp_dir("markbook-update-attached-legacy");
    let fixture_folder = fixture_path("fixtures/legacy/Sample25/MB8D25");
    let fixture_folder_s = fixture_folder.to_string_lossy().to_string();
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
        json!({ "legacyClassFolderPath": fixture_folder_s }),
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
        .and_then(|arr| {
            arr.iter()
                .find(|m| m.get("code").and_then(|v| v.as_str()) == Some("MAT1"))
        })
        .and_then(|m| m.get("id"))
        .and_then(|v| v.as_str())
        .expect("MAT1 mark set id")
        .to_string();

    let membership = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "students.membership.get",
        json!({ "classId": class_id }),
    );
    let target_student_id = membership
        .get("students")
        .and_then(|v| v.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|s| s.get("active").and_then(|v| v.as_bool()) == Some(true))
                .or_else(|| arr.first())
        })
        .and_then(|s| s.get("id"))
        .and_then(|v| v.as_str())
        .expect("student id")
        .to_string();

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "students.membership.set",
        json!({
            "classId": class_id,
            "studentId": target_student_id,
            "markSetId": mat1_id,
            "enabled": false
        }),
    );

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "classes.importLink.set",
        json!({
            "classId": class_id,
            "legacyClassFolderPath": fixture_folder.to_string_lossy()
        }),
    );

    let update = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "classes.updateFromAttachedLegacy",
        json!({
            "classId": class_id,
            "mode": "upsert_preserve",
            "collisionPolicy": "merge_existing",
            "preserveLocalValidity": true
        }),
    );
    assert_eq!(update.get("ok").and_then(|v| v.as_bool()), Some(true));
    assert!(
        update
            .pointer("/students/updated")
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
            > 0
    );

    let verify_membership = request_ok(
        &mut stdin,
        &mut reader,
        "8",
        "students.membership.get",
        json!({ "classId": class_id }),
    );
    let mat1_sort_order = verify_membership
        .get("markSets")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.iter().find(|m| m.get("id").and_then(|v| v.as_str()) == Some(mat1_id.as_str())))
        .and_then(|m| m.get("sortOrder"))
        .and_then(|v| v.as_i64())
        .expect("MAT1 sortOrder");
    let student_mask = verify_membership
        .get("students")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.iter().find(|s| s.get("id").and_then(|v| v.as_str()) == Some(target_student_id.as_str())))
        .and_then(|s| s.get("mask"))
        .and_then(|v| v.as_str())
        .expect("student mask");
    let bit = student_mask
        .chars()
        .nth(mat1_sort_order as usize)
        .unwrap_or('1');
    assert_eq!(bit, '0', "membership override should remain disabled");
}

