mod test_support;

use serde_json::json;
use test_support::{fixture_path, request_ok, spawn_sidecar, temp_dir};

#[test]
fn comments_flood_fill_updates_requested_targets_only() {
    let workspace = temp_dir("markbook-comments-flood-fill");
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
    let source_id = students
        .first()
        .and_then(|s| s.get("id"))
        .and_then(|v| v.as_str())
        .expect("source")
        .to_string();
    let target_a = students
        .get(1)
        .and_then(|s| s.get("id"))
        .and_then(|v| v.as_str())
        .expect("target_a")
        .to_string();
    let target_b = students
        .get(2)
        .and_then(|s| s.get("id"))
        .and_then(|v| v.as_str())
        .expect("target_b")
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
            "title": "Flood Fill",
            "fitMode": 0,
            "fitFontSize": 9,
            "fitWidth": 83,
            "fitLines": 12,
            "fitSubj": "",
            "maxChars": 100,
            "isDefault": true,
            "remarksByStudent": [
                { "studentId": source_id, "remark": "Seed Remark" },
                { "studentId": target_a, "remark": "" },
                { "studentId": target_b, "remark": "" }
            ]
        }),
    );

    let flood = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "comments.transfer.floodFill",
        json!({
            "classId": class_id.clone(),
            "markSetId": mark_set_id.clone(),
            "setNumber": 1,
            "sourceStudentId": source_id,
            "targetStudentIds": [target_a],
            "policy": "replace"
        }),
    );
    assert_eq!(
        flood.get("updated").and_then(|v| v.as_u64()).unwrap_or(0),
        1
    );

    let open = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "comments.sets.open",
        json!({ "classId": class_id, "markSetId": mark_set_id, "setNumber": 1 }),
    );
    let rows = open
        .get("remarksByStudent")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let a_remark = rows
        .iter()
        .find(|r| r.get("studentId").and_then(|v| v.as_str()) == Some(target_a.as_str()))
        .and_then(|r| r.get("remark"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let b_remark = rows
        .iter()
        .find(|r| r.get("studentId").and_then(|v| v.as_str()) == Some(target_b.as_str()))
        .and_then(|r| r.get("remark"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    assert_eq!(a_remark, "Seed Remark");
    assert_eq!(b_remark, "");
}

