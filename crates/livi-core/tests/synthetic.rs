//! Sanity tests on synthetic audio. These prove the plumbing and basic
//! correctness; accuracy on real music must be measured with a labelled
//! benchmark (see README).

mod common;

use std::sync::atomic::AtomicBool;

use common::*;
use livi_core::{analyze_samples, AnalysisOptions};

fn analyse(samples: &[f32], min: f64, max: f64) -> livi_core::AnalysisResult {
    let cancel = AtomicBool::new(false);
    analyze_samples(
        samples,
        SR,
        &AnalysisOptions {
            min_bpm: min,
            max_bpm: max,
        },
        &cancel,
        &|_| {},
    )
    .expect("analysis should succeed")
}

fn check_bpm(style: Style, bpm: f64, min: f64, max: f64, secs: f64, tol: f64) {
    let audio = drum_loop(bpm, secs, style);
    let r = analyse(&audio, min, max);
    println!("true {bpm} -> raw {:.3}, reported {}, conf {:.2}", r.bpm_raw, r.bpm, r.bpm_confidence);
    assert!(
        (r.bpm_raw - bpm).abs() <= tol,
        "expected {bpm} ± {tol}, measured {:.3} (candidates {:?})",
        r.bpm_raw,
        r.bpm_candidates
    );
}

#[test]
fn straight_loops_at_common_tempos() {
    for bpm in [90.0, 100.0, 112.0, 120.0, 128.0, 150.0] {
        check_bpm(Style::Straight, bpm, 80.0, 160.0, 60.0, 0.15);
    }
}

#[test]
fn fractional_tempo_is_resolved() {
    check_bpm(Style::Straight, 97.5, 80.0, 160.0, 60.0, 0.15);
    check_bpm(Style::Straight, 103.7, 80.0, 160.0, 60.0, 0.2);
}

#[test]
fn afrobeats_pattern() {
    check_bpm(Style::Afrobeats, 102.0, 90.0, 125.0, 60.0, 0.2);
    check_bpm(Style::Afrobeats, 108.0, 90.0, 125.0, 60.0, 0.2);
}

#[test]
fn amapiano_pattern() {
    check_bpm(Style::Amapiano, 112.0, 105.0, 125.0, 60.0, 0.2);
    check_bpm(Style::Amapiano, 115.0, 105.0, 125.0, 60.0, 0.2);
}

#[test]
fn trap_pattern_uses_producer_bpm_not_half_time() {
    // The preset range (125-175) is what keeps this at 140 and not 70.
    check_bpm(Style::Trap, 140.0, 125.0, 175.0, 60.0, 0.3);
    check_bpm(Style::Trap, 150.0, 125.0, 175.0, 60.0, 0.3);
}

#[test]
fn result_reports_confidence_and_candidates() {
    let r = analyse(&drum_loop(120.0, 40.0, Style::Straight), 70.0, 180.0);
    assert!(r.bpm_confidence > 0.0);
    assert!(!r.bpm_candidates.is_empty());
    assert!((r.bpm_candidates[0].score - 1.0).abs() < 1e-9);
    assert!(!r.variable_tempo);
}

#[test]
fn too_short_audio_is_rejected() {
    let cancel = AtomicBool::new(false);
    let audio = drum_loop(120.0, 2.0, Style::Straight);
    let err = analyze_samples(&audio, SR, &AnalysisOptions::default(), &cancel, &|_| {});
    assert!(matches!(err, Err(livi_core::AnalysisError::TooShort(_))));
}

#[test]
fn silence_is_reported_not_guessed() {
    let cancel = AtomicBool::new(false);
    let audio = vec![0.0f32; SR as usize * 20];
    let err = analyze_samples(&audio, SR, &AnalysisOptions::default(), &cancel, &|_| {});
    assert!(matches!(err, Err(livi_core::AnalysisError::NothingDetected)));
}

#[test]
fn cancel_flag_stops_analysis() {
    let cancel = AtomicBool::new(true);
    let audio = drum_loop(120.0, 30.0, Style::Straight);
    let err = analyze_samples(&audio, SR, &AnalysisOptions::default(), &cancel, &|_| {});
    assert!(matches!(err, Err(livi_core::AnalysisError::Cancelled)));
}

#[test]
fn tempo_change_is_flagged() {
    let mut audio = drum_loop(100.0, 30.0, Style::Straight);
    audio.extend(drum_loop(104.0, 30.0, Style::Straight));
    let r = analyse(&audio, 80.0, 160.0);
    assert!(r.variable_tempo, "a 100 -> 104 BPM change should be flagged");
}

// ----- key ---------------------------------------------------------------

const C_MAJ: &[i32] = &[48, 60, 64, 67];
const F_MAJ: &[i32] = &[41, 53, 57, 60];
const G_MAJ: &[i32] = &[43, 55, 59, 62];

fn transpose(p: &[&[i32]], by: i32) -> Vec<Vec<i32>> {
    p.iter().map(|c| c.iter().map(|n| n + by).collect()).collect()
}

fn chord_audio(p: &[Vec<i32>], cents: f32) -> Vec<f32> {
    let refs: Vec<&[i32]> = p.iter().map(|v| v.as_slice()).collect();
    chords(&refs, 1.6, 40.0, cents)
}

fn key_of(samples: &[f32]) -> (String, String) {
    let r = analyse(samples, 70.0, 180.0);
    println!("key {} {} conf {:.2} tuning {:.0}c", r.key.name, r.key.camelot, r.key.confidence, r.key.tuning_cents);
    (r.key.name, r.key.camelot)
}

#[test]
fn c_major() {
    let p = transpose(&[C_MAJ, F_MAJ, G_MAJ, C_MAJ], 0);
    assert_eq!(key_of(&chord_audio(&p, 0.0)), ("C major".into(), "8B".into()));
}

#[test]
fn a_minor() {
    let am: &[i32] = &[45, 57, 60, 64];
    let dm: &[i32] = &[50, 53, 57, 62];
    let e: &[i32] = &[40, 52, 56, 59];
    let p = transpose(&[am, dm, e, am], 0);
    assert_eq!(key_of(&chord_audio(&p, 0.0)), ("A minor".into(), "8A".into()));
}

#[test]
fn transposed_keys_follow() {
    // C major up 3 semitones = Eb major (5B); up 6 = F# major (2B).
    let p3 = transpose(&[C_MAJ, F_MAJ, G_MAJ, C_MAJ], 3);
    assert_eq!(key_of(&chord_audio(&p3, 0.0)).1, "5B");
    let p6 = transpose(&[C_MAJ, F_MAJ, G_MAJ, C_MAJ], 6);
    assert_eq!(key_of(&chord_audio(&p6, 0.0)).1, "2B");
}

#[test]
fn detuned_recording_still_found() {
    let p = transpose(&[C_MAJ, F_MAJ, G_MAJ, C_MAJ], 0);
    let r = key_of(&chord_audio(&p, 30.0));
    assert_eq!(r.1, "8B");
}

#[test]
fn key_survives_drums() {
    let p = transpose(&[C_MAJ, F_MAJ, G_MAJ, C_MAJ], 0);
    let harmony = chord_audio(&p, 0.0);
    let drums = drum_loop(120.0, 40.0, Style::Straight);
    let both = mix(&harmony, &drums, 1.0);
    assert_eq!(key_of(&both).1, "8B");
}

#[test]
fn tempo_and_key_together() {
    let p = transpose(&[C_MAJ, F_MAJ, G_MAJ, C_MAJ], 0);
    let both = mix(&chord_audio(&p, 0.0), &drum_loop(112.0, 40.0, Style::Amapiano), 1.0);
    let r = analyse(&both, 100.0, 125.0);
    assert!((r.bpm_raw - 112.0).abs() < 0.3, "bpm {}", r.bpm_raw);
    assert_eq!(r.key.camelot, "8B");
}

#[test]
fn auto_range_mid_tempos_all_styles() {
    // No genre preset (70-180). Tempos here have no in-range half/double,
    // so the answer must be unambiguous.
    let mut failures = Vec::new();
    for (name, style) in [
        ("straight", Style::Straight),
        ("afrobeats", Style::Afrobeats),
        ("amapiano", Style::Amapiano),
        ("trap", Style::Trap),
    ] {
        for bpm in [95.0, 105.0, 112.0, 120.0, 128.0, 135.0] {
            let r = analyse(&drum_loop(bpm, 60.0, style), 70.0, 180.0);
            if (r.bpm_raw - bpm).abs() > 0.3 {
                failures.push(format!("{name} {bpm}: got {:.2}", r.bpm_raw));
            }
        }
    }
    assert!(failures.is_empty(), "wrong tempos: {failures:?}");
}
