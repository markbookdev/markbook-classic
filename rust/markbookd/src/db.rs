use rusqlite::Connection;
use std::path::Path;

pub fn open_db(workspace: &Path) -> anyhow::Result<Connection> {
    std::fs::create_dir_all(workspace)?;
    let db_path = workspace.join("markbook.sqlite3");
    let conn = Connection::open(db_path)?;
    conn.execute("PRAGMA foreign_keys = ON", [])?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS classes(
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS students(
            id TEXT PRIMARY KEY,
            class_id TEXT NOT NULL,
            last_name TEXT NOT NULL,
            first_name TEXT NOT NULL,
            student_no TEXT,
            birth_date TEXT,
            active INTEGER NOT NULL,
            sort_order INTEGER NOT NULL,
            raw_line TEXT NOT NULL,
            updated_at TEXT,
            FOREIGN KEY(class_id) REFERENCES classes(id)
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_students_class ON students(class_id)",
        [],
    )?;

    // Existing workspaces may have a students table without sort_order. Add and backfill if needed.
    ensure_students_sort_order(&conn)?;
    ensure_students_updated_at(&conn)?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_students_class_sort ON students(class_id, sort_order)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS student_notes(
            id TEXT PRIMARY KEY,
            class_id TEXT NOT NULL,
            student_id TEXT NOT NULL,
            note TEXT NOT NULL,
            FOREIGN KEY(class_id) REFERENCES classes(id),
            FOREIGN KEY(student_id) REFERENCES students(id),
            UNIQUE(class_id, student_id)
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_student_notes_class ON student_notes(class_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_student_notes_student ON student_notes(student_id)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS learning_skills_cells(
            class_id TEXT NOT NULL,
            student_id TEXT NOT NULL,
            term INTEGER NOT NULL,
            skill_code TEXT NOT NULL,
            value TEXT NOT NULL,
            updated_at TEXT,
            PRIMARY KEY(class_id, student_id, term, skill_code),
            FOREIGN KEY(class_id) REFERENCES classes(id),
            FOREIGN KEY(student_id) REFERENCES students(id)
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_learning_skills_class ON learning_skills_cells(class_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_learning_skills_student ON learning_skills_cells(student_id)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS attendance_settings(
            class_id TEXT PRIMARY KEY,
            school_year_start_month INTEGER NOT NULL DEFAULT 9,
            FOREIGN KEY(class_id) REFERENCES classes(id)
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS attendance_months(
            class_id TEXT NOT NULL,
            month INTEGER NOT NULL,
            type_of_day_codes TEXT NOT NULL,
            PRIMARY KEY(class_id, month),
            FOREIGN KEY(class_id) REFERENCES classes(id)
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS attendance_student_months(
            class_id TEXT NOT NULL,
            student_id TEXT NOT NULL,
            month INTEGER NOT NULL,
            day_codes TEXT NOT NULL,
            PRIMARY KEY(class_id, student_id, month),
            FOREIGN KEY(class_id) REFERENCES classes(id),
            FOREIGN KEY(student_id) REFERENCES students(id)
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_attendance_months_class ON attendance_months(class_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_attendance_student_months_class ON attendance_student_months(class_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_attendance_student_months_student ON attendance_student_months(student_id)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS seating_plans(
            class_id TEXT PRIMARY KEY,
            rows INTEGER NOT NULL,
            seats_per_row INTEGER NOT NULL,
            blocked_mask TEXT NOT NULL,
            FOREIGN KEY(class_id) REFERENCES classes(id)
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS seating_assignments(
            class_id TEXT NOT NULL,
            student_id TEXT NOT NULL,
            seat_code INTEGER NOT NULL,
            PRIMARY KEY(class_id, student_id),
            FOREIGN KEY(class_id) REFERENCES classes(id),
            FOREIGN KEY(student_id) REFERENCES students(id)
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_seating_assignments_class ON seating_assignments(class_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_seating_assignments_student ON seating_assignments(student_id)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS loaned_items(
            id TEXT PRIMARY KEY,
            class_id TEXT NOT NULL,
            student_id TEXT NOT NULL,
            mark_set_id TEXT,
            item_name TEXT NOT NULL,
            quantity REAL,
            notes TEXT,
            raw_line TEXT NOT NULL,
            FOREIGN KEY(class_id) REFERENCES classes(id),
            FOREIGN KEY(student_id) REFERENCES students(id),
            FOREIGN KEY(mark_set_id) REFERENCES mark_sets(id)
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_loaned_items_class ON loaned_items(class_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_loaned_items_student ON loaned_items(student_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_loaned_items_mark_set ON loaned_items(mark_set_id)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS student_device_map(
            id TEXT PRIMARY KEY,
            class_id TEXT NOT NULL,
            student_id TEXT NOT NULL,
            device_code TEXT NOT NULL,
            raw_line TEXT NOT NULL,
            UNIQUE(class_id, student_id),
            FOREIGN KEY(class_id) REFERENCES classes(id),
            FOREIGN KEY(student_id) REFERENCES students(id)
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_student_device_map_class ON student_device_map(class_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_student_device_map_student ON student_device_map(student_id)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS mark_sets(
            id TEXT PRIMARY KEY,
            class_id TEXT NOT NULL,
            code TEXT NOT NULL,
            file_prefix TEXT NOT NULL,
            description TEXT NOT NULL,
            weight REAL,
            source_filename TEXT,
            sort_order INTEGER NOT NULL,
            full_code TEXT,
            room TEXT,
            day TEXT,
            period TEXT,
            weight_method INTEGER NOT NULL DEFAULT 1,
            calc_method INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY(class_id) REFERENCES classes(id)
        )",
        [],
    )?;
    ensure_mark_sets_settings_columns(&conn)?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_mark_sets_class ON mark_sets(class_id)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS categories(
            id TEXT PRIMARY KEY,
            mark_set_id TEXT NOT NULL,
            name TEXT NOT NULL,
            weight REAL,
            sort_order INTEGER NOT NULL,
            FOREIGN KEY(mark_set_id) REFERENCES mark_sets(id),
            UNIQUE(mark_set_id, name)
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_categories_mark_set ON categories(mark_set_id)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS assessments(
            id TEXT PRIMARY KEY,
            mark_set_id TEXT NOT NULL,
            idx INTEGER NOT NULL,
            date TEXT,
            category_name TEXT,
            title TEXT NOT NULL,
            term INTEGER,
            legacy_kind INTEGER,
            legacy_type INTEGER,
            weight REAL,
            out_of REAL,
            avg_percent REAL,
            avg_raw REAL,
            FOREIGN KEY(mark_set_id) REFERENCES mark_sets(id),
            UNIQUE(mark_set_id, idx)
        )",
        [],
    )?;
    ensure_assessments_legacy_type(&conn)?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_assessments_mark_set ON assessments(mark_set_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_assessments_mark_set_idx ON assessments(mark_set_id, idx)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS scores(
            id TEXT PRIMARY KEY,
            assessment_id TEXT NOT NULL,
            student_id TEXT NOT NULL,
            raw_value REAL,
            status TEXT NOT NULL,
            remark TEXT,
            FOREIGN KEY(assessment_id) REFERENCES assessments(id),
            FOREIGN KEY(student_id) REFERENCES students(id),
            UNIQUE(assessment_id, student_id)
        )",
        [],
    )?;
    ensure_scores_remark(&conn)?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_scores_assessment ON scores(assessment_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_scores_student ON scores(student_id)",
        [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS comment_set_indexes(
            id TEXT PRIMARY KEY,
            class_id TEXT NOT NULL,
            mark_set_id TEXT NOT NULL,
            set_number INTEGER NOT NULL,
            title TEXT NOT NULL,
            fit_mode INTEGER NOT NULL DEFAULT 0,
            fit_font_size INTEGER NOT NULL DEFAULT 8,
            fit_width INTEGER NOT NULL DEFAULT 50,
            fit_lines INTEGER NOT NULL DEFAULT 1,
            fit_subj TEXT NOT NULL DEFAULT '',
            max_chars INTEGER NOT NULL DEFAULT 100,
            is_default INTEGER NOT NULL DEFAULT 0,
            bank_short TEXT,
            FOREIGN KEY(class_id) REFERENCES classes(id),
            FOREIGN KEY(mark_set_id) REFERENCES mark_sets(id),
            UNIQUE(mark_set_id, set_number)
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS comment_set_remarks(
            id TEXT PRIMARY KEY,
            comment_set_index_id TEXT NOT NULL,
            student_id TEXT NOT NULL,
            remark TEXT NOT NULL,
            FOREIGN KEY(comment_set_index_id) REFERENCES comment_set_indexes(id),
            FOREIGN KEY(student_id) REFERENCES students(id),
            UNIQUE(comment_set_index_id, student_id)
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS comment_banks(
            id TEXT PRIMARY KEY,
            short_name TEXT NOT NULL UNIQUE,
            is_default INTEGER NOT NULL DEFAULT 0,
            fit_profile TEXT,
            source_path TEXT
        )",
        [],
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS comment_bank_entries(
            id TEXT PRIMARY KEY,
            bank_id TEXT NOT NULL,
            sort_order INTEGER NOT NULL,
            type_code TEXT NOT NULL,
            level_code TEXT NOT NULL,
            text TEXT NOT NULL,
            FOREIGN KEY(bank_id) REFERENCES comment_banks(id),
            UNIQUE(bank_id, sort_order)
        )",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_comment_set_indexes_class ON comment_set_indexes(class_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_comment_set_indexes_mark_set ON comment_set_indexes(mark_set_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_comment_set_remarks_set ON comment_set_remarks(comment_set_index_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_comment_set_remarks_student ON comment_set_remarks(student_id)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_comment_bank_entries_bank ON comment_bank_entries(bank_id)",
        [],
    )?;

    // Migrate older workspaces to the expanded mark-state semantics:
    // - "missing" (raw_value NULL) => "zero"
    // - "scored" with raw_value=0 => "no_mark"
    migrate_scores_statuses(&conn)?;

    Ok(conn)
}

fn ensure_students_sort_order(conn: &Connection) -> anyhow::Result<()> {
    // If the column already exists, we're done.
    if table_has_column(conn, "students", "sort_order")? {
        return Ok(());
    }

    conn.execute(
        "ALTER TABLE students ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0",
        [],
    )?;

    // Backfill per class using existing insert order as a best-effort.
    let mut class_stmt = conn.prepare("SELECT id FROM classes ORDER BY rowid")?;
    let class_ids = class_stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;

    let mut stud_stmt =
        conn.prepare("SELECT id FROM students WHERE class_id = ? ORDER BY rowid")?;

    for cid in class_ids {
        let student_ids = stud_stmt
            .query_map([&cid], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        for (i, sid) in student_ids.iter().enumerate() {
            conn.execute(
                "UPDATE students SET sort_order = ? WHERE id = ?",
                (i as i64, sid),
            )?;
        }
    }

    Ok(())
}

fn ensure_students_updated_at(conn: &Connection) -> anyhow::Result<()> {
    if table_has_column(conn, "students", "updated_at")? {
        return Ok(());
    }
    conn.execute("ALTER TABLE students ADD COLUMN updated_at TEXT", [])?;
    Ok(())
}

fn ensure_mark_sets_settings_columns(conn: &Connection) -> anyhow::Result<()> {
    if !table_has_column(conn, "mark_sets", "full_code")? {
        conn.execute("ALTER TABLE mark_sets ADD COLUMN full_code TEXT", [])?;
    }
    if !table_has_column(conn, "mark_sets", "room")? {
        conn.execute("ALTER TABLE mark_sets ADD COLUMN room TEXT", [])?;
    }
    if !table_has_column(conn, "mark_sets", "day")? {
        conn.execute("ALTER TABLE mark_sets ADD COLUMN day TEXT", [])?;
    }
    if !table_has_column(conn, "mark_sets", "period")? {
        conn.execute("ALTER TABLE mark_sets ADD COLUMN period TEXT", [])?;
    }
    if !table_has_column(conn, "mark_sets", "weight_method")? {
        conn.execute(
            "ALTER TABLE mark_sets ADD COLUMN weight_method INTEGER NOT NULL DEFAULT 1",
            [],
        )?;
    }
    if !table_has_column(conn, "mark_sets", "calc_method")? {
        conn.execute(
            "ALTER TABLE mark_sets ADD COLUMN calc_method INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }
    Ok(())
}

fn ensure_assessments_legacy_type(conn: &Connection) -> anyhow::Result<()> {
    if table_has_column(conn, "assessments", "legacy_type")? {
        return Ok(());
    }
    conn.execute("ALTER TABLE assessments ADD COLUMN legacy_type INTEGER", [])?;
    Ok(())
}

fn ensure_scores_remark(conn: &Connection) -> anyhow::Result<()> {
    if table_has_column(conn, "scores", "remark")? {
        return Ok(());
    }
    conn.execute("ALTER TABLE scores ADD COLUMN remark TEXT", [])?;
    Ok(())
}

fn migrate_scores_statuses(conn: &Connection) -> anyhow::Result<()> {
    // v0 -> v1 mark state semantics:
    // - legacy raw < 0 means "Zero" (counts as 0) not "Missing"
    // - legacy raw == 0 means "No Mark" (excluded) not "Scored 0"
    //
    // Older DBs used:
    // - status="missing" + raw_value NULL
    // - status="scored" + raw_value 0
    conn.execute(
        "UPDATE scores SET status = 'zero' WHERE status = 'missing' AND raw_value IS NULL",
        [],
    )?;
    conn.execute(
        "UPDATE scores SET status = 'no_mark', raw_value = 0 WHERE status = 'scored' AND raw_value = 0",
        [],
    )?;
    Ok(())
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> anyhow::Result<bool> {
    let sql = format!("PRAGMA table_info({})", table);
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}
