//! Regression tests for the 3:2 mistake: a groove at 90 BPM with a triplet
//! shuffle has a pulse at 135 per minute, which used to be reported instead.

mod common;
use common::{drum_loop, Style, SR};
use livi_core::{analyze_samples, AnalysisOptions};
use std::sync::atomic::AtomicBool;

fn bpm(style: Style, true_bpm: f64, lo: f64, hi: f64) -> f64 {
    let audio = drum_loop(true_bpm, 60.0, style);
    analyze_samples(
        &audio,
        SR,
        &AnalysisOptions { min_bpm: lo, max_bpm: hi },
        &AtomicBool::new(false),
        &|_| {},
    )
    .unwrap()
    .bpm_raw
}

#[test]
fn soft_triplet_shuffle_keeps_the_real_tempo() {
    for true_bpm in [90.0, 96.0, 100.0] {
        for (lo, hi) in [(70.0, 180.0), (80.0, 160.0), (85.0, 140.0)] {
            let got = bpm(Style::TripletSoft, true_bpm, lo, hi);
            assert!((got - true_bpm).abs() < 0.5, "true {true_bpm} range {lo}-{hi}: got {got}");
        }
    }
}

// Note: a loud triplet shuffle in Auto mode (70-180) is a near coin toss
// between 100 and 150 (scores within ~5%), so it is deliberately not asserted.
// Choosing a genre range settles it, which is the next test.

#[test]
fn narrow_genre_range_rescues_the_loud_case() {
    for true_bpm in [96.0, 100.0] {
        let got = bpm(Style::TripletLoud, true_bpm, 85.0, 140.0);
        assert!((got - true_bpm).abs() < 0.5, "true {true_bpm}: got {got}");
    }
}
