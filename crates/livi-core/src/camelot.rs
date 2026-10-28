//! Key naming: standard notation and Camelot wheel codes.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Major,
    Minor,
}

const MAJOR_NAMES: [&str; 12] = ["C", "Db", "D", "Eb", "E", "F", "F#", "G", "Ab", "A", "Bb", "B"];
const MINOR_NAMES: [&str; 12] = ["C", "C#", "D", "Eb", "E", "F", "F#", "G", "G#", "A", "Bb", "B"];

/// Camelot numbers indexed by tonic pitch class (C = 0).
const CAMELOT_MAJOR: [u8; 12] = [8, 3, 10, 5, 12, 7, 2, 9, 4, 11, 6, 1];
const CAMELOT_MINOR: [u8; 12] = [5, 12, 7, 2, 9, 4, 11, 6, 1, 8, 3, 10];

pub fn key_name(tonic: u8, mode: Mode) -> String {
    let t = (tonic % 12) as usize;
    match mode {
        Mode::Major => format!("{} major", MAJOR_NAMES[t]),
        Mode::Minor => format!("{} minor", MINOR_NAMES[t]),
    }
}

/// Short DJ-style name, e.g. "Fm" or "Bb".
pub fn key_short(tonic: u8, mode: Mode) -> String {
    let t = (tonic % 12) as usize;
    match mode {
        Mode::Major => MAJOR_NAMES[t].to_string(),
        Mode::Minor => format!("{}m", MINOR_NAMES[t]),
    }
}

/// Camelot code, e.g. "8B" for C major and "8A" for A minor.
pub fn camelot(tonic: u8, mode: Mode) -> String {
    let t = (tonic % 12) as usize;
    match mode {
        Mode::Major => format!("{}B", CAMELOT_MAJOR[t]),
        Mode::Minor => format!("{}A", CAMELOT_MINOR[t]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spot_checks() {
        assert_eq!(camelot(0, Mode::Major), "8B"); // C major
        assert_eq!(camelot(9, Mode::Minor), "8A"); // A minor
        assert_eq!(camelot(5, Mode::Minor), "4A"); // F minor
        assert_eq!(camelot(3, Mode::Major), "5B"); // Eb major
        assert_eq!(camelot(6, Mode::Major), "2B"); // F# major
        assert_eq!(camelot(6, Mode::Minor), "11A"); // F# minor
        assert_eq!(key_name(5, Mode::Minor), "F minor");
        assert_eq!(key_short(10, Mode::Minor), "Bbm");
    }

    #[test]
    fn relative_keys_share_a_number() {
        // Relative major/minor pairs sit on the same Camelot number.
        for tonic in 0..12u8 {
            let minor_tonic = (tonic + 9) % 12;
            let maj = camelot(tonic, Mode::Major);
            let min = camelot(minor_tonic, Mode::Minor);
            assert_eq!(&maj[..maj.len() - 1], &min[..min.len() - 1]);
        }
    }
}
