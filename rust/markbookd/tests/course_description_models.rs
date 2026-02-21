mod test_support;

use serde_json::json;
use test_support::{request_ok, spawn_sidecar, temp_dir};

#[test]
fn course_description_profile_and_models_roundtrip() {
    let workspace = temp_dir("markbook-course-description");
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
        json!({ "name": "Course Profile Class" }),
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
        "planner.units.create",
        json!({ "classId": class_id, "input": { "title": "Unit A" } }),
    );
    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "planner.lessons.create",
        json!({
            "classId": class_id,
            "input": {
                "title": "Lesson A",
                "durationMinutes": 80
            }
        }),
    );

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "courseDescription.updateProfile",
        json!({
            "classId": class_id,
            "patch": {
                "courseTitle": "Applied Math",
                "gradeLabel": "Grade 9",
                "periodMinutes": 80,
                "periodsPerWeek": 4,
                "totalWeeks": 38,
                "strands": ["Knowledge", "Application"],
                "policyText": "Attendance required"
            }
        }),
    );

    let profile = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "courseDescription.getProfile",
        json!({ "classId": class_id }),
    );
    assert_eq!(
        profile
            .get("profile")
            .and_then(|v| v.get("courseTitle"))
            .and_then(|v| v.as_str()),
        Some("Applied Math")
    );

    let generated = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "courseDescription.generateModel",
        json!({ "classId": class_id, "options": { "includePolicy": true } }),
    );
    assert_eq!(
        generated
            .pointer("/profile/courseTitle")
            .and_then(|v| v.as_str()),
        Some("Applied Math")
    );
    assert!(
        generated
            .get("units")
            .and_then(|v| v.as_array())
            .map(|arr| !arr.is_empty())
            .unwrap_or(false)
    );

    let time_model = request_ok(
        &mut stdin,
        &mut reader,
        "8",
        "courseDescription.timeManagementModel",
        json!({ "classId": class_id }),
    );
    assert!(
        time_model
            .pointer("/totals/availableMinutes")
            .and_then(|v| v.as_i64())
            .unwrap_or(0)
            > 0
    );
}
