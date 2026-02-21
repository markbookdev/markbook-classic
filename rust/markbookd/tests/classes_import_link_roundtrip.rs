mod test_support;

use serde_json::json;
use test_support::{request_ok, spawn_sidecar, temp_dir};

#[test]
fn classes_import_link_set_get_roundtrip() {
    let workspace = temp_dir("markbook-class-import-link");
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
        json!({ "name": "ImportLink Class" }),
    );
    let class_id = created
        .get("classId")
        .and_then(|v| v.as_str())
        .expect("classId")
        .to_string();

    let initial = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "classes.importLink.get",
        json!({ "classId": class_id }),
    );
    assert!(
        initial
            .get("legacyClassFolderPath")
            .map(|v| v.is_null())
            .unwrap_or(false),
        "initial import link should be null"
    );

    let attached_path = workspace.join("legacy-folder");
    let _ = std::fs::create_dir_all(&attached_path);
    let attached_path = attached_path.to_string_lossy().to_string();

    let set_res = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "classes.importLink.set",
        json!({
            "classId": class_id,
            "legacyClassFolderPath": attached_path
        }),
    );
    assert_eq!(set_res.get("ok").and_then(|v| v.as_bool()), Some(true));

    let linked = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "classes.importLink.get",
        json!({ "classId": class_id }),
    );
    assert_eq!(
        linked
            .get("legacyClassFolderPath")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        Some(attached_path.clone())
    );

    let meta = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "classes.meta.get",
        json!({ "classId": class_id }),
    );
    assert_eq!(
        meta.pointer("/meta/legacyFolderPath")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        Some(attached_path)
    );
}

