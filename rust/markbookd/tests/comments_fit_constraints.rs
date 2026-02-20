mod test_support;

use serde_json::json;
use test_support::{fixture_path, request_ok, spawn_sidecar, temp_dir};

#[test]
fn comments_transfer_apply_enforces_max_chars_and_fit_limits() {
    let workspace = temp_dir("markbook-comments-fit-constraints");
    let fixture_folder = fixture_path("fixtures/legacy/Sample25/MB8D25");
    let (_child, mut stdin, mut reader) = spawn_sidecar();

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );
    let import = request_ok(
        &mut stdin,
        &mut reader,
        "2",
        "class.importLegacy",
        json!({ "legacyClassFolderPath": fixture_folder.to_string_lossy() }),
    );
    let class_id = import
        .get("classId")
        .and_then(|v| v.as_str())
        .expect("classId")
        .to_string();
    let marksets = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "marksets.list",
        json!({ "classId": class_id.clone() }),
    );
    let mark_set_id = marksets
        .get("markSets")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("id"))
        .and_then(|v| v.as_str())
        .expect("markSetId")
        .to_string();
    let students = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "students.list",
        json!({ "classId": class_id.clone() }),
    )
    .get("students")
    .and_then(|v| v.as_array())
    .cloned()
    .unwrap_or_default();
    let s1 = students
        .first()
        .and_then(|s| s.get("id"))
        .and_then(|v| v.as_str())
        .expect("student")
        .to_string();

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "comments.sets.upsert",
        json!({
            "classId": class_id.clone(),
            "markSetId": mark_set_id.clone(),
            "setNumber": 1,
            "title": "Source",
            "fitMode": 0,
            "fitFontSize": 9,
            "fitWidth": 83,
            "fitLines": 12,
            "fitSubj": "",
            "maxChars": 100,
            "isDefault": true,
            "remarksByStudent": [{ "studentId": s1, "remark": "ABCDEFGHIJKL" }]
        }),
    );
    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "comments.sets.upsert",
        json!({
            "classId": class_id.clone(),
            "markSetId": mark_set_id.clone(),
            "setNumber": 2,
            "title": "Target",
            "fitMode": 1,
            "fitFontSize": 9,
            "fitWidth": 3,
            "fitLines": 2,
            "fitSubj": "",
            "maxChars": 10,
            "isDefault": false,
            "remarksByStudent": [{ "studentId": s1, "remark": "" }]
        }),
    );

    let applied = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "comments.transfer.apply",
        json!({
            "sourceClassId": class_id.clone(),
            "sourceMarkSetId": mark_set_id.clone(),
            "sourceSetNumber": 1,
            "targetClassId": class_id.clone(),
            "targetMarkSetId": mark_set_id.clone(),
            "targetSetNumber": 2,
            "policy": "replace",
            "targetScope": "selected_target_students",
            "selectedTargetStudentIds": [s1]
        }),
    );
    assert_eq!(applied.get("updated").and_then(|v| v.as_u64()).unwrap_or(0), 1);
    assert!(
        applied
            .get("warnings")
            .and_then(|v| v.as_array())
            .map(|arr| !arr.is_empty())
            .unwrap_or(false),
        "expected truncation warning"
    );

    let target = request_ok(
        &mut stdin,
        &mut reader,
        "8",
        "comments.sets.open",
        json!({ "classId": class_id, "markSetId": mark_set_id, "setNumber": 2 }),
    );
    let remark = target
        .get("remarksByStudent")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .find(|r| r.get("studentId").and_then(|v| v.as_str()) == Some(s1.as_str()))
        .and_then(|r| r.get("remark"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert_eq!(remark, "ABCDEF");
}
