mod test_support;

use serde_json::json;
use test_support::{fixture_path, request_ok, spawn_sidecar, temp_dir};

#[test]
fn setup_sections_marks_exchange_analytics_persist_and_marks_default_applies() {
    let workspace = temp_dir("markbook-setup-defaults-extended");
    let fixture_folder = fixture_path("fixtures/legacy/Sample25/MB8D25");
    let (_child, mut stdin, mut reader) = spawn_sidecar();

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "2",
        "setup.update",
        json!({
            "section": "marks",
            "patch": {
                "defaultHideDeletedEntries": false,
                "defaultAutoPreviewBeforeBulkApply": false
            }
        }),
    );
    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "setup.update",
        json!({
            "section": "exchange",
            "patch": {
                "defaultExportStudentScope": "active",
                "includeStateColumnsByDefault": false
            }
        }),
    );
    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "setup.update",
        json!({
            "section": "analytics",
            "patch": {
                "defaultPageSize": 60,
                "defaultCohortMode": "threshold"
            }
        }),
    );

    let setup = request_ok(&mut stdin, &mut reader, "5", "setup.get", json!({}));
    assert_eq!(
        setup.pointer("/marks/defaultHideDeletedEntries")
            .and_then(|v| v.as_bool()),
        Some(false)
    );
    assert_eq!(
        setup.pointer("/marks/defaultAutoPreviewBeforeBulkApply")
            .and_then(|v| v.as_bool()),
        Some(false)
    );
    assert_eq!(
        setup.pointer("/exchange/defaultExportStudentScope")
            .and_then(|v| v.as_str()),
        Some("active")
    );
    assert_eq!(
        setup.pointer("/exchange/includeStateColumnsByDefault")
            .and_then(|v| v.as_bool()),
        Some(false)
    );
    assert_eq!(
        setup.pointer("/analytics/defaultPageSize")
            .and_then(|v| v.as_i64()),
        Some(60)
    );
    assert_eq!(
        setup.pointer("/analytics/defaultCohortMode")
            .and_then(|v| v.as_str()),
        Some("threshold")
    );

    let imported = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "class.importLegacy",
        json!({ "legacyClassFolderPath": fixture_folder.to_string_lossy() }),
    );
    let class_id = imported
        .get("classId")
        .and_then(|v| v.as_str())
        .expect("class id");

    let marksets = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "marksets.list",
        json!({ "classId": class_id }),
    );
    let mark_set_id = marksets
        .get("markSets")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("id"))
        .and_then(|v| v.as_str())
        .expect("mark set id");

    // No per-mark-set preference was written yet, so marks pref should use setup.marks default.
    let hide_pref = request_ok(
        &mut stdin,
        &mut reader,
        "8",
        "marks.pref.hideDeleted.get",
        json!({ "classId": class_id, "markSetId": mark_set_id }),
    );
    assert_eq!(
        hide_pref.get("hideDeleted").and_then(|v| v.as_bool()),
        Some(false)
    );
}

