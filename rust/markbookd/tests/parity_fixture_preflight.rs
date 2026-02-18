use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

fn fixture_path(rel: &str) -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    base.join("../../").join(rel)
}

fn read_manifest(path: &Path) -> Value {
    let text = fs::read_to_string(path).unwrap_or_else(|e| {
        panic!(
            "failed to read parity manifest at {}: {}",
            path.to_string_lossy(),
            e
        )
    });
    serde_json::from_str(&text).unwrap_or_else(|e| {
        panic!(
            "failed to parse parity manifest JSON at {}: {}",
            path.to_string_lossy(),
            e
        )
    })
}

fn required_list(manifest: &Value, key: &str) -> Vec<String> {
    manifest
        .get("required")
        .and_then(|v| v.get(key))
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| panic!("manifest missing required.{}", key))
        .iter()
        .map(|v| {
            v.as_str()
                .unwrap_or_else(|| panic!("required.{} entries must be strings", key))
                .to_string()
        })
        .collect()
}

fn missing_files(base: &Path, rels: &[String]) -> Vec<String> {
    rels.iter()
        .filter(|rel| !base.join(rel).is_file())
        .cloned()
        .collect()
}

#[test]
fn parity_fixture_preflight() {
    let expected_dir = fixture_path("fixtures/legacy/Sample25/expected");
    let manifest_path = expected_dir.join("parity-manifest.json");
    let manifest = read_manifest(&manifest_path);

    let regression_required = required_list(&manifest, "regression");
    let strict_required = required_list(&manifest, "strict");

    let missing_regression = missing_files(&expected_dir, &regression_required);
    assert!(
        missing_regression.is_empty(),
        "regression parity fixture files missing under {}:\n{}",
        expected_dir.to_string_lossy(),
        missing_regression.join("\n")
    );

    let strict_mode = std::env::var("MBC_STRICT_FRESH_SUMMARIES")
        .ok()
        .as_deref()
        == Some("1");
    let missing_strict = missing_files(&expected_dir, &strict_required);

    if strict_mode {
        assert!(
            missing_strict.is_empty(),
            "strict parity fixture files missing under {}:\n{}\n\nSet MBC_STRICT_FRESH_SUMMARIES=0 or add the files listed in parity-manifest.json.",
            expected_dir.to_string_lossy(),
            missing_strict.join("\n")
        );
    } else if !missing_strict.is_empty() {
        eprintln!(
            "parity preflight: strict files missing (non-strict mode, continuing):\n{}",
            missing_strict.join("\n")
        );
    }
}

