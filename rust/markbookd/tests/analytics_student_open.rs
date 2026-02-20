mod test_support;

use serde_json::json;
use test_support::{fixture_path, request_ok, spawn_sidecar, temp_dir};

#[test]
fn analytics_student_open_returns_breakdown_and_tracks_membership_validity() {
    let workspace = temp_dir("markbook-analytics-student-open");
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

    let open = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "markset.open",
        json!({ "classId": class_id.clone(), "markSetId": mark_set_id.clone() }),
    );
    let student_id = open
        .get("students")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("id"))
        .and_then(|v| v.as_str())
        .expect("studentId")
        .to_string();

    let baseline = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "analytics.student.open",
        json!({
            "classId": class_id.clone(),
            "markSetId": mark_set_id.clone(),
            "studentId": student_id.clone(),
            "filters": { "term": serde_json::Value::Null, "categoryName": serde_json::Value::Null, "typesMask": serde_json::Value::Null }
        }),
    );
    assert!(
        baseline
            .get("assessmentTrail")
            .and_then(|v| v.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false),
        "assessmentTrail should not be empty"
    );

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "students.membership.set",
        json!({
            "classId": class_id.clone(),
            "studentId": student_id.clone(),
            "markSetId": mark_set_id.clone(),
            "enabled": false
        }),
    );

    let after_disable = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "analytics.student.open",
        json!({
            "classId": class_id,
            "markSetId": mark_set_id,
            "studentId": student_id,
            "filters": { "term": serde_json::Value::Null, "categoryName": serde_json::Value::Null, "typesMask": serde_json::Value::Null }
        }),
    );

    assert!(
        after_disable.get("finalMark").map(|v| v.is_null()).unwrap_or(false),
        "finalMark should become null when student membership is disabled for this mark set"
    );
}
