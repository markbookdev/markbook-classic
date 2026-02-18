#[path = "../src/legacy.rs"]
mod legacy;

use std::path::PathBuf;

fn fixture_path(rel: &str) -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    base.join("../../").join(rel)
}

#[test]
fn parse_mat18d_13_export_file() {
    let p = fixture_path("fixtures/legacy/Sample25/MB8D25/MAT18D.13");
    let parsed = legacy::parse_legacy_export_file(&p).expect("parse MAT18D.13");
    assert_eq!(parsed.last_student, 27);
    assert!(parsed.blocks.len() >= 2);
    assert_eq!(parsed.blocks[0].title.trim(), "Group Report");
    assert!((parsed.blocks[0].out_of - 100.0).abs() < 1e-9);
    assert!(
        (27..=28).contains(&parsed.blocks[0].values.len()),
        "unexpected MAT18D.13 block value count: {}",
        parsed.blocks[0].values.len()
    );
}

#[test]
fn parse_snc28d_15_export_file() {
    let p = fixture_path("fixtures/legacy/Sample25/MB8D25/SNC28D.15");
    let parsed = legacy::parse_legacy_export_file(&p).expect("parse SNC28D.15");
    assert_eq!(parsed.last_student, 27);
    assert!(parsed.blocks.len() >= 3);
    assert_eq!(parsed.blocks[0].title.trim(), "True / False");
    assert!((parsed.blocks[0].out_of - 9.0).abs() < 1e-9);
    assert!(
        (27..=28).contains(&parsed.blocks[0].values.len()),
        "unexpected SNC28D.15 block value count: {}",
        parsed.blocks[0].values.len()
    );
}

#[test]
fn parse_mat28d_32_and_40_have_same_block_count() {
    let p32 = fixture_path("fixtures/legacy/Sample25/MB8D25/MAT28D.32");
    let p40 = fixture_path("fixtures/legacy/Sample25/MB8D25/MAT28D.40");
    let a = legacy::parse_legacy_export_file(&p32).expect("parse MAT28D.32");
    let b = legacy::parse_legacy_export_file(&p40).expect("parse MAT28D.40");
    assert_eq!(a.last_student, b.last_student);
    assert_eq!(a.blocks.len(), b.blocks.len());
}
