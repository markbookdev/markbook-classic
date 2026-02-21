mod test_support;

use serde_json::json;
use test_support::{fixture_path, request_ok, spawn_sidecar, temp_dir};

#[test]
fn analytics_class_rows_supports_search_sort_paging_and_cohorts() {
    let workspace = temp_dir("markbook-analytics-class-rows");
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

    let base = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "analytics.class.rows",
        json!({
            "classId": class_id.clone(),
            "markSetId": mark_set_id.clone(),
            "filters": { "term": serde_json::Value::Null, "categoryName": serde_json::Value::Null, "typesMask": serde_json::Value::Null },
            "studentScope": "valid",
            "query": { "sortBy": "sortOrder", "sortDir": "asc", "page": 1, "pageSize": 10 }
        }),
    );
    assert_eq!(base.get("page").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(base.get("pageSize").and_then(|v| v.as_u64()), Some(10));
    let total_rows = base.get("totalRows").and_then(|v| v.as_u64()).unwrap_or(0);
    assert!(total_rows > 0);
    let rows = base
        .get("rows")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(!rows.is_empty(), "expected at least one row on first page");

    let searched = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "analytics.class.rows",
        json!({
            "classId": class_id.clone(),
            "markSetId": mark_set_id.clone(),
            "filters": { "term": serde_json::Value::Null, "categoryName": serde_json::Value::Null, "typesMask": serde_json::Value::Null },
            "studentScope": "valid",
            "query": {
                "search": "boame",
                "sortBy": "displayName",
                "sortDir": "asc",
                "page": 1,
                "pageSize": 50
            }
        }),
    );
    let searched_rows = searched
        .get("rows")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        searched_rows.iter().all(|r| {
            r.get("displayName")
                .and_then(|v| v.as_str())
                .map(|s| s.to_ascii_lowercase().contains("boame"))
                .unwrap_or(false)
        }),
        "search should constrain returned rows"
    );

    let cohort = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "analytics.class.rows",
        json!({
            "classId": class_id,
            "markSetId": mark_set_id,
            "filters": { "term": serde_json::Value::Null, "categoryName": serde_json::Value::Null, "typesMask": serde_json::Value::Null },
            "studentScope": "valid",
            "query": {
                "sortBy": "finalMark",
                "sortDir": "desc",
                "page": 1,
                "pageSize": 100,
                "cohort": { "finalMin": 70.0, "finalMax": 100.0, "includeNoFinal": false }
            }
        }),
    );
    let cohort_total = cohort
        .get("totalRows")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(
        cohort_total <= total_rows,
        "cohort filter must not increase row count"
    );
    assert_eq!(
        cohort
            .get("appliedCohort")
            .and_then(|v| v.get("finalMin"))
            .and_then(|v| v.as_f64()),
        Some(70.0)
    );
}
