mod test_support;

use serde_json::json;
use test_support::{request_ok, spawn_sidecar, temp_dir};

#[test]
fn planner_publish_preview_commit_and_status_changes() {
    let workspace = temp_dir("markbook-planner-publish");
    let (_child, mut stdin, mut reader) = spawn_sidecar();

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );
    let created = request_ok(
        &mut stdin,
        &mut reader,
        "2",
        "classes.create",
        json!({ "name": "Planner Publish Class" }),
    );
    let class_id = created
        .get("classId")
        .and_then(|v| v.as_str())
        .expect("classId")
        .to_string();

    let unit = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "planner.units.create",
        json!({ "classId": class_id, "input": { "title": "Unit P" } }),
    );
    let unit_id = unit
        .get("unitId")
        .and_then(|v| v.as_str())
        .expect("unitId")
        .to_string();

    let preview = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "planner.publish.preview",
        json!({
            "classId": class_id,
            "artifactKind": "unit",
            "sourceId": unit_id
        }),
    );
    assert_eq!(
        preview.get("artifactKind").and_then(|v| v.as_str()),
        Some("unit")
    );

    let commit = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "planner.publish.commit",
        json!({
            "classId": class_id,
            "artifactKind": "unit",
            "sourceId": unit_id,
            "title": "Unit P Publish",
            "model": preview.get("model").cloned().unwrap_or_else(|| json!({})),
            "status": "draft"
        }),
    );
    let publish_id = commit
        .get("publishId")
        .and_then(|v| v.as_str())
        .expect("publishId")
        .to_string();
    assert_eq!(commit.get("status").and_then(|v| v.as_str()), Some("draft"));

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "planner.publish.updateStatus",
        json!({
            "classId": class_id,
            "publishId": publish_id,
            "status": "published"
        }),
    );

    let listed = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "planner.publish.list",
        json!({ "classId": class_id, "artifactKind": "unit" }),
    );
    assert_eq!(
        listed
            .get("published")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.get("status"))
            .and_then(|v| v.as_str()),
        Some("published")
    );
}
