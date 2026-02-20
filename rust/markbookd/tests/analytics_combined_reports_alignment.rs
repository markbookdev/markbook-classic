mod test_support;

use serde_json::json;
use test_support::{fixture_path, request_ok, spawn_sidecar, temp_dir};

#[test]
fn combined_analytics_and_report_model_align_for_same_inputs() {
    let workspace = temp_dir("markbook-analytics-combined-alignment");
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
    let options = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "analytics.combined.options",
        json!({ "classId": class_id.clone() }),
    );
    let selected_ids = options
        .get("markSets")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .take(3)
        .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect::<Vec<_>>();
    assert!(
        selected_ids.len() >= 2,
        "need at least two mark sets for combined alignment"
    );

    let params = json!({
        "classId": class_id,
        "markSetIds": selected_ids,
        "filters": {
            "term": 1,
            "categoryName": serde_json::Value::Null,
            "typesMask": serde_json::Value::Null
        },
        "studentScope": "valid"
    });
    let analytics = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "analytics.combined.open",
        params.clone(),
    );
    let report = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "reports.combinedAnalysisModel",
        params,
    );

    let analytics_kpis = analytics.get("kpis").cloned().unwrap_or_else(|| json!({}));
    let report_kpis = report.get("kpis").cloned().unwrap_or_else(|| json!({}));
    assert_eq!(
        analytics_kpis.get("studentCount"),
        report_kpis.get("studentCount")
    );
    assert_eq!(
        analytics_kpis.get("finalMarkCount"),
        report_kpis.get("finalMarkCount")
    );
    assert_eq!(
        analytics_kpis.get("noCombinedFinalCount"),
        report_kpis.get("noCombinedFinalCount")
    );

    let analytics_avg = analytics_kpis.get("classAverage").and_then(|v| v.as_f64());
    let report_avg = report_kpis.get("classAverage").and_then(|v| v.as_f64());
    match (analytics_avg, report_avg) {
        (None, None) => {}
        (Some(a), Some(b)) => assert!((a - b).abs() <= 0.05),
        other => panic!("unexpected combined average pair: {:?}", other),
    }

    let analytics_rows = analytics
        .get("rows")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let report_rows = report
        .get("rows")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert_eq!(analytics_rows.len(), report_rows.len());
}

