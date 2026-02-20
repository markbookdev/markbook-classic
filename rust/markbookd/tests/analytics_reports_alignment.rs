mod test_support;

use serde_json::json;
use test_support::{fixture_path, request_ok, spawn_sidecar, temp_dir};

#[test]
fn analytics_and_reports_align_for_same_filter_scope_inputs() {
    let workspace = temp_dir("markbook-analytics-reports-alignment");
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

    let filters = json!({
        "term": 1,
        "categoryName": serde_json::Value::Null,
        "typesMask": serde_json::Value::Null
    });

    let class_analytics = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "analytics.class.open",
        json!({
            "classId": class_id.clone(),
            "markSetId": mark_set_id.clone(),
            "filters": filters,
            "studentScope": "valid"
        }),
    );
    let report_summary = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "reports.markSetSummaryModel",
        json!({
            "classId": class_id.clone(),
            "markSetId": mark_set_id.clone(),
            "filters": filters,
            "studentScope": "valid"
        }),
    );

    let analytics_avg = class_analytics
        .get("kpis")
        .and_then(|v| v.get("classAverage"))
        .and_then(|v| v.as_f64());
    let report_avg = {
        let marks: Vec<f64> = report_summary
            .get("perStudent")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter_map(|s| s.get("finalMark").and_then(|v| v.as_f64()))
            .collect();
        if marks.is_empty() {
            None
        } else {
            Some(marks.iter().sum::<f64>() / (marks.len() as f64))
        }
    };

    match (analytics_avg, report_avg) {
        (None, None) => {}
        (Some(a), Some(b)) => assert!(
            (a - b).abs() <= 0.05,
            "class average drift: analytics={} reports={}",
            a,
            b
        ),
        other => panic!("unexpected average pair: {:?}", other),
    }

    let student_id = report_summary
        .get("perStudent")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("studentId"))
        .and_then(|v| v.as_str())
        .expect("studentId")
        .to_string();
    let student_analytics = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "analytics.student.open",
        json!({
            "classId": class_id.clone(),
            "markSetId": mark_set_id.clone(),
            "studentId": student_id.clone(),
            "filters": filters
        }),
    );
    let report_student = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "reports.studentSummaryModel",
        json!({
            "classId": class_id,
            "markSetId": mark_set_id,
            "studentId": student_id,
            "filters": filters,
            "studentScope": "valid"
        }),
    );

    let analytics_final = student_analytics
        .get("finalMark")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let report_final = report_student
        .get("student")
        .and_then(|v| v.get("finalMark"))
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    assert_eq!(analytics_final, report_final);
}
