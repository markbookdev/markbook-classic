mod test_support;

use serde_json::json;
use test_support::{fixture_path, request_ok, spawn_sidecar, temp_dir};

#[test]
fn integrations_sis_exports_write_roster_and_marks_csv() {
    let workspace = temp_dir("markbook-integrations-sis-exports");
    let legacy_class_folder = fixture_path("fixtures/legacy/Sample25/MB8D25");
    let out_dir = temp_dir("markbook-integrations-sis-exports-out");
    let roster_path = out_dir.join("sis-roster.csv");
    let marks_path = out_dir.join("sis-marks.csv");
    let (_child, mut stdin, mut reader) = spawn_sidecar();

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );
    let imported = request_ok(
        &mut stdin,
        &mut reader,
        "2",
        "class.importLegacy",
        json!({ "legacyClassFolderPath": legacy_class_folder.to_string_lossy() }),
    );
    let class_id = imported["classId"].as_str().expect("class id").to_string();
    let marksets = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "marksets.list",
        json!({ "classId": class_id }),
    );
    let mark_set_id = marksets["markSets"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|m| m.get("id"))
        .and_then(|v| v.as_str())
        .expect("mark set id")
        .to_string();

    let roster_res = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "integrations.sis.exportRoster",
        json!({
            "classId": class_id,
            "outPath": roster_path.to_string_lossy(),
            "profile": "sis_roster_v1",
            "studentScope": "active"
        }),
    );
    assert!(
        roster_res["rowsExported"].as_i64().unwrap_or(0) > 0,
        "expected exported roster rows: {}",
        roster_res
    );
    let roster_text = std::fs::read_to_string(&roster_path).expect("read roster export");
    assert!(roster_text.starts_with("student_id,student_no,last_name,first_name"));

    let marks_res = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "integrations.sis.exportMarks",
        json!({
            "classId": class_id,
            "markSetId": mark_set_id,
            "outPath": marks_path.to_string_lossy(),
            "profile": "sis_marks_v1",
            "studentScope": "valid",
            "filters": { "term": 1, "categoryName": null, "typesMask": null },
            "includeStateColumns": true
        }),
    );
    assert!(
        marks_res["rowsExported"].as_i64().unwrap_or(0) > 0,
        "expected exported marks rows: {}",
        marks_res
    );
    assert!(
        marks_res["assessmentsExported"].as_i64().unwrap_or(0) > 0,
        "expected exported assessments: {}",
        marks_res
    );
    let marks_text = std::fs::read_to_string(&marks_path).expect("read marks export");
    assert!(
        marks_text
            .starts_with("student_id,student_no,last_name,first_name,mark_set_code,assessment_idx"),
        "unexpected marks header: {}",
        marks_text.lines().next().unwrap_or_default()
    );
    assert!(marks_text.contains(",status,"));

    let summary = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "reports.markSetSummaryModel",
        json!({
            "classId": class_id,
            "markSetId": mark_set_id,
            "filters": { "term": 1, "categoryName": null, "typesMask": null },
            "studentScope": "valid"
        }),
    );
    let scoped_students = summary["perStudent"]
        .as_array()
        .map(|arr| arr.len() as i64)
        .unwrap_or(0);
    let expected_rows = scoped_students * marks_res["assessmentsExported"].as_i64().unwrap_or(0);
    assert_eq!(
        marks_res["rowsExported"].as_i64().unwrap_or(0),
        expected_rows,
        "SIS marks export row count should align with reports model student scope/filter set"
    );
}
