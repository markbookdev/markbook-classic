mod test_support;

use serde_json::json;
use test_support::{fixture_path, request_ok, spawn_sidecar, temp_dir};

#[test]
fn comments_transfer_preview_returns_match_counts_and_rows() {
    let workspace = temp_dir("markbook-comments-transfer-preview");
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

    let sets = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "comments.sets.list",
        json!({ "classId": class_id.clone(), "markSetId": mark_set_id.clone() }),
    );
    let set_number = sets
        .get("sets")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.get("setNumber"))
        .and_then(|v| v.as_i64())
        .unwrap_or(1);

    if sets
        .get("sets")
        .and_then(|v| v.as_array())
        .map(|arr| arr.is_empty())
        .unwrap_or(true)
    {
        let _ = request_ok(
            &mut stdin,
            &mut reader,
            "5",
            "comments.sets.upsert",
            json!({
                "classId": class_id.clone(),
                "markSetId": mark_set_id.clone(),
                "setNumber": 1,
                "title": "Auto Set",
                "fitMode": 0,
                "fitFontSize": 9,
                "fitWidth": 83,
                "fitLines": 12,
                "fitSubj": "",
                "maxChars": 100,
                "isDefault": true,
                "remarksByStudent": []
            }),
        );
    }

    let preview = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "comments.transfer.preview",
        json!({
            "sourceClassId": class_id.clone(),
            "sourceMarkSetId": mark_set_id.clone(),
            "sourceSetNumber": set_number,
            "targetClassId": class_id.clone(),
            "targetMarkSetId": mark_set_id.clone(),
            "targetSetNumber": set_number,
            "studentMatchMode": "student_no_then_name"
        }),
    );

    let counts = preview.get("counts").expect("counts");
    let source_rows = counts
        .get("sourceRows")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let matched = counts.get("matched").and_then(|v| v.as_u64()).unwrap_or(0);
    assert!(source_rows > 0, "source rows should be non-zero");
    assert!(matched > 0, "matched should be non-zero");
    assert!(
        preview
            .get("rows")
            .and_then(|v| v.as_array())
            .map(|arr| !arr.is_empty())
            .unwrap_or(false),
        "preview rows should not be empty"
    );
}

