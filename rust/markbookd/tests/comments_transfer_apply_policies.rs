mod test_support;

use serde_json::json;
use test_support::{fixture_path, request_ok, spawn_sidecar, temp_dir};

#[test]
fn comments_transfer_apply_respects_fill_blank_and_source_if_longer_policies() {
    let workspace = temp_dir("markbook-comments-transfer-apply");
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
        .expect("student1")
        .to_string();
    let s2 = students
        .get(1)
        .and_then(|s| s.get("id"))
        .and_then(|v| v.as_str())
        .expect("student2")
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
            "remarksByStudent": [
                { "studentId": s1, "remark": "From source one" },
                { "studentId": s2, "remark": "Source remark that is much longer" }
            ]
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
            "fitMode": 0,
            "fitFontSize": 9,
            "fitWidth": 83,
            "fitLines": 12,
            "fitSubj": "",
            "maxChars": 100,
            "isDefault": false,
            "remarksByStudent": [
                { "studentId": s1, "remark": "" },
                { "studentId": s2, "remark": "short" }
            ]
        }),
    );

    let fill_blank = request_ok(
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
            "policy": "fill_blank"
        }),
    );
    assert!(
        fill_blank
            .get("updated")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            >= 1
    );

    let source_if_longer = request_ok(
        &mut stdin,
        &mut reader,
        "8",
        "comments.transfer.apply",
        json!({
            "sourceClassId": class_id.clone(),
            "sourceMarkSetId": mark_set_id.clone(),
            "sourceSetNumber": 1,
            "targetClassId": class_id.clone(),
            "targetMarkSetId": mark_set_id.clone(),
            "targetSetNumber": 2,
            "policy": "source_if_longer"
        }),
    );
    assert!(
        source_if_longer
            .get("updated")
            .and_then(|v| v.as_u64())
            .unwrap_or(0)
            >= 1
    );

    let target_open = request_ok(
        &mut stdin,
        &mut reader,
        "9",
        "comments.sets.open",
        json!({ "classId": class_id, "markSetId": mark_set_id, "setNumber": 2 }),
    );
    let by_student = target_open
        .get("remarksByStudent")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let r1 = by_student
        .iter()
        .find(|r| r.get("studentId").and_then(|v| v.as_str()) == Some(s1.as_str()))
        .and_then(|r| r.get("remark"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let r2 = by_student
        .iter()
        .find(|r| r.get("studentId").and_then(|v| v.as_str()) == Some(s2.as_str()))
        .and_then(|r| r.get("remark"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert_eq!(r1, "From source one");
    assert_eq!(r2, "Source remark that is much longer");
}

