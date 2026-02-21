mod test_support;

use serde_json::json;
use test_support::{request_ok, spawn_sidecar, temp_dir};

#[test]
fn planner_lessons_create_reorder_archive_and_filter() {
    let workspace = temp_dir("markbook-planner-lessons");
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
        json!({ "name": "Planner Lesson Class" }),
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
        json!({ "classId": class_id, "input": { "title": "Unit 1" } }),
    );
    let unit_id = unit
        .get("unitId")
        .and_then(|v| v.as_str())
        .expect("unitId")
        .to_string();

    let l1 = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "planner.lessons.create",
        json!({
            "classId": class_id,
            "input": {
                "unitId": unit_id,
                "title": "Lesson 1",
                "lessonDate": "2026-02-01",
                "durationMinutes": 70
            }
        }),
    );
    let l1_id = l1
        .get("lessonId")
        .and_then(|v| v.as_str())
        .expect("lessonId")
        .to_string();

    let l2 = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "planner.lessons.create",
        json!({
            "classId": class_id,
            "input": {
                "unitId": unit_id,
                "title": "Lesson 2",
                "lessonDate": "2026-02-02",
                "durationMinutes": 75
            }
        }),
    );
    let l2_id = l2
        .get("lessonId")
        .and_then(|v| v.as_str())
        .expect("lessonId")
        .to_string();

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "planner.lessons.reorder",
        json!({ "classId": class_id, "unitId": unit_id, "lessonIdOrder": [l2_id, l1_id] }),
    );

    let filtered = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "planner.lessons.list",
        json!({ "classId": class_id, "unitId": unit_id, "includeArchived": false }),
    );
    let lessons = filtered
        .get("lessons")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    assert_eq!(lessons.len(), 2);
    assert_eq!(
        lessons[0].get("title").and_then(|v| v.as_str()),
        Some("Lesson 2")
    );

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "8",
        "planner.lessons.update",
        json!({
            "classId": class_id,
            "lessonId": l2_id,
            "patch": { "title": "Lesson 2 Updated", "homework": "Read chapter 1" }
        }),
    );

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "9",
        "planner.lessons.archive",
        json!({ "classId": class_id, "lessonId": l1_id, "archived": true }),
    );

    let visible = request_ok(
        &mut stdin,
        &mut reader,
        "10",
        "planner.lessons.list",
        json!({ "classId": class_id, "includeArchived": false }),
    );
    assert_eq!(
        visible
            .get("lessons")
            .and_then(|v| v.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0),
        1
    );

    let archived = request_ok(
        &mut stdin,
        &mut reader,
        "11",
        "planner.lessons.list",
        json!({ "classId": class_id, "includeArchived": true }),
    );
    assert_eq!(
        archived
            .get("lessons")
            .and_then(|v| v.as_array())
            .map(|arr| arr.len())
            .unwrap_or(0),
        2
    );
}
