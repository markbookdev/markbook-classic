use std::path::{Path, PathBuf};

pub fn find_cl_file(folder: &Path) -> anyhow::Result<PathBuf> {
    let entries = std::fs::read_dir(folder)?;
    for ent in entries {
        let ent = ent?;
        let p = ent.path();
        if !p.is_file() {
            continue;
        }
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let name_up = name.to_ascii_uppercase();
        // Example: CL8D.Y25
        if name_up.starts_with("CL") && name_up.contains(".Y") {
            return Ok(p);
        }
    }
    anyhow::bail!("no CL*.Yxx file found in folder")
}

pub struct ParsedCl {
    pub class_name: String,
    pub mark_sets: Vec<ParsedMarkSetDef>,
    pub students: Vec<ParsedStudent>,
}

#[derive(Clone)]
pub struct ParsedMarkSetDef {
    pub file_prefix: String,
    pub code: String,
    pub description: String,
    pub weight: f64,
    pub sort_order: usize,
}

pub struct ParsedStudent {
    pub active: bool,
    pub last_name: String,
    pub first_name: String,
    pub student_no: Option<String>,
    pub birth_date: Option<String>,
    pub mark_set_mask: Option<String>,
    pub raw_line: String,
}

pub fn parse_legacy_cl(cl_path: &Path) -> anyhow::Result<ParsedCl> {
    let bytes = std::fs::read(cl_path)?;
    let text = String::from_utf8_lossy(&bytes);

    let mut section: Option<String> = None;
    let mut general: Vec<String> = Vec::new();
    let mut expected_mark_sets: Option<usize> = None;
    let mut mark_sets: Vec<ParsedMarkSetDef> = Vec::new();
    let mut expected_students: Option<usize> = None;
    let mut students: Vec<ParsedStudent> = Vec::new();

    for raw in text.lines() {
        let t = raw.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('[') && t.ends_with(']') && t.len() >= 2 {
            section = Some(t.trim_start_matches('[').trim_end_matches(']').to_string());
            continue;
        }

        match section.as_deref() {
            Some("Mark Sets created for this class") => {
                if expected_mark_sets.is_none() {
                    if let Ok(n) = strip_quotes(t).trim().parse::<usize>() {
                        expected_mark_sets = Some(n);
                    }
                    continue;
                }

                let n = expected_mark_sets.unwrap_or(0);
                if mark_sets.len() >= n {
                    continue;
                }

                let v = strip_quotes(t);
                if v.is_empty() {
                    continue;
                }

                if let Some(def) = parse_mark_set_def(&v, mark_sets.len()) {
                    mark_sets.push(def);
                }
            }
            Some("General Information") => {
                let v = strip_quotes(t);
                // VB6 file contains lots of empty "" lines. Ignore them.
                if !v.is_empty() {
                    general.push(v);
                }
            }
            Some("Class List") => {
                if expected_students.is_none() {
                    // First non-empty line is the count.
                    if let Ok(n) = strip_quotes(t).trim().parse::<usize>() {
                        expected_students = Some(n);
                    }
                    continue;
                }

                let n = expected_students.unwrap_or(0);
                if students.len() >= n {
                    continue;
                }
                if t == "\"\"" {
                    continue;
                }
                if let Some(s) = parse_student_line(raw) {
                    students.push(s);
                }
            }
            _ => {}
        }
    }

    // From sample: phone, school, class name, teacher name...
    let class_name = general
        .get(2)
        .cloned()
        .unwrap_or_else(|| "Imported Class".to_string());

    Ok(ParsedCl {
        class_name,
        mark_sets,
        students,
    })
}

fn strip_quotes(s: &str) -> String {
    let mut out = s.trim().to_string();
    if out.starts_with('"') && out.ends_with('"') && out.len() >= 2 {
        out = out[1..out.len() - 1].to_string();
    }
    out.trim().to_string()
}

fn parse_student_line(line: &str) -> Option<ParsedStudent> {
    let raw = line.trim().to_string();
    let parts: Vec<String> = line.split(',').map(|x| x.trim().to_string()).collect();
    if parts.len() < 4 {
        return None;
    }

    let active = parts
        .get(0)
        .and_then(|s| s.trim().parse::<i32>().ok())
        .unwrap_or(1)
        != 0;
    let last_name = parts.get(1).cloned().unwrap_or_default();
    let first_name = parts.get(2).cloned().unwrap_or_default();
    let student_no = parts.get(4).cloned().filter(|s| !s.is_empty());
    let birth_date = parts.get(9).cloned().filter(|s| !s.is_empty());
    let mark_set_mask = parts.last().and_then(|s| parse_mark_set_mask_token(s));

    Some(ParsedStudent {
        active,
        last_name,
        first_name,
        student_no,
        birth_date,
        mark_set_mask,
        raw_line: raw,
    })
}

fn parse_mark_set_mask_token(token: &str) -> Option<String> {
    let t = token.trim();
    if t.is_empty() {
        return None;
    }
    let up = t.to_ascii_uppercase();
    if up == "TBA" {
        return Some("TBA".into());
    }
    if up.chars().all(|ch| ch == '0' || ch == '1') {
        return Some(up);
    }
    None
}

fn parse_mark_set_def(line: &str, sort_order: usize) -> Option<ParsedMarkSetDef> {
    // Format: prefix&code,description,weight
    let parts: Vec<String> = line.split(',').map(|x| x.trim().to_string()).collect();
    if parts.len() < 3 {
        return None;
    }

    let id_part = parts[0].trim();
    let mut file_prefix = id_part.to_string();
    let mut code = id_part.to_string();
    if let Some((a, b)) = id_part.split_once('&') {
        file_prefix = a.trim().to_string();
        code = b.trim().to_string();
    }

    let description = parts[1].trim().to_string();
    let weight = parts[2].trim().parse::<f64>().ok().unwrap_or(0.0);

    Some(ParsedMarkSetDef {
        file_prefix,
        code,
        description,
        weight,
        sort_order,
    })
}

fn is_legacy_year_file(name: &str) -> bool {
    let name_up = name.to_ascii_uppercase();
    if name_up.len() < 4 {
        return false;
    }
    let bytes = name_up.as_bytes();
    let n = bytes.len();
    if bytes[n - 4] != b'.' || bytes[n - 3] != b'Y' {
        return false;
    }
    (bytes[n - 2] as char).is_ascii_digit() && (bytes[n - 1] as char).is_ascii_digit()
}

pub fn find_mark_file(folder: &Path, file_prefix: &str) -> anyhow::Result<Option<PathBuf>> {
    let prefix_up = file_prefix.to_ascii_uppercase();
    let mut candidates: Vec<PathBuf> = Vec::new();
    for ent in std::fs::read_dir(folder)? {
        let ent = ent?;
        let p = ent.path();
        if !p.is_file() {
            continue;
        }
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let name_up = name.to_ascii_uppercase();
        if name_up.starts_with("CL") {
            continue;
        }
        if !name_up.starts_with(&prefix_up) {
            continue;
        }
        if !is_legacy_year_file(&name_up) {
            continue;
        }
        candidates.push(p);
    }
    candidates.sort();
    Ok(candidates.into_iter().next())
}

pub fn find_note_file(folder: &Path) -> anyhow::Result<Option<PathBuf>> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    for ent in std::fs::read_dir(folder)? {
        let ent = ent?;
        let p = ent.path();
        if !p.is_file() {
            continue;
        }
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let name_up = name.to_ascii_uppercase();
        if name_up.ends_with("NOTE.TXT") {
            candidates.push(p);
        }
    }
    candidates.sort();
    Ok(candidates.into_iter().next())
}

pub fn find_attendance_file(folder: &Path) -> anyhow::Result<Option<PathBuf>> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    for ent in std::fs::read_dir(folder)? {
        let ent = ent?;
        let p = ent.path();
        if !p.is_file() {
            continue;
        }
        let Some(ext) = p.extension().and_then(|s| s.to_str()) else {
            continue;
        };
        if ext.eq_ignore_ascii_case("ATN") {
            candidates.push(p);
        }
    }
    candidates.sort();
    Ok(candidates.into_iter().next())
}

pub fn find_seating_file(folder: &Path) -> anyhow::Result<Option<PathBuf>> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    for ent in std::fs::read_dir(folder)? {
        let ent = ent?;
        let p = ent.path();
        if !p.is_file() {
            continue;
        }
        let Some(ext) = p.extension().and_then(|s| s.to_str()) else {
            continue;
        };
        if ext.eq_ignore_ascii_case("SPL") {
            candidates.push(p);
        }
    }
    candidates.sort();
    Ok(candidates.into_iter().next())
}

pub fn find_bnk_files(folder: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = Vec::new();
    for ent in std::fs::read_dir(folder)? {
        let ent = ent?;
        let p = ent.path();
        if !p.is_file() {
            continue;
        }
        let Some(ext) = p.extension().and_then(|s| s.to_str()) else {
            continue;
        };
        if ext.eq_ignore_ascii_case("BNK") {
            out.push(p);
        }
    }
    out.sort();
    Ok(out)
}

pub fn find_tbk_files(folder: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = Vec::new();
    for ent in std::fs::read_dir(folder)? {
        let ent = ent?;
        let p = ent.path();
        if !p.is_file() {
            continue;
        }
        let Some(ext) = p.extension().and_then(|s| s.to_str()) else {
            continue;
        };
        if ext.eq_ignore_ascii_case("TBK") {
            out.push(p);
        }
    }
    out.sort();
    Ok(out)
}

pub fn find_icc_file(folder: &Path) -> anyhow::Result<Option<PathBuf>> {
    let mut out: Vec<PathBuf> = Vec::new();
    for ent in std::fs::read_dir(folder)? {
        let ent = ent?;
        let p = ent.path();
        if !p.is_file() {
            continue;
        }
        let Some(ext) = p.extension().and_then(|s| s.to_str()) else {
            continue;
        };
        if ext.eq_ignore_ascii_case("ICC") {
            out.push(p);
        }
    }
    out.sort();
    Ok(out.into_iter().next())
}

pub fn find_all_idx_file(folder: &Path) -> anyhow::Result<Option<PathBuf>> {
    let mut out: Vec<PathBuf> = Vec::new();
    for ent in std::fs::read_dir(folder)? {
        let ent = ent?;
        let p = ent.path();
        if !p.is_file() {
            continue;
        }
        let Some(name) = p.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let up = name.to_ascii_uppercase();
        if up.starts_with("ALL!") && up.ends_with(".IDX") {
            out.push(p);
        }
    }
    out.sort();
    Ok(out.into_iter().next())
}

pub struct ParsedCategory {
    pub name: String,
    pub weight: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LegacyScore {
    /// Legacy raw == 0. Excluded from calculations and displayed as blank.
    NoMark,
    /// Legacy raw < 0 (typically -1). Included in calculations as 0 and displayed as 0.
    Zero,
    /// Legacy raw > 0.
    Scored(f64),
}

pub struct ParsedAssessment {
    pub idx: usize,
    pub date: String,
    pub category_name: String,
    pub title: String,
    pub term: i32,
    pub legacy_kind: i32,
    pub weight: f64,
    pub out_of: f64,
    pub avg_percent: f64,
    pub avg_raw: f64,
    pub raw_scores: Vec<LegacyScore>,
}

#[allow(dead_code)]
pub struct ParsedMiscInfo {
    pub full_code: String,
    pub room: String,
    pub day: String,
    pub period: String,
    pub weight_method: i32,
    pub calc_method: i32,
    // Legacy file contains an extra serial-ish value we don't interpret yet.
    pub legacy_serial: Option<f64>,
}

pub struct ParsedMarkFile {
    pub misc: Option<ParsedMiscInfo>,
    pub categories: Vec<ParsedCategory>,
    pub last_student: usize,
    pub assessments: Vec<ParsedAssessment>,
}

pub fn parse_legacy_mark_file(path: &Path) -> anyhow::Result<ParsedMarkFile> {
    let bytes = std::fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<String> = text
        .lines()
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect();

    let misc = find_section(&lines, "Misc Info").map(|misc_idx| {
        // The [Misc Info] block is positional and includes significant empty values.
        // Observed layout (Sample25):
        // 0 full_code, 1 room, 2 day, 3 period, 4 weight_method, 5 legacy_serial, 6 calc_method, 7 unused
        let mut m = misc_idx + 1;
        let full_code = next_keep_empty(&lines, &mut m).unwrap_or_default();
        let room = next_keep_empty(&lines, &mut m).unwrap_or_default();
        let day = next_keep_empty(&lines, &mut m).unwrap_or_default();
        let period = next_keep_empty(&lines, &mut m).unwrap_or_default();
        let weight_method = next_keep_empty(&lines, &mut m)
            .and_then(|s| s.trim().parse::<i32>().ok())
            .unwrap_or(1);
        let legacy_serial = next_keep_empty(&lines, &mut m).and_then(|s| s.parse::<f64>().ok());
        let calc_method = next_keep_empty(&lines, &mut m)
            .and_then(|s| s.trim().parse::<i32>().ok())
            .unwrap_or(0);
        let _unused = next_keep_empty(&lines, &mut m);

        ParsedMiscInfo {
            full_code,
            room,
            day,
            period,
            weight_method,
            calc_method,
            legacy_serial,
        }
    });

    let cat_idx = find_section(&lines, "Categories")
        .ok_or_else(|| anyhow::anyhow!("missing [Categories] section"))?;
    let last_idx = find_section(&lines, "LastStudent")
        .ok_or_else(|| anyhow::anyhow!("missing [LastStudent] section"))?;
    let marks_idx =
        find_section(&lines, "Marks").ok_or_else(|| anyhow::anyhow!("missing [Marks] section"))?;

    let mut i = cat_idx + 1;
    let cat_count_line =
        next_non_noise(&lines, &mut i).ok_or_else(|| anyhow::anyhow!("missing category count"))?;
    let cat_count = cat_count_line
        .trim()
        .parse::<usize>()
        .map_err(|_| anyhow::anyhow!("bad category count: {}", cat_count_line))?;

    let mut categories: Vec<ParsedCategory> = Vec::new();
    for _ in 0..cat_count {
        let l = next_non_noise(&lines, &mut i)
            .ok_or_else(|| anyhow::anyhow!("unexpected EOF in categories"))?;
        let parts: Vec<String> = l.split(',').map(|x| x.trim().to_string()).collect();
        if parts.len() < 2 {
            return Err(anyhow::anyhow!("bad category line: {}", l));
        }
        let name = parts[0].clone();
        let weight = parts[1].parse::<f64>().unwrap_or(0.0);
        categories.push(ParsedCategory { name, weight });
    }

    let mut j = last_idx + 1;
    let last_student_line =
        next_non_noise(&lines, &mut j).ok_or_else(|| anyhow::anyhow!("missing last student"))?;
    let last_student = last_student_line
        .trim()
        .parse::<usize>()
        .map_err(|_| anyhow::anyhow!("bad last student: {}", last_student_line))?;

    let mut k = marks_idx + 1;
    let marks_count_line =
        next_non_noise(&lines, &mut k).ok_or_else(|| anyhow::anyhow!("missing marks count"))?;
    let marks_count = marks_count_line
        .trim()
        .parse::<usize>()
        .map_err(|_| anyhow::anyhow!("bad marks count: {}", marks_count_line))?;

    let mut assessments: Vec<ParsedAssessment> = Vec::new();
    for idx in 0..marks_count {
        let date_line = next_non_noise(&lines, &mut k)
            .ok_or_else(|| anyhow::anyhow!("unexpected EOF reading date"))?;
        let date = parse_date_ymd(&date_line)
            .ok_or_else(|| anyhow::anyhow!("bad date line: {}", date_line))?;

        let category_name = next_non_noise(&lines, &mut k)
            .ok_or_else(|| anyhow::anyhow!("unexpected EOF reading category"))?;
        let title = next_non_noise(&lines, &mut k)
            .ok_or_else(|| anyhow::anyhow!("unexpected EOF reading title"))?;
        let term_line = next_non_noise(&lines, &mut k)
            .ok_or_else(|| anyhow::anyhow!("unexpected EOF reading term"))?;
        let term = term_line.trim().parse::<i32>().unwrap_or(0);
        let summary_line = next_non_noise(&lines, &mut k)
            .ok_or_else(|| anyhow::anyhow!("unexpected EOF reading summary"))?;
        let summary = parse_csv_numbers(&summary_line, 5)
            .ok_or_else(|| anyhow::anyhow!("bad summary line: {}", summary_line))?;
        let legacy_kind = summary[0] as i32;
        let weight = summary[1];
        let avg_percent = summary[2];
        let out_of = summary[3];
        let avg_raw = summary[4];

        let mut raw_scores: Vec<LegacyScore> = Vec::with_capacity(last_student);
        for _ in 0..last_student {
            let sline = next_non_noise(&lines, &mut k)
                .ok_or_else(|| anyhow::anyhow!("unexpected EOF reading student marks"))?;
            let nums = parse_csv_numbers(&sline, 2)
                .ok_or_else(|| anyhow::anyhow!("bad student mark line: {}", sline))?;
            let raw = nums[1];
            // Legacy semantics:
            // - raw == 0 => No Mark (excluded)
            // - raw < 0  => Zero (counts as 0)
            // - raw > 0  => Scored
            if raw == 0.0 {
                raw_scores.push(LegacyScore::NoMark);
            } else if raw < 0.0 {
                raw_scores.push(LegacyScore::Zero);
            } else {
                raw_scores.push(LegacyScore::Scored(raw));
            }
        }

        assessments.push(ParsedAssessment {
            idx,
            date,
            category_name,
            title,
            term,
            legacy_kind,
            weight,
            out_of,
            avg_percent,
            avg_raw,
            raw_scores,
        });
    }

    Ok(ParsedMarkFile {
        misc,
        categories,
        last_student,
        assessments,
    })
}

pub fn parse_legacy_typ_file(path: &Path) -> anyhow::Result<Vec<i32>> {
    let bytes = std::fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<String> = text
        .lines()
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect();

    let idx = find_section(&lines, "Last Entry")
        .ok_or_else(|| anyhow::anyhow!("missing [Last Entry] section"))?;
    let mut i = idx + 1;
    let count_line = next_non_noise(&lines, &mut i)
        .ok_or_else(|| anyhow::anyhow!("missing last entry count"))?;
    let count = count_line
        .trim()
        .parse::<usize>()
        .map_err(|_| anyhow::anyhow!("bad last entry count: {}", count_line))?;

    let mut out: Vec<i32> = Vec::with_capacity(count);
    for _ in 0..count {
        let l = next_non_noise(&lines, &mut i)
            .ok_or_else(|| anyhow::anyhow!("unexpected EOF in .TYP entries"))?;
        let v = l.trim().parse::<i32>().unwrap_or(0);
        out.push(v);
    }

    Ok(out)
}

#[allow(dead_code)]
pub struct ParsedRmkFile {
    pub last_student: usize,
    pub last_entry: usize,
    pub entry_titles: Vec<String>,
    // Per entry, per student (excluding kid0), in legacy row order.
    pub remarks_by_entry: Vec<Vec<String>>,
}

pub fn parse_legacy_rmk_file(path: &Path) -> anyhow::Result<ParsedRmkFile> {
    let bytes = std::fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<String> = text
        .lines()
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect();

    let idx = find_section(&lines, "LastStudent - Last Entry")
        .ok_or_else(|| anyhow::anyhow!("missing [LastStudent - Last Entry] section"))?;

    let mut i = idx + 1;
    let count_line = next_non_noise(&lines, &mut i)
        .ok_or_else(|| anyhow::anyhow!("missing last student/entry line"))?;
    let parts: Vec<&str> = count_line.split(',').collect();
    if parts.len() < 2 {
        return Err(anyhow::anyhow!(
            "bad last student/entry line: {}",
            count_line
        ));
    }
    let last_student = parts[0].trim().parse::<usize>().unwrap_or(0);
    let last_entry = parts[1].trim().parse::<usize>().unwrap_or(0);

    let mut entry_titles: Vec<String> = Vec::with_capacity(last_entry);
    let mut remarks_by_entry: Vec<Vec<String>> = Vec::with_capacity(last_entry);

    for _ in 0..last_entry {
        let title = next_non_noise(&lines, &mut i)
            .ok_or_else(|| anyhow::anyhow!("unexpected EOF reading .RMK title"))?;
        entry_titles.push(title);

        let mut remarks: Vec<String> = Vec::with_capacity(last_student);
        for s_idx in 0..=last_student {
            let v = next_keep_empty(&lines, &mut i)
                .ok_or_else(|| anyhow::anyhow!("unexpected EOF reading .RMK remarks"))?;
            // kid0 is the first line; ignore for now.
            if s_idx == 0 {
                continue;
            }
            remarks.push(v);
        }
        remarks_by_entry.push(remarks);
    }

    Ok(ParsedRmkFile {
        last_student,
        last_entry,
        entry_titles,
        remarks_by_entry,
    })
}

pub fn parse_legacy_note_file(path: &Path) -> anyhow::Result<Vec<String>> {
    let bytes = std::fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<String> = text
        .lines()
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect();

    let idx = find_section(&lines, "Comments")
        .ok_or_else(|| anyhow::anyhow!("missing [Comments] section"))?;
    let mut i = idx + 1;
    let count_line =
        next_non_noise(&lines, &mut i).ok_or_else(|| anyhow::anyhow!("missing comment count"))?;
    let count = count_line
        .trim()
        .parse::<usize>()
        .map_err(|_| anyhow::anyhow!("bad comment count: {}", count_line))?;

    let mut out: Vec<String> = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(read_quoted_block(&lines, &mut i)?);
    }
    Ok(out)
}

pub struct ParsedAttendanceMonth {
    pub month: i32,
    pub type_of_day_codes: String,
    pub student_day_codes: Vec<String>,
}

pub struct ParsedAttendanceFile {
    pub last_student: usize,
    pub school_year_start_month: i32,
    pub months: Vec<ParsedAttendanceMonth>,
}

pub fn parse_legacy_attendance_file(path: &Path) -> anyhow::Result<ParsedAttendanceFile> {
    let bytes = std::fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<String> = text
        .lines()
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect();

    let last_idx = find_section(&lines, "LastStudent")
        .ok_or_else(|| anyhow::anyhow!("missing [LastStudent] section in attendance file"))?;
    let mut i = last_idx + 1;
    let last_student_line =
        next_non_noise(&lines, &mut i).ok_or_else(|| anyhow::anyhow!("missing last student"))?;
    let last_student = last_student_line
        .trim()
        .parse::<usize>()
        .map_err(|_| anyhow::anyhow!("bad last student: {}", last_student_line))?;

    let school_idx = find_section(&lines, "School Year Starts")
        .ok_or_else(|| anyhow::anyhow!("missing [School Year Starts] section"))?;
    let mut j = school_idx + 1;
    let school_year_start_month = next_non_noise(&lines, &mut j)
        .and_then(|v| v.trim().parse::<i32>().ok())
        .unwrap_or(9);

    let data_idx = find_section(&lines, "Attendance Data - DO NOT EDIT!!!")
        .ok_or_else(|| anyhow::anyhow!("missing [Attendance Data - DO NOT EDIT!!!] section"))?;
    let mut k = data_idx + 1;
    let mut months: Vec<ParsedAttendanceMonth> = Vec::new();
    for month in 1..=12_i32 {
        let _label = next_non_noise(&lines, &mut k)
            .ok_or_else(|| anyhow::anyhow!("unexpected EOF reading month label {}", month))?;
        let type_of_day_codes = next_keep_empty(&lines, &mut k).unwrap_or_default();
        let mut student_day_codes: Vec<String> = Vec::with_capacity(last_student);
        for _ in 0..last_student {
            student_day_codes.push(next_keep_empty(&lines, &mut k).unwrap_or_default());
        }
        months.push(ParsedAttendanceMonth {
            month,
            type_of_day_codes,
            student_day_codes,
        });
    }

    Ok(ParsedAttendanceFile {
        last_student,
        school_year_start_month,
        months,
    })
}

pub struct ParsedSeatingFile {
    pub rows: i32,
    pub seats_per_row: i32,
    pub last_student: usize,
    pub blocked_mask: String,
    pub seat_codes: Vec<i32>,
}

pub fn parse_legacy_seating_file(path: &Path) -> anyhow::Result<ParsedSeatingFile> {
    let bytes = std::fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<String> = text
        .lines()
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect();

    let rows_idx = find_section(&lines, "Number of Rows / Seats per Row")
        .ok_or_else(|| anyhow::anyhow!("missing [Number of Rows / Seats per Row] section"))?;
    let mut i = rows_idx + 1;
    let row_line =
        next_non_noise(&lines, &mut i).ok_or_else(|| anyhow::anyhow!("missing rows/seats line"))?;
    let row_parts = parse_csv_i32(&row_line, 2)
        .ok_or_else(|| anyhow::anyhow!("bad rows/seats line: {}", row_line))?;
    let rows = row_parts[0];
    let seats_per_row = row_parts[1];

    let last_idx = find_section(&lines, "LastStudent")
        .ok_or_else(|| anyhow::anyhow!("missing [LastStudent] section in seating file"))?;
    let mut j = last_idx + 1;
    let last_student_line =
        next_non_noise(&lines, &mut j).ok_or_else(|| anyhow::anyhow!("missing last student"))?;
    let last_student = last_student_line
        .trim()
        .parse::<usize>()
        .map_err(|_| anyhow::anyhow!("bad last student: {}", last_student_line))?;
    let blocked_mask = next_keep_empty(&lines, &mut j).unwrap_or_default();
    let blocked_mask = blocked_mask
        .chars()
        .map(|ch| if ch == '1' { '1' } else { '0' })
        .collect::<String>();

    let mut seat_codes: Vec<i32> = Vec::with_capacity(last_student);
    for _ in 0..last_student {
        let line = next_non_noise(&lines, &mut j)
            .ok_or_else(|| anyhow::anyhow!("unexpected EOF reading seat assignment"))?;
        seat_codes.push(line.trim().parse::<i32>().unwrap_or(0));
    }

    Ok(ParsedSeatingFile {
        rows,
        seats_per_row,
        last_student,
        blocked_mask,
        seat_codes,
    })
}

pub struct ParsedCommentSetDef {
    pub set_number: usize,
    pub title: String,
    pub fit_mode: i32,
    pub fit_font_size: i32,
    pub fit_width: i32,
    pub fit_lines: i32,
    pub fit_subj: String,
    pub max_chars: i32,
    pub is_default: bool,
    pub bank_short: Option<String>,
}

pub struct ParsedIdxFile {
    pub fit_max_letters: i32,
    pub default_set: usize,
    pub sets: Vec<ParsedCommentSetDef>,
    pub bank_short: Option<String>,
}

pub fn parse_legacy_idx_file(path: &Path) -> anyhow::Result<ParsedIdxFile> {
    let bytes = std::fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<String> = text
        .lines()
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect();

    if lines.is_empty() {
        return Err(anyhow::anyhow!("empty IDX file"));
    }

    // Legacy "old format" starts directly with count/default/title list.
    let mut probe = 0;
    let first_token = next_non_noise(&lines, &mut probe).unwrap_or_default();
    if let Ok(erc_count) = first_token.trim().parse::<usize>() {
        let mut i = probe;
        let default_set = next_non_noise(&lines, &mut i)
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(1);
        let mut sets: Vec<ParsedCommentSetDef> = Vec::with_capacity(erc_count);
        for set_number in 1..=erc_count {
            let title =
                next_non_noise(&lines, &mut i).unwrap_or_else(|| format!("Set {}", set_number));
            sets.push(ParsedCommentSetDef {
                set_number,
                title,
                fit_mode: 0,
                fit_font_size: 8,
                fit_width: 50,
                fit_lines: 1,
                fit_subj: String::new(),
                max_chars: 100,
                is_default: set_number == default_set,
                bank_short: None,
            });
        }
        return Ok(ParsedIdxFile {
            fit_max_letters: 100,
            default_set,
            sets,
            bank_short: None,
        });
    }

    // Current format with header lines and fit metadata.
    let owner_idx = lines
        .iter()
        .position(|l| {
            l.to_ascii_lowercase()
                .contains("this comment index file belongs")
        })
        .ok_or_else(|| anyhow::anyhow!("unable to locate IDX owner section"))?;

    let mut i = owner_idx + 1;
    let _folder_line = next_non_noise(&lines, &mut i)
        .ok_or_else(|| anyhow::anyhow!("missing IDX folder/class line"))?;
    let fit_max_letters = next_non_noise(&lines, &mut i)
        .and_then(|v| v.trim().parse::<i32>().ok())
        .unwrap_or(100)
        .max(100);
    let erc_count = next_non_noise(&lines, &mut i)
        .and_then(|v| v.trim().parse::<usize>().ok())
        .ok_or_else(|| anyhow::anyhow!("missing IDX set count"))?;
    let default_set = next_non_noise(&lines, &mut i)
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(1)
        .clamp(1, erc_count.max(1));

    let mut bank_short: Option<String> = None;
    let mut sets: Vec<ParsedCommentSetDef> = Vec::with_capacity(erc_count);
    for set_number in 1..=erc_count {
        let title = next_non_noise(&lines, &mut i).unwrap_or_else(|| format!("Set {}", set_number));
        let fit_line = next_non_noise(&lines, &mut i).unwrap_or_else(|| "0,8,50,1".to_string());
        let fit_vals = parse_csv_i32(&fit_line, 4).unwrap_or_else(|| vec![0, 8, 50, 1]);
        let fit_subj = next_keep_empty(&lines, &mut i).unwrap_or_default();
        let bank_line = next_keep_empty(&lines, &mut i).unwrap_or_default();
        if set_number == 1 && !bank_line.trim().is_empty() {
            bank_short = Some(bank_line.trim().to_string());
        }

        sets.push(ParsedCommentSetDef {
            set_number,
            title,
            fit_mode: *fit_vals.first().unwrap_or(&0),
            fit_font_size: *fit_vals.get(1).unwrap_or(&8),
            fit_width: *fit_vals.get(2).unwrap_or(&50),
            fit_lines: *fit_vals.get(3).unwrap_or(&1),
            fit_subj,
            max_chars: 100,
            is_default: set_number == default_set,
            bank_short: if set_number == 1 {
                bank_short.clone()
            } else {
                None
            },
        });
    }

    if let Some(max_idx) = find_section(&lines, "Max Characters for each Comment Set") {
        let mut m = max_idx + 1;
        for set in &mut sets {
            if let Some(v) = next_non_noise(&lines, &mut m).and_then(|s| s.parse::<i32>().ok()) {
                set.max_chars = v.max(100);
            }
        }
    }

    Ok(ParsedIdxFile {
        fit_max_letters,
        default_set,
        sets,
        bank_short,
    })
}

pub struct ParsedRCommentFile {
    pub last_student: usize,
    pub remarks: Vec<String>,
}

pub fn parse_legacy_r_comment_file(path: &Path) -> anyhow::Result<ParsedRCommentFile> {
    let bytes = std::fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<String> = text
        .lines()
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect();

    let idx = find_section(&lines, "Comments")
        .ok_or_else(|| anyhow::anyhow!("missing [Comments] section"))?;
    let mut i = idx + 1;
    let count_line =
        next_non_noise(&lines, &mut i).ok_or_else(|| anyhow::anyhow!("missing comment count"))?;
    let count = count_line
        .trim()
        .parse::<usize>()
        .map_err(|_| anyhow::anyhow!("bad comment count: {}", count_line))?;

    let mut remarks: Vec<String> = Vec::with_capacity(count);
    for _ in 0..count {
        remarks.push(read_quoted_block(&lines, &mut i)?);
    }

    Ok(ParsedRCommentFile {
        last_student: count,
        remarks,
    })
}

pub struct ParsedBnkEntry {
    pub sort_order: usize,
    pub type_code: String,
    pub level_code: String,
    pub text: String,
}

pub struct ParsedBnkFile {
    pub fit_profile: Option<String>,
    pub entries: Vec<ParsedBnkEntry>,
}

pub fn parse_bnk_file(path: &Path) -> anyhow::Result<ParsedBnkFile> {
    let bytes = std::fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes);
    let mut entries: Vec<ParsedBnkEntry> = Vec::new();
    let mut fit_profile: Option<String> = None;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let fields = parse_csv_fields(line);
        if fields.len() < 3 {
            continue;
        }

        let type_code = fields[0].trim().to_string();
        let level_code = fields[1].trim().to_string();
        let body = fields[2].to_string();

        if is_fit_sentinel(&type_code, &level_code) {
            fit_profile = extract_fit_profile(&body);
            continue;
        }

        entries.push(ParsedBnkEntry {
            sort_order: entries.len(),
            type_code,
            level_code,
            text: body,
        });
    }

    Ok(ParsedBnkFile {
        fit_profile,
        entries,
    })
}

pub struct ParsedTbkAssignment {
    pub item_id: String,
    pub note: String,
}

pub struct ParsedTbkItem {
    pub title: String,
    pub publisher: String,
    pub cost: f64,
    pub assignments: Vec<ParsedTbkAssignment>,
}

pub struct ParsedTbkFile {
    pub last_student: usize,
    pub items: Vec<ParsedTbkItem>,
}

pub fn parse_legacy_tbk_file(path: &Path) -> anyhow::Result<ParsedTbkFile> {
    let bytes = std::fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<String> = text
        .lines()
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect();

    let last_student_idx = find_section(&lines, "LastStudent")
        .ok_or_else(|| anyhow::anyhow!("missing [LastStudent] section"))?;
    let mut i = last_student_idx + 1;
    let last_student = next_non_noise(&lines, &mut i)
        .and_then(|s| s.parse::<usize>().ok())
        .ok_or_else(|| anyhow::anyhow!("missing [LastStudent] count"))?;

    let data_idx = lines
        .iter()
        .position(|l| {
            l.to_ascii_lowercase()
                .contains("[loaned items data - do not edit")
        })
        .ok_or_else(|| anyhow::anyhow!("missing [Loaned Items Data - DO NOT EDIT!!!] section"))?;
    let mut c = data_idx + 1;
    let item_count = next_non_noise(&lines, &mut c)
        .and_then(|s| s.parse::<usize>().ok())
        .ok_or_else(|| anyhow::anyhow!("missing TBK item count"))?;

    let next_raw = |idx: &mut usize| -> Option<String> {
        while *idx < lines.len() {
            let raw = lines[*idx].trim();
            *idx += 1;
            if raw.is_empty() {
                continue;
            }
            return Some(raw.to_string());
        }
        None
    };

    let mut items: Vec<ParsedTbkItem> = Vec::new();
    for _ in 0..=item_count {
        let item_line = next_raw(&mut c)
            .ok_or_else(|| anyhow::anyhow!("unexpected EOF reading TBK item header"))?;
        let fields = parse_csv_fields(&item_line);
        let title = fields.first().cloned().unwrap_or_default();
        let publisher = fields.get(1).cloned().unwrap_or_default();
        let cost = fields
            .get(2)
            .and_then(|v| v.trim().parse::<f64>().ok())
            .unwrap_or(0.0);

        let mut assignments: Vec<ParsedTbkAssignment> = Vec::with_capacity(last_student);
        for _ in 0..last_student {
            let assignment_line = next_raw(&mut c)
                .ok_or_else(|| anyhow::anyhow!("unexpected EOF reading TBK assignment"))?;
            let fields = parse_csv_fields(&assignment_line);
            assignments.push(ParsedTbkAssignment {
                item_id: fields.first().cloned().unwrap_or_default(),
                note: fields.get(1).cloned().unwrap_or_default(),
            });
        }

        items.push(ParsedTbkItem {
            title,
            publisher,
            cost,
            assignments,
        });
    }

    Ok(ParsedTbkFile {
        last_student,
        items,
    })
}

pub struct ParsedIccFile {
    pub last_student: usize,
    pub subject_count: usize,
    /// [row][subject], where row 0 is class defaults and rows 1..N are students.
    pub codes: Vec<Vec<String>>,
}

pub fn parse_legacy_icc_file(path: &Path) -> anyhow::Result<ParsedIccFile> {
    let bytes = std::fs::read(path)?;
    let text = String::from_utf8_lossy(&bytes);
    let lines: Vec<String> = text
        .lines()
        .map(|l| l.trim_end_matches('\r').to_string())
        .collect();
    if lines.is_empty() {
        return Err(anyhow::anyhow!("empty ICC file"));
    }

    let mut first_line_idx = 0usize;
    while first_line_idx < lines.len() && lines[first_line_idx].trim().is_empty() {
        first_line_idx += 1;
    }
    if first_line_idx >= lines.len() {
        return Err(anyhow::anyhow!("missing ICC header line"));
    }

    let first_fields = parse_csv_fields(lines[first_line_idx].trim());
    if first_fields.len() < 2 {
        return Err(anyhow::anyhow!("bad ICC header line"));
    }
    let last_student = first_fields[0]
        .trim()
        .parse::<usize>()
        .map_err(|_| anyhow::anyhow!("bad ICC student count"))?;
    let subject_count = first_fields[1]
        .trim()
        .parse::<usize>()
        .map_err(|_| anyhow::anyhow!("bad ICC subject count"))?;

    let expected_tokens = (last_student + 1) * (subject_count + 1);
    let mut tokens: Vec<String> = Vec::with_capacity(expected_tokens);
    for line in lines.iter().skip(first_line_idx + 1) {
        let raw = line.trim();
        if raw.is_empty() {
            continue;
        }
        let fields = parse_csv_fields(raw);
        if fields.is_empty() {
            continue;
        }
        for f in fields {
            tokens.push(f);
        }
    }
    if tokens.len() < expected_tokens {
        tokens.resize(expected_tokens, String::new());
    }

    let mut codes: Vec<Vec<String>> = Vec::with_capacity(last_student + 1);
    let mut cursor = 0usize;
    for _ in 0..=last_student {
        let mut row: Vec<String> = Vec::with_capacity(subject_count + 1);
        for _ in 0..=subject_count {
            row.push(tokens.get(cursor).cloned().unwrap_or_default());
            cursor += 1;
        }
        codes.push(row);
    }

    Ok(ParsedIccFile {
        last_student,
        subject_count,
        codes,
    })
}

pub fn serialize_bnk_file(parsed: &ParsedBnkFile) -> String {
    let mut out = String::new();
    for e in &parsed.entries {
        out.push_str(&format!(
            "{},{},{}\n",
            csv_quote(&e.type_code),
            csv_quote(&e.level_code),
            csv_quote(&e.text)
        ));
    }
    if let Some(fit) = parsed.fit_profile.as_deref() {
        out.push_str(&format!(
            "{},{},{}\n",
            csv_quote("FIT"),
            csv_quote("FIT"),
            csv_quote(&format!("Please DO NOT EDIT or DELETE this line: {}", fit))
        ));
    }
    out
}

pub struct ParsedLegacyExportBlock {
    pub title: String,
    pub out_of: f64,
    /// Raw values as exported by legacy report file.
    /// This usually includes one leading aggregate/sentinel row followed by student rows.
    pub values: Vec<f64>,
}

pub struct ParsedLegacyExportFile {
    pub last_student: usize,
    pub blocks: Vec<ParsedLegacyExportBlock>,
}

pub fn parse_legacy_export_file(path: &Path) -> anyhow::Result<ParsedLegacyExportFile> {
    let text = String::from_utf8_lossy(&std::fs::read(path)?).to_string();
    let lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();

    let mut i = 0usize;
    let mut last_student = 0usize;
    while i < lines.len() {
        let t = strip_quotes(lines[i].trim());
        if t.eq_ignore_ascii_case("[LastStudent]") {
            i += 1;
            while i < lines.len() {
                let n = strip_quotes(lines[i].trim());
                if n.is_empty() {
                    i += 1;
                    continue;
                }
                last_student = n.parse::<usize>().unwrap_or(0);
                break;
            }
            break;
        }
        i += 1;
    }
    if last_student == 0 {
        anyhow::bail!("missing [LastStudent] in export file");
    }

    let mut blocks: Vec<ParsedLegacyExportBlock> = Vec::new();
    let mut cursor = 0usize;
    while cursor < lines.len() {
        let line = strip_quotes(lines[cursor].trim());
        if line.is_empty() {
            cursor += 1;
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            cursor += 1;
            continue;
        }
        // Skip metadata/header lines.
        if line.contains("Folder:")
            || line.starts_with("Mark File:")
            || line.starts_with("This file belongs")
        {
            cursor += 1;
            continue;
        }

        // A block starts with title, then an "out_of,..." line, then numeric value lines.
        let title = line;
        cursor += 1;

        let mut out_of: Option<f64> = None;
        while cursor < lines.len() {
            let candidate = strip_quotes(lines[cursor].trim());
            cursor += 1;
            if candidate.is_empty() {
                continue;
            }
            if candidate.starts_with('[') && candidate.ends_with(']') {
                break;
            }
            let fields = parse_csv_fields(&candidate);
            if fields.is_empty() {
                continue;
            }
            if let Ok(n) = fields[0].trim().parse::<f64>() {
                out_of = Some(n);
                break;
            } else {
                // Not a valid block; keep scanning from this line as a potential new title.
                cursor = cursor.saturating_sub(1);
                break;
            }
        }
        let Some(out_of) = out_of else {
            continue;
        };

        let mut values: Vec<f64> = Vec::new();
        while cursor < lines.len() && values.len() < (last_student + 1) {
            let candidate = strip_quotes(lines[cursor].trim());
            if candidate.is_empty() {
                cursor += 1;
                continue;
            }
            if candidate.starts_with('[') && candidate.ends_with(']') {
                break;
            }
            if let Ok(v) = candidate.parse::<f64>() {
                values.push(v);
                cursor += 1;
                continue;
            }

            // This line is likely next block title; re-process in outer loop.
            break;
        }

        if !values.is_empty() {
            blocks.push(ParsedLegacyExportBlock {
                title,
                out_of,
                values,
            });
        }
    }

    Ok(ParsedLegacyExportFile {
        last_student,
        blocks,
    })
}

fn find_section(lines: &[String], name: &str) -> Option<usize> {
    let needle = format!("[{}]", name);
    for (i, l) in lines.iter().enumerate() {
        if l.trim().eq_ignore_ascii_case(&needle) {
            return Some(i);
        }
    }
    None
}

fn read_quoted_block(lines: &[String], i: &mut usize) -> anyhow::Result<String> {
    // Notes are stored as VB6-quoted strings, sometimes spanning multiple lines.
    // Each note begins with a line that starts with `"` and ends with a line that ends with `"`.
    while *i < lines.len() {
        let l0_raw = lines[*i].as_str();
        let l0 = l0_raw.trim();
        if l0.is_empty() {
            *i += 1;
            continue;
        }
        if !l0.starts_with('"') {
            // Unexpected noise line; skip.
            *i += 1;
            continue;
        }

        // Fast path: single-line quoted string.
        if l0.len() >= 2 && l0.ends_with('"') {
            *i += 1;
            return Ok(l0[1..l0.len() - 1].to_string());
        }

        // Multi-line quoted string.
        let mut buf = String::new();
        buf.push_str(&l0[1..]);
        *i += 1;

        while *i < lines.len() {
            let l = lines[*i].as_str();
            // Don't trim leading whitespace inside note content.
            let l = l.trim_end();
            *i += 1;

            if l.ends_with('"') {
                buf.push('\n');
                buf.push_str(&l[..l.len() - 1]);
                return Ok(buf.trim_end_matches('\n').to_string());
            }

            buf.push('\n');
            buf.push_str(l);
        }

        return Err(anyhow::anyhow!("unterminated quoted note"));
    }
    Err(anyhow::anyhow!("unexpected EOF reading notes"))
}

fn next_non_noise(lines: &[String], i: &mut usize) -> Option<String> {
    while *i < lines.len() {
        let t = lines[*i].trim();
        *i += 1;
        if t.is_empty() {
            continue;
        }
        let v = strip_quotes(t);
        if v.is_empty() {
            continue;
        }
        return Some(v);
    }
    None
}

fn next_keep_empty(lines: &[String], i: &mut usize) -> Option<String> {
    if *i >= lines.len() {
        return None;
    }
    let t = lines[*i].trim();
    *i += 1;
    Some(strip_quotes(t))
}

fn parse_date_ymd(s: &str) -> Option<String> {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 3 {
        return None;
    }
    let y = parts[0].parse::<i32>().ok()?;
    let m = parts[1].parse::<i32>().ok()?;
    let d = parts[2].parse::<i32>().ok()?;
    Some(format!("{:04}-{:02}-{:02}", y, m, d))
}

fn parse_csv_numbers(s: &str, expected: usize) -> Option<Vec<f64>> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() < expected {
        return None;
    }
    let mut out: Vec<f64> = Vec::new();
    for p in parts.into_iter().take(expected) {
        out.push(p.trim().parse::<f64>().ok()?);
    }
    Some(out)
}

fn parse_csv_i32(s: &str, expected: usize) -> Option<Vec<i32>> {
    let fields = parse_csv_fields(s);
    if fields.len() < expected {
        return None;
    }
    let mut out: Vec<i32> = Vec::with_capacity(expected);
    for item in fields.into_iter().take(expected) {
        out.push(item.trim().parse::<i32>().ok()?);
    }
    Some(out)
}

fn parse_csv_fields(line: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut buf = String::new();
    let mut in_quotes = false;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let ch = chars[i];
        if ch == '"' {
            if in_quotes && i + 1 < chars.len() && chars[i + 1] == '"' {
                buf.push('"');
                i += 2;
                continue;
            }
            in_quotes = !in_quotes;
            i += 1;
            continue;
        }
        if ch == ',' && !in_quotes {
            out.push(buf.trim().to_string());
            buf.clear();
            i += 1;
            continue;
        }
        buf.push(ch);
        i += 1;
    }
    out.push(buf.trim().to_string());
    out
}

fn csv_quote(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\"\""))
}

fn normalize_fit_token(s: &str) -> String {
    s.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase()
}

fn is_fit_sentinel(type_code: &str, level_code: &str) -> bool {
    normalize_fit_token(type_code) == "FIT" && normalize_fit_token(level_code) == "FIT"
}

fn extract_fit_profile(s: &str) -> Option<String> {
    let mut out = s.trim().to_string();
    if let Some(pos) = out.find(':') {
        out = out[(pos + 1)..].trim().to_string();
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_path(rel: &str) -> PathBuf {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        base.join("../../").join(rel)
    }

    #[test]
    fn parse_cl_includes_mark_sets() {
        let p = fixture_path("fixtures/legacy/Sample25/MB8D25/CL8D.Y25");
        let cl = parse_legacy_cl(&p).expect("parse cl");
        assert_eq!(cl.mark_sets.len(), 6);
        assert!(cl.mark_sets.iter().any(|m| m.code == "MAT1"));

        // Membership mask is the trailing field in the legacy class list line.
        let tam = cl
            .students
            .iter()
            .find(|s| s.last_name == "O'Shanter" && s.first_name == "Tam")
            .expect("Tam present");
        assert_eq!(tam.mark_set_mask.as_deref(), Some("111111"));

        let melody = cl
            .students
            .iter()
            .find(|s| s.last_name == "Lyons" && s.first_name == "Melody")
            .expect("Melody present");
        assert_eq!(melody.mark_set_mask.as_deref(), Some("000000"));
    }

    #[test]
    fn parse_mat18d_mark_file() {
        let p = fixture_path("fixtures/legacy/Sample25/MB8D25/MAT18D.Y25");
        let mf = parse_legacy_mark_file(&p).expect("parse mark file");
        assert!(mf.misc.is_some());
        assert_eq!(mf.categories.len(), 5);
        assert_eq!(mf.last_student, 27);
        assert_eq!(mf.assessments.len(), 18);

        // First assessment, first student raw score should be 2.
        let a0 = &mf.assessments[0];
        assert_eq!(a0.raw_scores.len(), 27);
        assert_eq!(a0.raw_scores[0], LegacyScore::Scored(2.0));

        // Ensure we detect both legacy NoMark (raw==0) and Zero (raw<0) sentinels.
        assert!(a0.raw_scores.iter().any(|v| *v == LegacyScore::NoMark));
        assert!(a0.raw_scores.iter().any(|v| *v == LegacyScore::Zero));
    }

    #[test]
    fn parse_mat18d_typ_file() {
        let p = fixture_path("fixtures/legacy/Sample25/MB8D25/MAT18D.TYP");
        let v = parse_legacy_typ_file(&p).expect("parse typ");
        assert_eq!(v.len(), 18);
    }

    #[test]
    fn parse_mat18d_rmk_file() {
        let p = fixture_path("fixtures/legacy/Sample25/MB8D25/MAT18D.RMK");
        let r = parse_legacy_rmk_file(&p).expect("parse rmk");
        assert_eq!(r.last_student, 27);
        assert_eq!(r.last_entry, 18);
        assert_eq!(r.entry_titles.len(), 18);
        assert_eq!(r.remarks_by_entry.len(), 18);
        assert_eq!(r.remarks_by_entry[0].len(), 27);
    }

    #[test]
    fn parse_class_note_file() {
        let p = fixture_path("fixtures/legacy/Sample25/MB8D25/8DNOTE.TXT");
        let v = parse_legacy_note_file(&p).expect("parse notes");
        assert_eq!(v.len(), 27);
        assert!(v[0].contains("called re Math"));
    }

    #[test]
    fn parse_legacy_spl_file() {
        let p = fixture_path("fixtures/legacy/Sample25/MB8D25/8D.SPL");
        let s = parse_legacy_seating_file(&p).expect("parse spl");
        assert_eq!(s.rows, 6);
        assert_eq!(s.seats_per_row, 5);
        assert_eq!(s.last_student, 27);
        assert_eq!(s.seat_codes.len(), 27);
        assert_eq!(s.blocked_mask.len(), 100);
        assert!(s.blocked_mask.chars().all(|ch| ch == '0' || ch == '1'));
    }

    #[test]
    fn parse_legacy_idx_file_new_format() {
        let p = fixture_path("fixtures/legacy/Sample25/MB8D25/MAT18D.IDX");
        let idx = parse_legacy_idx_file(&p).expect("parse idx");
        assert_eq!(idx.sets.len(), 1);
        assert_eq!(idx.default_set, 1);
        assert_eq!(idx.sets[0].title, "First MAT1 Set");
        assert_eq!(idx.sets[0].fit_mode, 1);
        assert_eq!(idx.sets[0].fit_font_size, 9);
        assert_eq!(idx.sets[0].fit_width, 83);
        assert_eq!(idx.sets[0].fit_lines, 12);
        assert_eq!(idx.sets[0].max_chars, 100);
        assert_eq!(idx.bank_short.as_deref(), Some("COMMENT.BNK"));
    }

    #[test]
    fn parse_legacy_r_comment_file_fixture() {
        let p = fixture_path("fixtures/legacy/Sample25/MB8D25/MAT18D.R1");
        let parsed = parse_legacy_r_comment_file(&p).expect("parse r1");
        assert_eq!(parsed.last_student, 27);
        assert_eq!(parsed.remarks.len(), 27);
        assert!(parsed.remarks.iter().any(|s| s.contains("Daniella")));
    }

    #[test]
    fn parse_legacy_bnk_file() {
        let p = fixture_path("fixtures/legacy/Sample25/COMMENT.BNK");
        let parsed = parse_bnk_file(&p).expect("parse bnk");
        assert!(!parsed.entries.is_empty());
        assert!(
            parsed
                .fit_profile
                .as_deref()
                .unwrap_or_default()
                .contains("DO NOT EDIT")
                || parsed.fit_profile.is_some()
        );
        let serialized = serialize_bnk_file(&parsed);
        assert!(serialized.contains("FIT"));
    }

    #[test]
    fn parse_legacy_tbk_file_fixture() {
        let p = fixture_path("fixtures/legacy/Sample25/MB8D25/MAT18D.TBK");
        let parsed = parse_legacy_tbk_file(&p).expect("parse tbk");
        assert_eq!(parsed.last_student, 27);
        assert_eq!(parsed.items.len(), 1);
        assert_eq!(parsed.items[0].title, "Mathpower 8");
        assert_eq!(parsed.items[0].assignments.len(), 27);
        assert_eq!(
            parsed.items[0].assignments[0].item_id.to_uppercase(),
            "W98-102"
        );
    }

    #[test]
    fn parse_legacy_icc_file_fixture() {
        let p = fixture_path("fixtures/legacy/Sample25/MB8D25/8D.ICC");
        let parsed = parse_legacy_icc_file(&p).expect("parse icc");
        assert_eq!(parsed.last_student, 27);
        assert_eq!(parsed.subject_count, 6);
        assert_eq!(parsed.codes.len(), 28);
        assert_eq!(parsed.codes[0].len(), 7);
        assert_eq!(parsed.codes[0][1], "MAT2D1-01");
    }

    #[test]
    fn parse_legacy_attendance_file_from_synthetic() {
        let tmp = std::env::temp_dir().join(format!(
            "markbook-attendance-{}.ATN",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let mut src = String::new();
        src.push_str("[MarkBook]\n[Version]\n\"\"\n");
        src.push_str("[This Attendance File belongs to...]\nFolder: TEST\n\"\"\n");
        src.push_str("[LastStudent]\n2\n\"\"\n");
        src.push_str("[School Year Starts]\n9\n\"\"\n");
        src.push_str("[Attendance Data - DO NOT EDIT!!!]\n");
        for m in 1..=12 {
            src.push_str(&format!("\"[Month{}]\"\n", m));
            src.push_str("\"PPPP\"\n");
            src.push_str("\"PA\"\n");
            src.push_str("\"LP\"\n");
        }
        fs::write(&tmp, src).expect("write tmp atn");
        let parsed = parse_legacy_attendance_file(&tmp).expect("parse atn");
        let _ = fs::remove_file(&tmp);
        assert_eq!(parsed.last_student, 2);
        assert_eq!(parsed.school_year_start_month, 9);
        assert_eq!(parsed.months.len(), 12);
        assert_eq!(parsed.months[0].student_day_codes.len(), 2);
        assert_eq!(parsed.months[0].type_of_day_codes, "PPPP");
    }

    #[test]
    fn parse_legacy_export_mat18d_13() {
        let p = fixture_path("fixtures/legacy/Sample25/MB8D25/MAT18D.13");
        let parsed = parse_legacy_export_file(&p).expect("parse export");
        assert_eq!(parsed.last_student, 27);
        assert!(!parsed.blocks.is_empty());
        assert_eq!(parsed.blocks[0].title, "Group Report");
        assert_eq!(parsed.blocks[0].out_of, 100.0);
        assert_eq!(parsed.blocks[0].values.len(), 27);
    }

    #[test]
    fn parse_legacy_export_snc28d_15() {
        let p = fixture_path("fixtures/legacy/Sample25/MB8D25/SNC28D.15");
        let parsed = parse_legacy_export_file(&p).expect("parse export");
        assert_eq!(parsed.last_student, 27);
        assert!(parsed.blocks.len() >= 3);
        assert_eq!(parsed.blocks[0].title, "True / False");
        assert_eq!(parsed.blocks[0].out_of, 9.0);
    }
}
