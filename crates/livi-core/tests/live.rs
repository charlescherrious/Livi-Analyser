//! The plugin hears audio at the host rate (44.1/48/96 kHz), not at the
//! engine rate. These tests make sure analysis still works.

use livi_core::{analyze_live, AnalysisOptions};
use std::sync::atomic::AtomicBool;

/// Click + chord signal at the given rate: 124 BPM, A minor.
fn signal(sr: u32, secs: f32) -> Vec<f32> {
    let n = (secs * sr as f32) as usize;
    let beat = 60.0 / 124.0;
    (0..n)
        .map(|i| {
            let t = i as f32 / sr as f32;
            let since = t % beat;
            let kick = 0.6 * (2.0 * std::f32::consts::PI * 55.0 * t).sin() * (-12.0 * since).exp();
            let chord: f32 = [220.0f32, 261.63, 329.63]
                .iter()
                .map(|f| 0.15 * (2.0 * std::f32::consts::PI * f * t).sin())
                .sum();
            kick + chord
        })
        .collect()
}

#[test]
fn live_audio_at_common_host_rates() {
    for sr in [44_100u32, 48_000, 96_000] {
        let audio = signal(sr, 30.0);
        let r = analyze_live(&audio, sr, &AnalysisOptions::default(), &AtomicBool::new(false), &|_| {})
            .unwrap();
        assert!((r.bpm_raw - 124.0).abs() < 0.3, "sr {sr}: bpm {}", r.bpm_raw);
        assert_eq!(r.key.camelot, "8A", "sr {sr}: key {}", r.key.name);
        assert_eq!(r.source_sample_rate, sr);
        assert!((r.duration_secs - 30.0).abs() < 0.01);
    }
}

#[test]
fn live_audio_too_short_is_rejected() {
    let audio = signal(48_000, 2.0);
    assert!(analyze_live(&audio, 48_000, &AnalysisOptions::default(), &AtomicBool::new(false), &|_| {}).is_err());
}
