mod test_support;

use serde_json::json;
use test_support::{request, request_ok, spawn_sidecar, temp_dir};

#[test]
fn setup_depth_sections_persist_and_validate() {
    let workspace = temp_dir("markbook-setup-depth");
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
            "section": "attendance",
            "patch": {
                "presentCode": "PR",
                "absentCode": "AB",
                "lateCode": "LT",
                "schoolYearStartMonth": 8
            }
        }),
    );
    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "setup.update",
        json!({
            "section": "comments",
            "patch": {
                "defaultSetNumber": 2,
                "defaultAppendSeparator": " | ",
                "enforceMaxCharsByDefault": false
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
                "repeatHeadersByDefault": false,
                "defaultPageMargins": {
                    "topMm": 10,
                    "rightMm": 11,
                    "bottomMm": 12,
                    "leftMm": 13
                }
            }
        }),
    );
    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "setup.update",
        json!({
            "section": "security",
            "patch": {
                "requireWorkspacePassword": true
            }
        }),
    );
    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "6",
        "setup.update",
        json!({
            "section": "printer",
            "patch": {
                "defaultPaperSize": "a4",
                "defaultOrientation": "landscape"
            }
        }),
    );

    let setup = request_ok(&mut stdin, &mut reader, "7", "setup.get", json!({}));
    assert_eq!(
        setup.pointer("/attendance/presentCode").and_then(|v| v.as_str()),
        Some("PR")
    );
    assert_eq!(
        setup.pointer("/attendance/schoolYearStartMonth").and_then(|v| v.as_i64()),
        Some(8)
    );
    assert_eq!(
        setup.pointer("/comments/defaultSetNumber").and_then(|v| v.as_i64()),
        Some(2)
    );
    assert_eq!(
        setup
            .pointer("/comments/defaultAppendSeparator")
            .and_then(|v| v.as_str()),
        Some("|")
    );
    assert_eq!(
        setup
            .pointer("/comments/enforceMaxCharsByDefault")
            .and_then(|v| v.as_bool()),
        Some(false)
    );
    assert_eq!(
        setup
            .pointer("/reports/repeatHeadersByDefault")
            .and_then(|v| v.as_bool()),
        Some(false)
    );
    assert_eq!(
        setup
            .pointer("/reports/defaultPageMargins/leftMm")
            .and_then(|v| v.as_i64()),
        Some(13)
    );
    assert_eq!(
        setup
            .pointer("/security/requireWorkspacePassword")
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        setup
            .pointer("/security/passwordEnabled")
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        setup
            .pointer("/printer/defaultPaperSize")
            .and_then(|v| v.as_str()),
        Some("a4")
    );
    assert_eq!(
        setup
            .pointer("/printer/defaultOrientation")
            .and_then(|v| v.as_str()),
        Some("landscape")
    );

    let invalid = request(
        &mut stdin,
        &mut reader,
        "8",
        "setup.update",
        json!({
            "section": "printer",
            "patch": {
                "defaultPaperSize": "ledger"
            }
        }),
    );
    assert_eq!(invalid.get("ok").and_then(|v| v.as_bool()), Some(false));
    assert_eq!(
        invalid.pointer("/error/code").and_then(|v| v.as_str()),
        Some("bad_params")
    );
}
