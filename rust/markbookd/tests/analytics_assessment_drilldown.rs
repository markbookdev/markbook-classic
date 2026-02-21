mod test_support;

use serde_json::json;
use test_support::{fixture_path, request_ok, spawn_sidecar, temp_dir};

#[test]
fn analytics_assessment_drilldown_returns_rows_with_class_stats() {
    let workspace = temp_dir("markbook-analytics-assessment-drilldown");
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

    let class_open = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "analytics.class.open",
        json!({
            "classId": class_id.clone(),
            "markSetId": mark_set_id.clone(),
            "filters": { "term": serde_json::Value::Null, "categoryName": serde_json::Value::Null, "typesMask": serde_json::Value::Null },
            "studentScope": "valid"
        }),
    );
    let assessment_id = class_open
        .get("perAssessment")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("assessmentId"))
        .and_then(|v| v.as_str())
        .expect("assessmentId")
        .to_string();

    let drilldown = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "analytics.class.assessmentDrilldown",
        json!({
            "classId": class_id,
            "markSetId": mark_set_id,
            "assessmentId": assessment_id,
            "filters": { "term": serde_json::Value::Null, "categoryName": serde_json::Value::Null, "typesMask": serde_json::Value::Null },
            "studentScope": "valid",
            "query": { "sortBy": "sortOrder", "sortDir": "asc", "page": 1, "pageSize": 50 }
        }),
    );

    let total_rows = drilldown
        .get("totalRows")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let rows = drilldown
        .get("rows")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        !rows.is_empty() || total_rows == 0,
        "rows should be non-empty unless totalRows is zero"
    );

    for row in &rows {
        let status = row
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        assert!(
            matches!(status, "no_mark" | "zero" | "scored"),
            "unexpected status {}",
            status
        );
    }

    let class_stats = drilldown.get("classStats").expect("classStats");
    assert!(class_stats.get("assessmentId").is_some());
    assert!(class_stats.get("avgPercent").is_some());
}
