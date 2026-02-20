mod test_support;

use serde_json::json;
use test_support::{fixture_path, request_ok, spawn_sidecar, temp_dir};

#[test]
fn analytics_combined_open_returns_weighted_rows_and_kpis() {
    let workspace = temp_dir("markbook-analytics-combined-open");
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
    let mark_sets = options
        .get("markSets")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        mark_sets.len() >= 2,
        "expected at least two mark sets for combined analytics"
    );
    let selected_ids = mark_sets
        .iter()
        .take(2)
        .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect::<Vec<_>>();
    assert_eq!(selected_ids.len(), 2, "selected ids");

    let combined = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "analytics.combined.open",
        json!({
            "classId": class_id,
            "markSetIds": selected_ids,
            "filters": {
                "term": serde_json::Value::Null,
                "categoryName": serde_json::Value::Null,
                "typesMask": serde_json::Value::Null
            },
            "studentScope": "valid"
        }),
    );

    let selected_len = combined
        .get("markSets")
        .and_then(|v| v.as_array())
        .map(|v| v.len())
        .unwrap_or(0);
    assert_eq!(selected_len, 2);

    let rows = combined
        .get("rows")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(!rows.is_empty(), "expected at least one combined row");

    let student_count = combined
        .get("kpis")
        .and_then(|v| v.get("studentCount"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;
    assert_eq!(student_count, rows.len());

    let per_mark_set = combined
        .get("perMarkSet")
        .and_then(|v| v.as_array())
        .map(|v| v.len())
        .unwrap_or(0);
    assert_eq!(per_mark_set, 2);

    let first_row_per_set = rows
        .first()
        .and_then(|r| r.get("perMarkSet"))
        .and_then(|v| v.as_array())
        .map(|v| v.len())
        .unwrap_or(0);
    assert_eq!(first_row_per_set, 2);
}

