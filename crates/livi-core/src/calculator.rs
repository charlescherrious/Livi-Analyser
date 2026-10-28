//! Tempo-synced timing maths for the delay and reverb tables. Mirrors
//! `src/calculator.ts` in the desktop app. Assumes 4/4 (one bar = 4 beats).

pub const BEATS_PER_BAR: f64 = 4.0;
pub const MIN_BPM: f64 = 20.0;
pub const MAX_BPM: f64 = 400.0;

pub fn is_valid_bpm(bpm: f64) -> bool {
    bpm.is_finite() && (MIN_BPM..=MAX_BPM).contains(&bpm)
}

/// Length of one quarter-note beat in milliseconds.
pub fn beat_ms(bpm: f64) -> f64 {
    60_000.0 / bpm
}

pub fn bar_ms(bpm: f64) -> f64 {
    beat_ms(bpm) * BEATS_PER_BAR
}

pub fn ms_to_hz(ms: f64) -> f64 {
    1000.0 / ms
}

/// Round to two decimals, the precision most delay plugins accept.
pub fn round_bpm(bpm: f64) -> f64 {
    (bpm * 100.0).round() / 100.0
}

pub fn halve(bpm: f64) -> f64 {
    round_bpm(bpm / 2.0)
}
pub fn double(bpm: f64) -> f64 {
    round_bpm(bpm * 2.0)
}
/// 135 -> 90: fixes a triplet pulse read as the beat.
pub fn two_thirds(bpm: f64) -> f64 {
    round_bpm(bpm * 2.0 / 3.0)
}
/// 90 -> 135.
pub fn three_halves(bpm: f64) -> f64 {
    round_bpm(bpm * 1.5)
}
/// True when two tempos are a 3:2 pair within 3% (e.g. 90 and 135).
pub fn is_three_two(a: f64, b: f64) -> bool {
    let r = a.max(b) / a.min(b);
    (r / 1.5 - 1.0).abs() <= 0.03
}

/// One delay row: note length in ms as straight, dotted and triplet.
#[derive(Debug, Clone, PartialEq)]
pub struct DelayRow {
    pub label: &'static str,
    pub straight: f64,
    pub dotted: f64,
    pub triplet: f64,
}

/// A pre-delay or decay suggestion.
#[derive(Debug, Clone, PartialEq)]
pub struct ReverbRow {
    pub label: &'static str,
    pub hint: &'static str,
    pub ms: f64,
}

const NOTE_VALUES: [(&str, f64); 7] = [
    ("1/1", 4.0),
    ("1/2", 2.0),
    ("1/4", 1.0),
    ("1/8", 0.5),
    ("1/16", 0.25),
    ("1/32", 0.125),
    ("1/64", 0.0625),
];

const PRE_DELAY: [(&str, f64, &str); 4] = [
    ("1/64", 1.0 / 16.0, "Very tight"),
    ("1/32", 1.0 / 8.0, "Tight"),
    ("1/16 triplet", 0.25 * 2.0 / 3.0, "Light gap"),
    ("1/16", 0.25, "Clear gap"),
];

const DECAY: [(&str, f64, &str); 5] = [
    ("1/8", 0.5, "Very short"),
    ("1/4", 1.0, "Short"),
    ("1/2", 2.0, "Medium"),
    ("1 bar", 4.0, "Long"),
    ("2 bars", 8.0, "Very long"),
];

pub fn delay_rows(bpm: f64) -> Vec<DelayRow> {
    let beat = beat_ms(bpm);
    NOTE_VALUES
        .iter()
        .map(|&(label, beats)| DelayRow {
            label,
            straight: beat * beats,
            dotted: beat * beats * 1.5,
            triplet: beat * beats * 2.0 / 3.0,
        })
        .collect()
}

pub fn pre_delay_rows(bpm: f64) -> Vec<ReverbRow> {
    let beat = beat_ms(bpm);
    PRE_DELAY
        .iter()
        .map(|&(label, beats, hint)| ReverbRow { label, hint, ms: beat * beats })
        .collect()
}

pub fn decay_rows(bpm: f64) -> Vec<ReverbRow> {
    let beat = beat_ms(bpm);
    DECAY
        .iter()
        .map(|&(label, beats, hint)| ReverbRow { label, hint, ms: beat * beats })
        .collect()
}

pub fn format_ms(ms: f64) -> String {
    if ms >= 1000.0 {
        format!("{:.0}", ms)
    } else if ms >= 100.0 {
        format!("{:.1}", ms)
    } else {
        format!("{:.2}", ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarter_note_at_120_is_500ms() {
        assert_eq!(beat_ms(120.0), 500.0);
        assert_eq!(bar_ms(120.0), 2000.0);
    }

    #[test]
    fn dotted_and_triplet_eighths_at_120() {
        let rows = delay_rows(120.0);
        let eighth = rows.iter().find(|r| r.label == "1/8").unwrap();
        assert_eq!(eighth.straight, 250.0);
        assert_eq!(eighth.dotted, 375.0);
        assert!((eighth.triplet - 166.6667).abs() < 1e-3);
    }

    #[test]
    fn reverb_rows_follow_the_beat() {
        let decay = decay_rows(100.0); // beat = 600 ms
        assert_eq!(decay.iter().find(|r| r.label == "1 bar").unwrap().ms, 2400.0);
        let pre = pre_delay_rows(100.0);
        assert!((pre[0].ms - 37.5).abs() < 1e-9);
    }

    #[test]
    fn three_two_corrections() {
        assert_eq!(two_thirds(135.0), 90.0);
        assert_eq!(three_halves(90.0), 135.0);
        assert!(is_three_two(144.0, 96.0));
        assert!(!is_three_two(140.0, 70.0));
        assert_eq!(halve(149.34), 74.67);
        assert_eq!(double(74.67), 149.34);
    }

    #[test]
    fn bpm_validity() {
        assert!(is_valid_bpm(90.0));
        assert!(!is_valid_bpm(f64::NAN));
        assert!(!is_valid_bpm(5.0));
    }
}
