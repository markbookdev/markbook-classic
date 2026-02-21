mod test_support;

use serde_json::json;
use test_support::{request_ok, spawn_sidecar, temp_dir};

#[test]
fn setup_integrations_defaults_persist() {
    let workspace = temp_dir("markbook-setup-integrations-defaults");
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
            "section": "integrations",
            "patch": {
                "defaultSisProfile": "mb_exchange_v1",
                "defaultMatchMode": "student_no_then_name",
                "defaultCollisionPolicy": "merge_existing",
                "autoPreviewBeforeApply": true,
                "adminTransferDefaultPolicy": "fill_blank"
            }
        }),
    );

    let setup = request_ok(&mut stdin, &mut reader, "3", "setup.get", json!({}));
    assert_eq!(
        setup.pointer("/integrations/defaultSisProfile").and_then(|v| v.as_str()),
        Some("mb_exchange_v1")
    );
    assert_eq!(
        setup.pointer("/integrations/defaultMatchMode").and_then(|v| v.as_str()),
        Some("student_no_then_name")
    );
    assert_eq!(
        setup
            .pointer("/integrations/defaultCollisionPolicy")
            .and_then(|v| v.as_str()),
        Some("merge_existing")
    );
    assert_eq!(
        setup
            .pointer("/integrations/autoPreviewBeforeApply")
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        setup
            .pointer("/integrations/adminTransferDefaultPolicy")
            .and_then(|v| v.as_str()),
        Some("fill_blank")
    );
}
