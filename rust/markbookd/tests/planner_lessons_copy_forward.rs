mod test_support;

use serde_json::json;
use test_support::{request_ok, spawn_sidecar, temp_dir};

#[test]
fn planner_lessons_copy_forward_and_bulk_assign_unit() {
    let workspace = temp_dir("markbook-planner-copy-forward");
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
        json!({ "name": "Planner Copy Class" }),
    );
    let class_id = created
        .get("classId")
        .and_then(|v| v.as_str())
        .expect("classId")
        .to_string();

    let source_unit = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "planner.units.create",
        json!({ "classId": class_id, "input": { "title": "Unit A" } }),
    );
    let source_unit_id = source_unit
        .get("unitId")
        .and_then(|v| v.as_str())
        .expect("source unit id")
        .to_string();
    let target_unit = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "planner.units.create",
        json!({ "classId": class_id, "input": { "title": "Unit B" } }),
    );
    let target_unit_id = target_unit
        .get("unitId")
        .and_then(|v| v.as_str())
        .expect("target unit id")
        .to_string();

    let lesson = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "planner.lessons.create",
        json!({
            "classId": class_id,
            "input": {
                "unitId": source_unit_id,
                "title": "Lesson Source",
                "lessonDate": "2026-03-10",
                "followUp": "Bring worksheet",
                "homework": "Page 11",
                "durationMinutes": 60
            }
        }),
    );
    let lesson_id = lesson
        .get("lessonId")
        .and_then(|v| v.as_str())
        .expect("lesson id")
        .to_string();

    let copied = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "planner.lessons.copyForward",
        json!({
            "classId": class_id,
            "lessonIds": [lesson_id],
            "dayOffset": 2,
            "includeFollowUp": false,
            "includeHomework": true
        }),
    );
    let copied_lesson_id = copied
        .get("createdLessonIds")
        .and_then(|v| v.as_array())
        .and_then(|rows| rows.first())
        .and_then(|v| v.as_str())
        .expect("created lesson id")
        .to_string();

    let copied_open = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "planner.lessons.open",
        json!({ "classId": class_id, "lessonId": copied_lesson_id }),
    );
    assert_eq!(
        copied_open
            .pointer("/lesson/lessonDate")
            .and_then(|v| v.as_str()),
        Some("2026-03-12")
    );
    assert_eq!(
        copied_open
            .pointer("/lesson/followUp")
            .and_then(|v| v.as_str()),
        Some("")
    );
    assert_eq!(
        copied_open
            .pointer("/lesson/homework")
            .and_then(|v| v.as_str()),
        Some("Page 11")
    );

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "8",
        "planner.lessons.bulkAssignUnit",
        json!({
            "classId": class_id,
            "lessonIds": [copied_lesson_id],
            "unitId": target_unit_id
        }),
    );
    let moved = request_ok(
        &mut stdin,
        &mut reader,
        "9",
        "planner.lessons.list",
        json!({ "classId": class_id, "unitId": target_unit_id, "includeArchived": true }),
    );
    let moved_count = moved
        .get("lessons")
        .and_then(|v| v.as_array())
        .map(|rows| rows.len())
        .unwrap_or(0);
    assert_eq!(moved_count, 1);
}
