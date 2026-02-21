mod test_support;

use serde_json::json;
use test_support::{fixture_path, request_ok, spawn_sidecar, temp_dir};

#[test]
fn analytics_drilldown_and_report_model_align() {
    let workspace = temp_dir("markbook-analytics-drilldown-reports-align");
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
            "filters": { "term": 1, "categoryName": serde_json::Value::Null, "typesMask": serde_json::Value::Null },
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

    let params = json!({
        "classId": class_id,
        "markSetId": mark_set_id,
        "assessmentId": assessment_id,
        "filters": { "term": 1, "categoryName": serde_json::Value::Null, "typesMask": serde_json::Value::Null },
        "studentScope": "valid",
        "query": { "sortBy": "percent", "sortDir": "desc", "page": 1, "pageSize": 25 }
    });

    let analytics = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "analytics.class.assessmentDrilldown",
        params.clone(),
    );
    let report = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "reports.classAssessmentDrilldownModel",
        params,
    );

    assert_eq!(
        analytics.get("totalRows").and_then(|v| v.as_u64()),
        report.get("totalRows").and_then(|v| v.as_u64())
    );
    assert_eq!(
        analytics
            .get("classStats")
            .and_then(|v| v.get("avgPercent"))
            .and_then(|v| v.as_f64()),
        report
            .get("classStats")
            .and_then(|v| v.get("avgPercent"))
            .and_then(|v| v.as_f64())
    );
    assert_eq!(
        analytics
            .get("rows")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("studentId"))
            .and_then(|v| v.as_str()),
        report
            .get("rows")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("studentId"))
            .and_then(|v| v.as_str())
    );
}
