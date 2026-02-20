mod test_support;

use rusqlite::Connection;
use serde_json::json;
use std::path::{Path, PathBuf};
use test_support::{request_ok, spawn_sidecar, temp_dir};

fn fixture_path(rel: &str) -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    base.join("../../").join(rel)
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> bool {
    let sql = format!("PRAGMA table_info({})", table);
    let mut stmt = conn.prepare(&sql).expect("prepare pragma");
    let mut rows = stmt.query([]).expect("query pragma");
    while let Some(row) = rows.next().expect("next row") {
        let name: String = row.get(1).expect("column name");
        if name == column {
            return true;
        }
    }
    false
}

fn copy_snapshot_to_workspace(snapshot_db_path: &Path) -> PathBuf {
    let workspace = temp_dir("markbook-classmeta-migration");
    std::fs::copy(snapshot_db_path, workspace.join("markbook.sqlite3")).expect("copy snapshot");
    workspace
}

#[test]
fn class_meta_import_link_columns_exist_after_open_db_migration() {
    let snapshot_db = fixture_path("rust/markbookd/tests/fixtures/db/v2/markbook.sqlite3");
    let workspace = copy_snapshot_to_workspace(&snapshot_db);
    let (_child, mut stdin, mut reader) = spawn_sidecar();

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );

    let conn = Connection::open(workspace.join("markbook.sqlite3")).expect("open db");
    assert!(table_has_column(&conn, "class_meta", "legacy_folder_path"));
    assert!(table_has_column(&conn, "class_meta", "legacy_cl_file"));
    assert!(table_has_column(&conn, "class_meta", "legacy_year_token"));
    assert!(table_has_column(&conn, "class_meta", "last_imported_at"));
}
