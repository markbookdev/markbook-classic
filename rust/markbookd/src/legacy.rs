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

    Some(ParsedStudent {
        active,
        last_name,
        first_name,
        student_no,
        birth_date,
        raw_line: raw,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
