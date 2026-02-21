mod test_support;

use serde_json::json;
use test_support::{fixture_path, request_ok, spawn_sidecar, temp_dir};

#[test]
fn analytics_student_trend_returns_sorted_points_and_membership_flags() {
    let workspace = temp_dir("markbook-analytics-student-trend");
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
    let first_mark_set_id = marksets
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
        json!({ "classId": class_id.clone(), "markSetId": first_mark_set_id.clone() }),
    );
    let student_id = open
        .get("students")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("id"))
        .and_then(|v| v.as_str())
        .expect("studentId")
        .to_string();

    let trend = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "analytics.student.trend",
        json!({
            "classId": class_id.clone(),
            "studentId": student_id.clone(),
            "filters": { "term": serde_json::Value::Null, "categoryName": serde_json::Value::Null, "typesMask": serde_json::Value::Null }
        }),
    );
    let points = trend
        .get("points")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(!points.is_empty(), "trend points should not be empty");
    for window in points.windows(2) {
        let a = window[0]
            .get("sortOrder")
            .and_then(|v| v.as_i64())
            .unwrap_or(i64::MAX);
        let b = window[1]
            .get("sortOrder")
            .and_then(|v| v.as_i64())
            .unwrap_or(i64::MAX);
        assert!(a <= b, "trend points must be sorted by sortOrder");
    }

    let first_point_mark_set_id = points
        .first()
        .and_then(|p| p.get("markSetId"))
        .and_then(|v| v.as_str())
        .expect("point markSetId")
        .to_string();
    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "students.membership.set",
        json!({
            "classId": class_id.clone(),
            "studentId": student_id.clone(),
            "markSetId": first_point_mark_set_id,
            "enabled": false
        }),
    );
    let after_disable = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "analytics.student.trend",
        json!({
            "classId": class_id,
            "studentId": student_id,
            "filters": { "term": serde_json::Value::Null, "categoryName": serde_json::Value::Null, "typesMask": serde_json::Value::Null }
        }),
    );
    let invalid_points = after_disable
        .get("points")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        invalid_points.iter().any(|p| {
            p.get("validForSet").and_then(|v| v.as_bool()) == Some(false)
                && p.get("finalMark").map(|v| v.is_null()).unwrap_or(false)
        }),
        "expected at least one invalid point with null finalMark after disabling membership"
    );
}
