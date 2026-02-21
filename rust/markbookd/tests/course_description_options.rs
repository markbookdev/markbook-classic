mod test_support;

use serde_json::json;
use test_support::{request_ok, spawn_sidecar, temp_dir};

#[test]
fn course_description_generate_model_respects_include_options() {
    let workspace = temp_dir("markbook-course-options");
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
        json!({ "name": "Course Options Class" }),
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
        "courseDescription.updateProfile",
        json!({
            "classId": class_id,
            "patch": {
                "courseTitle": "History",
                "gradeLabel": "Grade 10",
                "strands": ["Inquiry", "Communication"],
                "policyText": "Submit all work."
            }
        }),
    );
    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "planner.units.create",
        json!({
            "classId": class_id,
            "input": {
                "title": "Unit Resources",
                "resources": ["Textbook", "Projector"]
            }
        }),
    );

    let excluded = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "courseDescription.generateModel",
        json!({
            "classId": class_id,
            "options": {
                "includePolicy": false,
                "includeStrands": false,
                "includeAssessmentPlan": false,
                "includeResources": false
            }
        }),
    );
    assert_eq!(
        excluded
            .pointer("/profile/policyText")
            .and_then(|v| v.as_str()),
        Some("")
    );
    assert_eq!(
        excluded
            .pointer("/profile/strands")
            .and_then(|v| v.as_array())
            .map(|rows| rows.len()),
        Some(0)
    );
    assert!(
        excluded.get("assessmentPlan").is_none(),
        "assessmentPlan should be omitted when includeAssessmentPlan=false"
    );
    assert_eq!(
        excluded
            .pointer("/resources")
            .and_then(|v| v.as_array())
            .map(|rows| rows.len()),
        Some(0)
    );

    let included = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "courseDescription.generateModel",
        json!({
            "classId": class_id,
            "options": {
                "includePolicy": true,
                "includeStrands": true,
                "includeAssessmentPlan": true,
                "includeResources": true
            }
        }),
    );
    assert_eq!(
        included
            .pointer("/profile/policyText")
            .and_then(|v| v.as_str()),
        Some("Submit all work.")
    );
    assert!(
        included
            .pointer("/profile/strands")
            .and_then(|v| v.as_array())
            .map(|rows| !rows.is_empty())
            .unwrap_or(false)
    );
    assert!(
        included.get("assessmentPlan").is_some(),
        "assessmentPlan should exist when includeAssessmentPlan=true"
    );
    assert!(
        included
            .pointer("/resources")
            .and_then(|v| v.as_array())
            .map(|rows| !rows.is_empty())
            .unwrap_or(false)
    );
}
