mod test_support;

use rusqlite::Connection;
use serde_json::json;
use std::path::PathBuf;
use test_support::{fixture_path, request_ok, spawn_sidecar, temp_dir};

fn workspace_db_path(workspace: &std::path::Path) -> PathBuf {
    workspace.join("markbook.sqlite3")
}

#[test]
fn analytics_combined_falls_back_to_equal_weight_when_selected_weights_are_zero() {
    let workspace = temp_dir("markbook-analytics-combined-fallback");
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
        .take(2)
        .filter_map(|m| m.get("id").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect::<Vec<_>>();
    assert_eq!(selected_ids.len(), 2);

    // Force selected set weights to 0 to trigger equal-weight fallback.
    let conn = Connection::open(workspace_db_path(&workspace)).expect("open workspace db");
    for id in &selected_ids {
        conn.execute("UPDATE mark_sets SET weight = 0 WHERE id = ?", [id.as_str()])
            .expect("set mark set weight to zero");
    }

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

    let fallback_used_count = combined
        .get("settingsApplied")
        .and_then(|v| v.get("fallbackUsedCount"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    assert!(fallback_used_count > 0, "expected equal-weight fallback to be used");

    let rows = combined
        .get("rows")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for row in rows {
        let combined_final = row.get("combinedFinal").and_then(|v| v.as_f64());
        let marks = row
            .get("perMarkSet")
            .and_then(|v| v.as_array())
            .into_iter()
            .flatten()
            .filter_map(|x| x.get("finalMark").and_then(|v| v.as_f64()))
            .collect::<Vec<_>>();
        if marks.is_empty() {
            assert!(combined_final.is_none());
            continue;
        }
        let expected_raw = marks.iter().sum::<f64>() / (marks.len() as f64);
        let expected = (expected_raw * 10.0).round() / 10.0;
        let actual = combined_final.expect("combined final for available marks");
        assert!(
            (actual - expected).abs() <= 0.05,
            "expected equal-weight fallback average, actual={} expected={}",
            actual,
            expected
        );
    }
}
