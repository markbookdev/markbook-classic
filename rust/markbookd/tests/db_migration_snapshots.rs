use rusqlite::Connection;
use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

fn fixture_path(rel: &str) -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    base.join("../../").join(rel)
}

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

fn copy_snapshot_to_workspace(snapshot_db_path: &Path) -> PathBuf {
    let workspace = temp_dir("markbook-db-migration-workspace");
    let db_path = workspace.join("markbook.sqlite3");
    std::fs::copy(snapshot_db_path, &db_path).expect("copy snapshot db");
    workspace
}

fn spawn_sidecar() -> (Child, ChildStdin, BufReader<ChildStdout>) {
    let exe = env!("CARGO_BIN_EXE_markbookd");
    let mut child = Command::new(exe)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn markbookd");
    let stdin = child.stdin.take().expect("child stdin");
    let stdout = child.stdout.take().expect("child stdout");
    (child, stdin, BufReader::new(stdout))
}

fn request(
    stdin: &mut ChildStdin,
    reader: &mut BufReader<ChildStdout>,
    id: &str,
    method: &str,
    params: serde_json::Value,
) -> serde_json::Value {
    let payload = json!({
        "id": id,
        "method": method,
        "params": params,
    });
    writeln!(stdin, "{}", payload).expect("write request");
    stdin.flush().expect("flush request");

    let mut line = String::new();
    reader.read_line(&mut line).expect("read response line");
    assert!(!line.trim().is_empty(), "empty response for {}", method);
    serde_json::from_str(line.trim()).expect("parse response json")
}

fn request_ok(
    stdin: &mut ChildStdin,
    reader: &mut BufReader<ChildStdout>,
    id: &str,
    method: &str,
    params: serde_json::Value,
) -> serde_json::Value {
    let value = request(stdin, reader, id, method, params);
    assert_eq!(value.get("id").and_then(|v| v.as_str()), Some(id));
    assert!(
        value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false),
        "{} failed: {}",
        method,
        value
    );
    value.get("result").cloned().unwrap_or_else(|| json!({}))
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> bool {
    let sql = format!("PRAGMA table_info({})", table);
    let mut stmt = conn.prepare(&sql).expect("prepare pragma table_info");
    let mut rows = stmt.query([]).expect("query pragma table_info");
    while let Some(row) = rows.next().expect("next row") {
        let name: String = row.get(1).expect("column name");
        if name == column {
            return true;
        }
    }
    false
}

#[test]
fn v0_snapshot_migrates_and_supports_legacy_import() {
    let snapshot_db = fixture_path("rust/markbookd/tests/fixtures/db/v0/markbook.sqlite3");
    let workspace = copy_snapshot_to_workspace(&snapshot_db);
    let fixture_folder = fixture_path("fixtures/legacy/Sample25/MB8D25");

    let (_child, mut stdin, mut reader) = spawn_sidecar();

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );

    let classes = request_ok(&mut stdin, &mut reader, "2", "classes.list", json!({}));
    assert!(
        classes
            .get("classes")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().any(|c| c.get("id").and_then(|v| v.as_str()) == Some("c_old_v0")))
            .unwrap_or(false),
        "expected legacy class from snapshot"
    );

    let conn = Connection::open(workspace.join("markbook.sqlite3")).expect("open migrated db");
    assert!(table_has_column(&conn, "students", "sort_order"));
    assert!(table_has_column(&conn, "students", "updated_at"));
    assert!(table_has_column(&conn, "students", "mark_set_mask"));
    assert!(table_has_column(&conn, "scores", "remark"));
    assert!(table_has_column(&conn, "assessments", "legacy_type"));
    assert!(table_has_column(&conn, "mark_sets", "calc_method"));

    let imported = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "class.importLegacy",
        json!({ "legacyClassFolderPath": fixture_folder.to_string_lossy() }),
    );
    assert!(imported.get("classId").and_then(|v| v.as_str()).is_some());
}

#[test]
fn v1_snapshot_migrates_statuses_and_keeps_core_reads_working() {
    let snapshot_db = fixture_path("rust/markbookd/tests/fixtures/db/v1/markbook.sqlite3");
    let workspace = copy_snapshot_to_workspace(&snapshot_db);
    let fixture_folder = fixture_path("fixtures/legacy/Sample25/MB8D25");

    let (_child, mut stdin, mut reader) = spawn_sidecar();

    let _ = request_ok(
        &mut stdin,
        &mut reader,
        "1",
        "workspace.select",
        json!({ "path": workspace.to_string_lossy() }),
    );

    // Existing seeded data from snapshot should still be queryable through IPC.
    let classes = request_ok(&mut stdin, &mut reader, "2", "classes.list", json!({}));
    assert!(
        classes
            .get("classes")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().any(|c| c.get("id").and_then(|v| v.as_str()) == Some("c_old_v1")))
            .unwrap_or(false),
        "expected legacy class from v1 snapshot"
    );

    let grid = request_ok(
        &mut stdin,
        &mut reader,
        "3",
        "grid.get",
        json!({
            "classId": "c_old_v1",
            "markSetId": "ms_old_v1",
            "rowStart": 0,
            "rowCount": 2,
            "colStart": 0,
            "colCount": 1
        }),
    );
    assert_eq!(grid.get("rowCount").and_then(|v| v.as_i64()), Some(2));
    assert_eq!(grid.get("colCount").and_then(|v| v.as_i64()), Some(1));

    let summary = request_ok(
        &mut stdin,
        &mut reader,
        "4",
        "calc.markSetSummary",
        json!({ "classId": "c_old_v1", "markSetId": "ms_old_v1" }),
    );
    assert!(summary.get("perStudent").and_then(|v| v.as_array()).is_some());

    // Migration should convert old score statuses.
    let conn = Connection::open(workspace.join("markbook.sqlite3")).expect("open migrated db");
    let missing_now: String = conn
        .query_row(
            "SELECT status FROM scores WHERE id = 'sc_old_v1_missing'",
            [],
            |r| r.get(0),
        )
        .expect("read migrated missing score");
    let zero_now: String = conn
        .query_row(
            "SELECT status FROM scores WHERE id = 'sc_old_v1_zero'",
            [],
            |r| r.get(0),
        )
        .expect("read migrated scored-zero score");
    assert_eq!(missing_now, "zero");
    assert_eq!(zero_now, "no_mark");

    // Import should still work after migration.
    let imported = request_ok(
        &mut stdin,
        &mut reader,
        "5",
        "class.importLegacy",
        json!({ "legacyClassFolderPath": fixture_folder.to_string_lossy() }),
    );
    assert!(imported.get("classId").and_then(|v| v.as_str()).is_some());
}
