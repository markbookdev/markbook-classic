#[path = "../src/backup.rs"]
mod backup;

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_dir(prefix: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "{}-{}",
        prefix,
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&p).expect("create temp dir");
    p
}

#[test]
fn zip_export_and_import_roundtrip() {
    let workspace = temp_dir("markbook-backup-src");
    let workspace2 = temp_dir("markbook-backup-dst");
    let out_dir = temp_dir("markbook-backup-out");

    let db_src = workspace.join("markbook.sqlite3");
    let bytes = b"sqlite-test-payload";
    std::fs::write(&db_src, bytes).expect("write source db");

    let bundle_path = out_dir.join("workspace.mbcbackup.zip");
    let export = backup::export_workspace_bundle(&workspace, &bundle_path).expect("export bundle");
    assert_eq!(export.bundle_format, backup::BUNDLE_FORMAT_V2);
    assert_eq!(export.entry_count, 3);

    let f = File::open(&bundle_path).expect("open bundle");
    let mut archive = zip::ZipArchive::new(f).expect("open zip archive");
    let mut manifest = String::new();
    archive
        .by_name("manifest.json")
        .expect("manifest entry")
        .read_to_string(&mut manifest)
        .expect("read manifest");
    assert!(manifest.contains(backup::BUNDLE_FORMAT_V2));
    archive
        .by_name("db/markbook.sqlite3")
        .expect("database entry in bundle");

    let import = backup::import_workspace_bundle(&bundle_path, &workspace2).expect("import bundle");
    assert_eq!(import.bundle_format_detected, backup::BUNDLE_FORMAT_V2);

    let db_dst = workspace2.join("markbook.sqlite3");
    let restored = std::fs::read(&db_dst).expect("read restored db");
    assert_eq!(restored, bytes);

    let _ = std::fs::remove_dir_all(workspace);
    let _ = std::fs::remove_dir_all(workspace2);
    let _ = std::fs::remove_dir_all(out_dir);
}

#[test]
fn legacy_sqlite_import_is_supported() {
    let out_dir = temp_dir("markbook-backup-legacy");
    let workspace = temp_dir("markbook-backup-legacy-dst");

    let legacy_file = out_dir.join("legacy.sqlite3");
    let bytes = b"legacy-sqlite-copy";
    std::fs::write(&legacy_file, bytes).expect("write legacy sqlite file");

    let import =
        backup::import_workspace_bundle(&legacy_file, &workspace).expect("import legacy sqlite");
    assert_eq!(import.bundle_format_detected, "legacy-sqlite3");

    let restored = std::fs::read(workspace.join("markbook.sqlite3")).expect("read restored sqlite");
    assert_eq!(restored, bytes);

    let _ = std::fs::remove_dir_all(out_dir);
    let _ = std::fs::remove_dir_all(workspace);
}
