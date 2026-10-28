//! State shared between the audio thread, the analysis thread and the GUI.
//!
//! The audio thread must never block, allocate or lock, so the capture
//! buffer is a fixed array of atomics that it writes into, and everything the
//! GUI needs to change is an atomic flag.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;

use livi_core::{AnalysisOptions, AnalysisResult};

/// Longest recording we keep: 60 s at up to 96 kHz.
pub const MAX_CAPTURE_SAMPLES: usize = 96_000 * 60;
/// Shortest recording worth analysing (the engine needs 5 s; we ask for more).
pub const MIN_LISTEN_SECS: f32 = 8.0;
/// Recording stops by itself after this long.
pub const MAX_LISTEN_SECS: f32 = 60.0;

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Idle,
    Listening,
    Analysing(String),
    Done,
    Failed(String),
}

pub struct Preset {
    pub label: &'static str,
    pub min: f64,
    pub max: f64,
}

pub const PRESETS: [Preset; 5] = [
    Preset { label: "Auto (70-180)", min: 70.0, max: 180.0 },
    Preset { label: "Afrobeats (80-125)", min: 80.0, max: 125.0 },
    Preset { label: "Dancehall (80-115)", min: 80.0, max: 115.0 },
    Preset { label: "Amapiano (105-125)", min: 105.0, max: 125.0 },
    Preset { label: "Trap (125-175)", min: 125.0, max: 175.0 },
];

/// Everything the GUI edits or displays. Locked only briefly, never from the
/// audio thread.
pub struct UiState {
    pub status: Status,
    pub result: Option<AnalysisResult>,
    /// BPM the user chose by hand (buttons, typing, candidate chips).
    pub bpm_override: Option<f64>,
    pub preset: usize,
}

pub struct Shared {
    capture: Vec<AtomicU32>,
    pub write_pos: AtomicUsize,
    pub listening: AtomicBool,
    pub sample_rate: AtomicU32,
    pub host_playing: AtomicBool,
    host_tempo_bits: AtomicU64,
    pub cancel: AtomicBool,
    pub ui: Mutex<UiState>,
}

impl Shared {
    pub fn new() -> Self {
        Shared {
            capture: (0..MAX_CAPTURE_SAMPLES).map(|_| AtomicU32::new(0)).collect(),
            write_pos: AtomicUsize::new(0),
            listening: AtomicBool::new(false),
            sample_rate: AtomicU32::new(44_100),
            host_playing: AtomicBool::new(false),
            host_tempo_bits: AtomicU64::new(f64::NAN.to_bits()),
            cancel: AtomicBool::new(false),
            ui: Mutex::new(UiState {
                status: Status::Idle,
                result: None,
                bpm_override: None,
                preset: 0,
            }),
        }
    }

    pub fn set_host_tempo(&self, tempo: Option<f64>) {
        self.host_tempo_bits
            .store(tempo.unwrap_or(f64::NAN).to_bits(), Ordering::Relaxed);
    }

    pub fn host_tempo(&self) -> Option<f64> {
        let t = f64::from_bits(self.host_tempo_bits.load(Ordering::Relaxed));
        t.is_finite().then_some(t)
    }

    /// How many samples we can hold at the current sample rate.
    pub fn capacity(&self) -> usize {
        let sr = self.sample_rate.load(Ordering::Relaxed) as f32;
        ((MAX_LISTEN_SECS * sr) as usize).min(MAX_CAPTURE_SAMPLES)
    }

    /// Audio thread: add one mono sample if we are recording. Never blocks.
    #[inline]
    pub fn push(&self, sample: f32) {
        let pos = self.write_pos.load(Ordering::Relaxed);
        if pos < self.capacity() {
            self.capture[pos].store(sample.to_bits(), Ordering::Relaxed);
            self.write_pos.store(pos + 1, Ordering::Release);
        }
    }

    pub fn start_listening(&self) {
        self.write_pos.store(0, Ordering::SeqCst);
        self.listening.store(true, Ordering::SeqCst);
    }

    pub fn stop_listening(&self) {
        self.listening.store(false, Ordering::SeqCst);
    }

    pub fn listened_secs(&self) -> f32 {
        let sr = self.sample_rate.load(Ordering::Relaxed).max(1) as f32;
        self.write_pos.load(Ordering::Acquire) as f32 / sr
    }

    /// Copy of what has been recorded so far.
    pub fn snapshot(&self) -> Vec<f32> {
        let n = self.write_pos.load(Ordering::Acquire).min(self.capture.len());
        self.capture[..n]
            .iter()
            .map(|a| f32::from_bits(a.load(Ordering::Relaxed)))
            .collect()
    }

    pub fn range(&self) -> AnalysisOptions {
        let preset = self.ui.lock().map(|u| u.preset).unwrap_or(0);
        let p = &PRESETS[preset.min(PRESETS.len() - 1)];
        AnalysisOptions { min_bpm: p.min, max_bpm: p.max }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_records_and_stops_when_full() {
        let s = Shared::new();
        s.sample_rate.store(48_000, Ordering::Relaxed);
        for i in 0..100 {
            s.push(i as f32);
        }
        let snap = s.snapshot();
        assert_eq!(snap.len(), 100);
        assert_eq!(snap[42], 42.0);

        // Fill past capacity: extra samples are ignored, not a crash.
        let cap = s.capacity();
        for _ in 0..(cap + 1000) {
            s.push(0.5);
        }
        assert_eq!(s.snapshot().len(), cap);
    }

    #[test]
    fn restart_clears_the_recording() {
        let s = Shared::new();
        for _ in 0..500 {
            s.push(1.0);
        }
        s.start_listening();
        assert_eq!(s.snapshot().len(), 0);
        assert!(s.listening.load(Ordering::SeqCst));
    }

    #[test]
    fn host_tempo_roundtrip() {
        let s = Shared::new();
        assert_eq!(s.host_tempo(), None);
        s.set_host_tempo(Some(98.5));
        assert_eq!(s.host_tempo(), Some(98.5));
    }

    #[test]
    fn preset_range_is_used() {
        let s = Shared::new();
        s.ui.lock().unwrap().preset = 4;
        let o = s.range();
        assert_eq!((o.min_bpm, o.max_bpm), (125.0, 175.0));
    }
}
