mod test_support;

use serde_json::json;
use test_support::{fixture_path, request_ok, spawn_sidecar, temp_dir};

#[test]
fn analytics_class_open_respects_filters_and_scope() {
    let workspace = temp_dir("markbook-analytics-class-open");
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

    let all = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "analytics.class.open",
        json!({
            "classId": class_id.clone(),
            "markSetId": mark_set_id.clone(),
            "filters": { "term": serde_json::Value::Null, "categoryName": serde_json::Value::Null, "typesMask": serde_json::Value::Null },
            "studentScope": "all"
        }),
    );
    let valid = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "analytics.class.open",
        json!({
            "classId": class_id,
            "markSetId": mark_set_id,
            "filters": { "term": 1, "categoryName": serde_json::Value::Null, "typesMask": serde_json::Value::Null },
            "studentScope": "valid"
        }),
    );

    assert_eq!(
        valid
            .get("filters")
            .and_then(|f| f.get("term"))
            .and_then(|v| v.as_i64()),
        Some(1)
    );
    assert_eq!(
        valid.get("studentScope").and_then(|v| v.as_str()),
        Some("valid")
    );

    let all_rows = all
        .get("rows")
        .and_then(|v| v.as_array())
        .map(|v| v.len())
        .unwrap_or(0);
    let valid_rows = valid
        .get("rows")
        .and_then(|v| v.as_array())
        .map(|v| v.len())
        .unwrap_or(0);
    assert!(valid_rows <= all_rows);

    let bins = valid
        .get("distributions")
        .and_then(|d| d.get("bins"))
        .and_then(|v| v.as_array())
        .map(|v| v.len())
        .unwrap_or(0);
    assert_eq!(bins, 6);
}
