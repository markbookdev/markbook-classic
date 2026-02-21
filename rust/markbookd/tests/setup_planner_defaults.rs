mod test_support;

use serde_json::json;
use test_support::{request_ok, spawn_sidecar, temp_dir};

#[test]
fn setup_planner_course_description_and_reports_defaults_persist() {
    let workspace = temp_dir("markbook-setup-planner-defaults");
    let (_child, mut stdin, mut reader) = spawn_sidecar();

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "2",
        "setup.update",
        json!({
            "section": "planner",
            "patch": {
                "defaultLessonDurationMinutes": 90,
                "defaultPublishStatus": "published",
                "showArchivedByDefault": true,
                "defaultUnitTitlePrefix": "Module"
            }
        }),
    );
    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "3",
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
    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "setup.update",
        json!({
            "section": "reports",
            "patch": {
                "plannerHeaderStyle": "compact",
                "showGeneratedAt": false,
                "defaultStudentScope": "active"
            }
        }),
    );

    let setup = request_ok(&mut stdin, &mut reader, "5", "setup.get", json!({}));
    assert_eq!(
        setup
            .pointer("/planner/defaultLessonDurationMinutes")
            .and_then(|v| v.as_i64()),
        Some(90)
    );
    assert_eq!(
        setup
            .pointer("/planner/defaultPublishStatus")
            .and_then(|v| v.as_str()),
        Some("published")
    );
    assert_eq!(
        setup
            .pointer("/planner/defaultUnitTitlePrefix")
            .and_then(|v| v.as_str()),
        Some("Module")
    );
    assert_eq!(
        setup
            .pointer("/courseDescription/defaultPeriodMinutes")
            .and_then(|v| v.as_i64()),
        Some(80)
    );
    assert_eq!(
        setup
            .pointer("/courseDescription/defaultTotalWeeks")
            .and_then(|v| v.as_i64()),
        Some(40)
    );
    assert_eq!(
        setup
            .pointer("/reports/plannerHeaderStyle")
            .and_then(|v| v.as_str()),
        Some("compact")
    );
    assert_eq!(
        setup
            .pointer("/reports/defaultStudentScope")
            .and_then(|v| v.as_str()),
        Some("active")
    );
}
