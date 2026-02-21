mod test_support;

use serde_json::json;
use test_support::{request_ok, spawn_sidecar, temp_dir};

#[test]
fn planner_and_course_models_apply_setup_defaults_server_side() {
    let workspace = temp_dir("markbook-planner-settings-applied");
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
        json!({ "name": "Planner Defaults Class" }),
    );
    let class_id = created
        .get("classId")
        .and_then(|v| v.as_str())
        .expect("classId")
        .to_string();

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "setup.update",
        json!({
            "section": "planner",
            "patch": {
                "defaultLessonDurationMinutes": 95,
                "defaultPublishStatus": "published"
            }
        }),
    );
    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "setup.update",
        json!({
            "section": "courseDescription",
            "patch": {
                "defaultPeriodMinutes": 80,
                "defaultPeriodsPerWeek": 4,
                "defaultTotalWeeks": 40,
                "includePolicyByDefault": false
            }
        }),
    );

    let created_lesson = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "planner.lessons.create",
        json!({
            "classId": class_id,
            "input": {
                "title": "Lesson uses default duration"
            }
        }),
    );
    let lesson_id = created_lesson
        .get("lessonId")
        .and_then(|v| v.as_str())
        .expect("lessonId")
        .to_string();
    let lesson = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "planner.lessons.open",
        json!({ "classId": class_id, "lessonId": lesson_id }),
    );
    assert_eq!(
        lesson
            .pointer("/lesson/durationMinutes")
            .and_then(|v| v.as_i64()),
        Some(95)
    );

    let publish = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "planner.publish.commit",
        json!({
            "classId": class_id,
            "artifactKind": "course_description",
            "title": "Defaults Publish",
            "model": { "kind": "course_description" }
        }),
    );
    assert_eq!(publish.get("status").and_then(|v| v.as_str()), Some("published"));
    assert_eq!(
        publish
            .pointer("/settingsApplied/plannerDefaults/defaultPublishStatus")
            .and_then(|v| v.as_str()),
        Some("published")
    );

    let cd = request_ok(
        &mut stdin,
        &mut reader,
        "8",
        "courseDescription.generateModel",
        json!({
            "classId": class_id
        }),
    );
    assert_eq!(
        cd.pointer("/schedule/periodMinutes").and_then(|v| v.as_i64()),
        Some(80)
    );
    assert_eq!(
        cd.pointer("/schedule/periodsPerWeek").and_then(|v| v.as_i64()),
        Some(4)
    );
    assert_eq!(
        cd.pointer("/schedule/totalWeeks").and_then(|v| v.as_i64()),
        Some(40)
    );
    assert_eq!(
        cd.pointer("/settingsApplied/resolved/includePolicy")
            .and_then(|v| v.as_bool()),
        Some(false)
    );

    let tm = request_ok(
        &mut stdin,
        &mut reader,
        "9",
        "courseDescription.timeManagementModel",
        json!({
            "classId": class_id
        }),
    );
    assert_eq!(
        tm.pointer("/inputs/periodMinutes").and_then(|v| v.as_i64()),
        Some(80)
    );
    assert_eq!(
        tm.pointer("/inputs/periodsPerWeek").and_then(|v| v.as_i64()),
        Some(4)
    );
    assert_eq!(
        tm.pointer("/inputs/totalWeeks").and_then(|v| v.as_i64()),
        Some(40)
    );
}
