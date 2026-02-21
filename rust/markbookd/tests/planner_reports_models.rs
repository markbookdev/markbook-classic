mod test_support;

use serde_json::json;
use test_support::{request_ok, spawn_sidecar, temp_dir};

#[test]
fn planner_report_models_are_available() {
    let workspace = temp_dir("markbook-planner-reports-models");
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
        json!({ "name": "Planner Reports Class" }),
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
        json!({ "classId": class_id, "input": { "title": "Unit R" } }),
    );
    let unit_id = unit
        .get("unitId")
        .and_then(|v| v.as_str())
        .expect("unitId")
        .to_string();

    let lesson = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "planner.lessons.create",
        json!({
            "classId": class_id,
            "input": {
                "unitId": unit_id,
                "title": "Lesson R",
                "durationMinutes": 70
            }
        }),
    );
    let lesson_id = lesson
        .get("lessonId")
        .and_then(|v| v.as_str())
        .expect("lessonId")
        .to_string();

    let unit_model = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "reports.plannerUnitModel",
        json!({ "classId": class_id, "unitId": unit_id }),
    );
    assert_eq!(
        unit_model
            .get("artifactKind")
            .and_then(|v| v.as_str()),
        Some("unit")
    );

    let lesson_model = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "reports.plannerLessonModel",
        json!({ "classId": class_id, "lessonId": lesson_id }),
    );
    assert_eq!(
        lesson_model
            .get("artifactKind")
            .and_then(|v| v.as_str()),
        Some("lesson")
    );

    let course_model = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "reports.courseDescriptionModel",
        json!({ "classId": class_id }),
    );
    assert!(course_model.get("profile").is_some());

    let time_model = request_ok(
        &mut stdin,
        &mut reader,
        "8",
        "reports.timeManagementModel",
        json!({ "classId": class_id }),
    );
    assert!(time_model.get("totals").is_some());
}
