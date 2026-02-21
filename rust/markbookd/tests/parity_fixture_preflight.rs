use serde_json::Value;
use sha2::{Digest, Sha256};
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

fn checksum_map(manifest: &Value) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    let Some(obj) = manifest.get("checksums").and_then(|v| v.as_object()) else {
        return out;
    };
    for (k, v) in obj {
        let Some(expected) = v.as_str() else {
            continue;
        };
        let key = k.trim();
        let val = expected.trim().to_ascii_lowercase();
        if key.is_empty() || val.is_empty() {
            continue;
        }
        out.insert(key.to_string(), val);
    }
    out
}

fn file_sha256(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|e| {
        panic!("failed reading {} for checksum: {}", path.to_string_lossy(), e)
    });
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn checksum_mismatches(
    base: &Path,
    rels: &[String],
    checksums: &std::collections::HashMap<String, String>,
) -> Vec<String> {
    let mut out = Vec::new();
    for rel in rels {
        let Some(expected) = checksums.get(rel.as_str()) else {
            continue;
        };
        let abs = base.join(rel);
        if !abs.is_file() {
            continue;
        }
        let actual = file_sha256(&abs);
        if actual != *expected {
            out.push(format!(
                "{} expected={} actual={}",
                rel, expected, actual
            ));
        }
    }
    out
}

#[test]
fn parity_fixture_preflight() {
    const EXPECTED_MANIFEST_VERSION: i64 = 1;
    let expected_dir = fixture_path("fixtures/legacy/Sample25/expected");
    let manifest_path = expected_dir.join("parity-manifest.json");
    let manifest = read_manifest(&manifest_path);
    let manifest_version = manifest
        .get("version")
        .and_then(|v| v.as_i64())
        .unwrap_or_default();
    assert_eq!(
        manifest_version, EXPECTED_MANIFEST_VERSION,
        "parity manifest schema version mismatch at {}: expected {}, got {}",
        manifest_path.to_string_lossy(), EXPECTED_MANIFEST_VERSION, manifest_version
    );

    let regression_required = required_list(&manifest, "regression");
    let strict_required = required_list(&manifest, "strict");
    let checksums = checksum_map(&manifest);

    let missing_regression = missing_files(&expected_dir, &regression_required);
    assert!(
        missing_regression.is_empty(),
        "regression parity fixture files missing under {}:\n{}",
        expected_dir.to_string_lossy(),
        missing_regression.join("\n")
    );
    let regression_checksum_mismatches =
        checksum_mismatches(&expected_dir, &regression_required, &checksums);
    assert!(
        regression_checksum_mismatches.is_empty(),
        "regression parity checksum mismatch under {}:\n{}",
        expected_dir.to_string_lossy(),
        regression_checksum_mismatches.join("\n")
    );

    let strict_mode = std::env::var("MBC_STRICT_FRESH_SUMMARIES")
        .ok()
        .as_deref()
        == Some("1");
    let missing_strict = missing_files(&expected_dir, &strict_required);
    let strict_checksum_mismatches =
        checksum_mismatches(&expected_dir, &strict_required, &checksums);

    if strict_mode {
        assert!(
            missing_strict.is_empty(),
            "strict parity fixture files missing under {}:\n{}\n\nSet MBC_STRICT_FRESH_SUMMARIES=0 or add the files listed in parity-manifest.json.",
            expected_dir.to_string_lossy(),
            missing_strict.join("\n")
        );
        assert!(
            strict_checksum_mismatches.is_empty(),
            "strict parity checksum mismatch under {}:\n{}\n\nUpdate files or parity-manifest.json checksums.",
            expected_dir.to_string_lossy(),
            strict_checksum_mismatches.join("\n")
        );
    } else if !missing_strict.is_empty() {
        eprintln!(
            "parity preflight: strict files missing (non-strict mode, continuing):\n{}",
            missing_strict.join("\n")
        );
        if !strict_checksum_mismatches.is_empty() {
            eprintln!(
                "parity preflight: strict checksum mismatches (non-strict mode, continuing):\n{}",
                strict_checksum_mismatches.join("\n")
            );
        }
    } else if !strict_checksum_mismatches.is_empty() {
        eprintln!(
            "parity preflight: strict checksum mismatches (non-strict mode, continuing):\n{}",
            strict_checksum_mismatches.join("\n")
        );
    }
}
