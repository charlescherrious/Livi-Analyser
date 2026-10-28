//! Musical key detection.
//!
//! Method: long-window spectra -> keep only steady (tonal) energy by taking a
//! median over neighbouring frames (drums are short, so they drop out) ->
//! pick spectral peaks -> estimate the tuning offset -> fold into a 12-bin
//! pitch-class profile (chromagram) -> correlate with major/minor key
//! profiles for all 24 keys.

use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;

use crate::camelot::{self, Mode};
use crate::dsp;

pub const N_FFT: usize = 8192;
pub const HOP: usize = 4096;
/// Frames in the median filter used to suppress drums.
const MEDIAN_FRAMES: usize = 5;
const MIN_HZ: f32 = 60.0;
const MAX_HZ: f32 = 5_000.0;

// Krumhansl-Kessler and Temperley profiles (index 0 = tonic).
const KK_MAJOR: [f32; 12] = [6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88];
const KK_MINOR: [f32; 12] = [6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17];
const TP_MAJOR: [f32; 12] = [5.0, 2.0, 3.5, 2.0, 4.5, 4.0, 2.0, 4.5, 2.0, 3.5, 1.5, 4.0];
const TP_MINOR: [f32; 12] = [5.0, 2.0, 3.5, 4.5, 2.0, 4.0, 2.0, 4.5, 3.5, 2.0, 1.5, 4.0];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyCandidate {
    pub tonic: u8,
    pub mode: Mode,
    pub name: String,
    pub short: String,
    pub camelot: String,
    /// Correlation with the key profile, roughly -1..1 (higher is better).
    pub score: f32,
}

#[derive(Debug, Clone)]
pub struct KeyResult {
    pub best: KeyCandidate,
    /// Top candidates, best first (includes `best`).
    pub candidates: Vec<KeyCandidate>,
    /// 0.0..=1.0 from the gap between the best and second-best key.
    pub confidence: f32,
    /// Estimated tuning offset in cents relative to A = 440 Hz.
    pub tuning_cents: f32,
}

struct Peak {
    frame: usize,
    freq: f32,
    mag: f32,
}

/// Weight by frequency: ignore sub-bass, taper the very top.
fn freq_weight(f: f32) -> f32 {
    if f < MIN_HZ {
        0.0
    } else if f < 120.0 {
        (f - MIN_HZ) / (120.0 - MIN_HZ)
    } else if f <= 2_000.0 {
        1.0
    } else {
        ((MAX_HZ - f) / (MAX_HZ - 2_000.0)).clamp(0.0, 1.0)
    }
}

pub fn estimate_key(samples: &[f32], sr: u32, cancel: &AtomicBool) -> Option<KeyResult> {
    let n_frames = dsp::frame_count(samples.len(), N_FFT, HOP);
    if n_frames < MEDIAN_FRAMES + 2 {
        return None;
    }
    let bin_hz = sr as f32 / N_FFT as f32;
    let k_lo = (MIN_HZ / bin_hz).floor() as usize;
    let k_hi = (MAX_HZ / bin_hz).ceil() as usize;
    let width = k_hi - k_lo + 1;

    // 1. Magnitudes for the bins we care about.
    let mut spec = vec![0.0f32; n_frames * width];
    let finished = dsp::stft_magnitudes(samples, N_FFT, HOP, |i, mags| {
        if i % 64 == 0 && cancel.load(Ordering::Relaxed) {
            return false;
        }
        spec[i * width..(i + 1) * width].copy_from_slice(&mags[k_lo..=k_hi]);
        true
    });
    if !finished {
        return None;
    }

    // 2. Median over time keeps steady notes and removes drum hits.
    let half = MEDIAN_FRAMES / 2;
    let mut steady = vec![0.0f32; n_frames * width];
    let mut window = [0.0f32; MEDIAN_FRAMES];
    for t in 0..n_frames {
        if t % 64 == 0 && cancel.load(Ordering::Relaxed) {
            return None;
        }
        let lo = t.saturating_sub(half);
        let hi = (t + half).min(n_frames - 1);
        let len = hi - lo + 1;
        for b in 0..width {
            for (j, frame) in (lo..=hi).enumerate() {
                window[j] = spec[frame * width + b];
            }
            let w = &mut window[..len];
            w.sort_by(|a, b| a.partial_cmp(b).unwrap());
            steady[t * width + b] = w[len / 2];
        }
    }

    // 3. Spectral peaks per frame.
    let global_max = steady.iter().cloned().fold(0.0f32, f32::max);
    if global_max <= 0.0 {
        return None;
    }
    let mut peaks: Vec<Peak> = Vec::new();
    for t in 0..n_frames {
        let row = &steady[t * width..(t + 1) * width];
        let frame_max = row.iter().cloned().fold(0.0f32, f32::max);
        // Skip near-silent frames (more than 50 dB below the loudest moment).
        if frame_max < global_max * 0.003 {
            continue;
        }
        let threshold = frame_max * 0.02;
        for b in 1..width - 1 {
            if row[b] > threshold && row[b] > row[b - 1] && row[b] >= row[b + 1] {
                let (delta, _) = dsp::parabolic(
                    row[b - 1].max(1e-9).ln(),
                    row[b].max(1e-9).ln(),
                    row[b + 1].max(1e-9).ln(),
                );
                let freq = (k_lo + b) as f32 * bin_hz + delta * bin_hz;
                let w = freq_weight(freq);
                if w > 0.0 {
                    peaks.push(Peak {
                        frame: t,
                        freq,
                        mag: row[b].sqrt() * w / frame_max.sqrt(),
                    });
                }
            }
        }
    }
    if peaks.is_empty() {
        return None;
    }

    // 4. Tuning: circular mean of each peak's distance from the nearest semitone.
    let (mut sx, mut sy) = (0.0f64, 0.0f64);
    for p in &peaks {
        let midi = 69.0 + 12.0 * (p.freq / 440.0).log2();
        let angle = 2.0 * std::f64::consts::PI * midi as f64;
        sx += p.mag as f64 * angle.cos();
        sy += p.mag as f64 * angle.sin();
    }
    let tuning_semitones = (sy.atan2(sx) / (2.0 * std::f64::consts::PI)) as f32; // -0.5..0.5

    // 5. Chromagram, each frame normalised so loud sections do not dominate.
    let mut chroma = [0.0f32; 12];
    let mut frame_chroma = [0.0f32; 12];
    let mut current = peaks[0].frame;
    let flush = |frame_chroma: &mut [f32; 12], chroma: &mut [f32; 12]| {
        let total: f32 = frame_chroma.iter().sum();
        if total > 0.0 {
            for i in 0..12 {
                chroma[i] += frame_chroma[i] / total;
            }
        }
        *frame_chroma = [0.0; 12];
    };
    for p in &peaks {
        if p.frame != current {
            flush(&mut frame_chroma, &mut chroma);
            current = p.frame;
        }
        let midi = 69.0 + 12.0 * (p.freq / 440.0).log2() - tuning_semitones;
        let nearest = midi.round();
        let dev = (midi - nearest).abs();
        // Peaks midway between two semitones are not tonal; fade them out.
        let tonal = (1.0 - 2.0 * dev).max(0.0);
        let pc = (nearest as i32).rem_euclid(12) as usize;
        frame_chroma[pc] += p.mag * tonal;
    }
    flush(&mut frame_chroma, &mut chroma);

    // 6. Correlate with the 24 keys, averaging two profile families.
    let mut all: Vec<KeyCandidate> = Vec::with_capacity(24);
    for tonic in 0..12u8 {
        for (mode, profiles) in [
            (Mode::Major, [&KK_MAJOR, &TP_MAJOR]),
            (Mode::Minor, [&KK_MINOR, &TP_MINOR]),
        ] {
            let score = profiles
                .iter()
                .map(|p| correlation_rotated(&chroma, p, tonic as usize))
                .sum::<f32>()
                / profiles.len() as f32;
            all.push(KeyCandidate {
                tonic,
                mode,
                name: camelot::key_name(tonic, mode),
                short: camelot::key_short(tonic, mode),
                camelot: camelot::camelot(tonic, mode),
                score,
            });
        }
    }
    all.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

    let margin = all[0].score - all[1].score;
    // 0.08 correlation gap is treated as "clearly the best key".
    let confidence = (margin / 0.08).clamp(0.0, 1.0);

    all.truncate(3);
    Some(KeyResult {
        best: all[0].clone(),
        candidates: all,
        confidence,
        tuning_cents: tuning_semitones * 100.0,
    })
}

/// Pearson correlation between `chroma` and `profile` rotated so that the
/// profile's tonic sits on pitch class `tonic`.
fn correlation_rotated(chroma: &[f32; 12], profile: &[f32; 12], tonic: usize) -> f32 {
    let mean_c = chroma.iter().sum::<f32>() / 12.0;
    let mean_p = profile.iter().sum::<f32>() / 12.0;
    let (mut num, mut dc, mut dp) = (0.0f32, 0.0f32, 0.0f32);
    for pc in 0..12 {
        let c = chroma[pc] - mean_c;
        let p = profile[(pc + 12 - tonic) % 12] - mean_p;
        num += c * p;
        dc += c * c;
        dp += p * p;
    }
    if dc <= 0.0 || dp <= 0.0 {
        0.0
    } else {
        num / (dc.sqrt() * dp.sqrt())
    }
}
