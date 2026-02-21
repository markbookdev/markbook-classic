mod test_support;

use serde_json::json;
use test_support::{fixture_path, request_ok, spawn_sidecar, temp_dir};

#[test]
fn analytics_student_compare_reflects_scope_changes() {
    let workspace = temp_dir("markbook-analytics-student-compare");
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

    let all_scope = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "analytics.student.compare",
        json!({
            "classId": class_id.clone(),
            "markSetId": mark_set_id.clone(),
            "studentId": student_id.clone(),
            "filters": { "term": serde_json::Value::Null, "categoryName": serde_json::Value::Null, "typesMask": serde_json::Value::Null },
            "studentScope": "all"
        }),
    );
    let valid_scope = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "analytics.student.compare",
        json!({
            "classId": class_id,
            "markSetId": mark_set_id,
            "studentId": student_id,
            "filters": { "term": serde_json::Value::Null, "categoryName": serde_json::Value::Null, "typesMask": serde_json::Value::Null },
            "studentScope": "valid"
        }),
    );

    let all_count = all_scope
        .get("cohort")
        .and_then(|v| v.get("studentCount"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let valid_count = valid_scope
        .get("cohort")
        .and_then(|v| v.get("studentCount"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(valid_count <= all_count);
    assert_eq!(
        valid_scope.get("studentScope").and_then(|v| v.as_str()),
        Some("valid")
    );
    assert!(
        valid_scope
            .get("assessmentComparison")
            .and_then(|v| v.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false),
        "assessmentComparison should not be empty"
    );
}
