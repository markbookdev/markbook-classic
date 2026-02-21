mod test_support;

use serde_json::json;
use test_support::{request_ok, spawn_sidecar, temp_dir};

#[test]
fn planner_units_clone_duplicates_unit_and_lessons() {
    let workspace = temp_dir("markbook-planner-unit-clone");
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
        json!({ "name": "Planner Clone Class" }),
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
        json!({
            "classId": class_id,
            "input": {
                "title": "Unit Clone Source",
                "resources": ["Textbook", "Lab Kit"]
            }
        }),
    );
    let source_unit_id = source_unit
        .get("unitId")
        .and_then(|v| v.as_str())
        .expect("unitId")
        .to_string();

    for (idx, title) in ["Lesson A", "Lesson B"].iter().enumerate() {
        let _ = request_ok(
            &mut stdin,
            &mut reader,
            &format!("4-{}", idx),
            "planner.lessons.create",
            json!({
                "classId": class_id,
                "input": {
                    "unitId": source_unit_id,
                    "title": title,
                    "lessonDate": if idx == 0 { "2026-03-01" } else { "2026-03-02" },
                    "durationMinutes": 70
                }
            }),
        );
    }

    let cloned = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "planner.units.clone",
        json!({
            "classId": class_id,
            "unitId": source_unit_id,
            "titleMode": "appendCopy"
        }),
    );
    let cloned_unit_id = cloned
        .get("unitId")
        .and_then(|v| v.as_str())
        .expect("cloned unitId")
        .to_string();

    let units = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "planner.units.list",
        json!({ "classId": class_id, "includeArchived": true }),
    );
    let cloned_row = units
        .get("units")
        .and_then(|v| v.as_array())
        .and_then(|rows| {
            rows.iter()
                .find(|row| row.get("id").and_then(|v| v.as_str()) == Some(cloned_unit_id.as_str()))
        })
        .cloned()
        .expect("cloned unit row");
    assert_eq!(
        cloned_row.get("title").and_then(|v| v.as_str()),
        Some("Unit Clone Source (Copy)")
    );

    let source_lessons = request_ok(
        &mut stdin,
        &mut reader,
        "7",
        "planner.lessons.list",
        json!({ "classId": class_id, "unitId": source_unit_id, "includeArchived": true }),
    );
    let cloned_lessons = request_ok(
        &mut stdin,
        &mut reader,
        "8",
        "planner.lessons.list",
        json!({ "classId": class_id, "unitId": cloned_unit_id, "includeArchived": true }),
    );
    let source_count = source_lessons
        .get("lessons")
        .and_then(|v| v.as_array())
        .map(|rows| rows.len())
        .unwrap_or(0);
    let cloned_count = cloned_lessons
        .get("lessons")
        .and_then(|v| v.as_array())
        .map(|rows| rows.len())
        .unwrap_or(0);
    assert_eq!(source_count, 2);
    assert_eq!(cloned_count, source_count);
}
