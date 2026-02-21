mod test_support;

use serde_json::json;
use test_support::{request_ok, spawn_sidecar, temp_dir};

#[test]
fn planner_units_crud_reorder_archive_roundtrip() {
    let workspace = temp_dir("markbook-planner-units");
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
        json!({ "name": "Planner Unit Class" }),
    );
    let class_id = created
        .get("classId")
        .and_then(|v| v.as_str())
        .expect("classId")
        .to_string();

    let u1 = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "planner.units.create",
        json!({ "classId": class_id, "input": { "title": "Unit A" } }),
    );
    let u1_id = u1
        .get("unitId")
        .and_then(|v| v.as_str())
        .expect("unitId")
        .to_string();

    let u2 = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "planner.units.create",
        json!({ "classId": class_id, "input": { "title": "Unit B" } }),
    );
    let u2_id = u2
        .get("unitId")
        .and_then(|v| v.as_str())
        .expect("unitId")
        .to_string();

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "planner.units.update",
        json!({
            "classId": class_id,
            "unitId": u1_id,
            "patch": { "title": "Unit A Updated", "summary": "Summary" }
        }),
    );

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "planner.units.reorder",
        json!({ "classId": class_id, "unitIds": [u2_id, u1_id] }),
    );

    let list = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "planner.units.list",
        json!({ "classId": class_id, "includeArchived": false }),
    );
    let units = list
        .get("units")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert_eq!(units.len(), 2);
    assert_eq!(
        units[0].get("id").and_then(|v| v.as_str()),
        Some(u2_id.as_str())
    );
    assert_eq!(
        units[1].get("title").and_then(|v| v.as_str()),
        Some("Unit A Updated")
    );

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "8",
        "planner.units.archive",
        json!({ "classId": class_id, "unitId": u2_id, "archived": true }),
    );

    let visible = request_ok(
        &mut stdin,
        &mut reader,
        "9",
        "planner.units.list",
        json!({ "classId": class_id, "includeArchived": false }),
    );
    assert_eq!(
        visible
            .get("units")
            .and_then(|v| v.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0),
        1
    );

    let all = request_ok(
        &mut stdin,
        &mut reader,
        "10",
        "planner.units.list",
        json!({ "classId": class_id, "includeArchived": true }),
    );
    assert_eq!(
        all.get("units")
            .and_then(|v| v.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0),
        2
    );
}
