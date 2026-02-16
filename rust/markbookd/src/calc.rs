#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScoreState {
    NoMark,
    Zero,
    Scored(f64),
}

/// VB6-compatible 1-decimal rounding used in MarkBook:
/// `Int(10*x + 0.5) / 10`
#[allow(dead_code)]
pub fn round_off_1_decimal(x: f64) -> f64 {
    ((10.0 * x) + 0.5).floor() / 10.0
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssessmentAverage {
    pub avg_raw: f64,
    pub avg_percent: f64,
    pub scored_count: usize,
    pub zero_count: usize,
    pub no_mark_count: usize,
}

pub fn assessment_average<I>(scores: I, out_of: f64) -> AssessmentAverage
where
    I: IntoIterator<Item = ScoreState>,
{
    let mut denom: usize = 0;
    let mut sum_raw: f64 = 0.0;
    let mut scored_count: usize = 0;
    let mut zero_count: usize = 0;
    let mut no_mark_count: usize = 0;

    for s in scores {
        match s {
            ScoreState::NoMark => {
                no_mark_count += 1;
            }
            ScoreState::Zero => {
                zero_count += 1;
                denom += 1;
            }
            ScoreState::Scored(v) => {
                scored_count += 1;
                denom += 1;
                sum_raw += v;
            }
        }
    }

    let avg_raw = if denom > 0 {
        sum_raw / (denom as f64)
    } else {
        0.0
    };
    let avg_percent = if out_of > 0.0 {
        100.0 * avg_raw / out_of
    } else {
        0.0
    };

    AssessmentAverage {
        avg_raw,
        avg_percent,
        scored_count,
        zero_count,
        no_mark_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy::{parse_legacy_mark_file, LegacyScore};
    use std::path::PathBuf;

    fn fixture_path(rel: &str) -> PathBuf {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        base.join("../../").join(rel)
    }

    #[test]
    fn round_off_matches_vb6() {
        assert_eq!(round_off_1_decimal(0.0), 0.0);
        assert_eq!(round_off_1_decimal(3.54), 3.5);
        assert_eq!(round_off_1_decimal(3.55), 3.6);
        assert_eq!(round_off_1_decimal(35.6818), 35.7);
    }

    #[test]
    fn assessment_average_counts_no_mark_vs_zero() {
        let p = fixture_path("fixtures/legacy/Sample25/MB8D25/MAT18D.Y25");
        let mf = parse_legacy_mark_file(&p).expect("parse mark file");
        let a0 = &mf.assessments[0];

        let avg = assessment_average(
            a0.raw_scores.iter().copied().map(|s| match s {
                LegacyScore::NoMark => ScoreState::NoMark,
                LegacyScore::Zero => ScoreState::Zero,
                LegacyScore::Scored(v) => ScoreState::Scored(v),
            }),
            a0.out_of,
        );

        assert_eq!(avg.no_mark_count, 5);
        assert_eq!(avg.zero_count, 2);
        assert_eq!(avg.scored_count, 20);

        // VB6 Calculate semantics: denom counts all non-NoMark rows (Scored + Zero).
        // avg_raw here is computed from the fixture's raw values, not the file summary line.
        let expected_avg_raw = 78.5 / 22.0;
        assert!((avg.avg_raw - expected_avg_raw).abs() < 1e-9);
    }
}
