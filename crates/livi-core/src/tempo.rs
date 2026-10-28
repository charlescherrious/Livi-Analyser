//! Tempo (BPM) detection.
//!
//! Method, in plain words:
//! 1. Turn the audio into an "onset strength" curve: how much new sound
//!    appears in each ~12 ms step (log-mel spectral flux with a max filter,
//!    the SuperFlux idea, which ignores pitch glides such as 808 slides).
//! 2. Take the Fourier transform of that curve. A steady beat shows up as a
//!    sharp line at the beat frequency, so very fine BPM resolution is
//!    possible for tracks made in a DAW.
//! 3. Score each candidate tempo by adding the line at its beat frequency and
//!    its first few multiples (the subdivisions: 8ths, 16ths ...), then
//!    prefer tempos near the middle of the allowed range.
//!
//! Known weakness: half-time / double-time (70 vs 140). The allowed BPM range
//! (genre presets in the UI) is the main tool for resolving that.

use std::sync::atomic::{AtomicBool, Ordering};

use rustfft::{num_complex::Complex, FftPlanner};
use serde::Serialize;

use crate::dsp;

pub const N_FFT: usize = 1024;
pub const HOP: usize = 256;
const N_MELS: usize = 40;
const MEL_FMIN: f32 = 30.0;
const MEL_FMAX: f32 = 10_000.0;
const DB_FLOOR: f32 = -60.0;
/// Weights for the beat frequency and its 2nd, 3rd and 4th multiples.
const HARMONIC_WEIGHTS: [f32; 4] = [1.0, 0.5, 0.33, 0.25];
/// Weights for the pulses BELOW the beat: every 2 beats and every bar (4
/// beats). A true tempo has these; a false 3:2 tempo (triplet hats read as
/// the beat, e.g. 135 for a 90 BPM groove) does not. This is what separates
/// "90 with a triplet shuffle" from "135".
/// How much of the beat-level (fundamental) score comes from the low and mid
/// groups instead of all groups.
const BODY_SHARE: f32 = 0.7;
const SUBHARMONIC_WEIGHTS: [(usize, f32); 2] = [(2, 0.6), (4, 0.6)];
/// Half-width of the moving average removed from the onset curve (frames).
const DETREND_HALF_WIDTH: usize = 43;
/// Shortest segment (seconds) that is analysed on its own for drift checks.
const MIN_SEGMENT_SECS: f64 = 20.0;

#[derive(Debug, Clone, Copy)]
pub struct TempoConfig {
    pub min_bpm: f64,
    pub max_bpm: f64,
}

impl Default for TempoConfig {
    fn default() -> Self {
        Self {
            min_bpm: 70.0,
            max_bpm: 180.0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TempoCandidate {
    pub bpm: f64,
    /// Strength relative to the best candidate (best = 1.0).
    pub score: f64,
}

#[derive(Debug, Clone)]
pub struct TempoResult {
    /// Reported BPM (snapped to the nearest 0.5 when very close to it).
    pub bpm: f64,
    /// Measured BPM before snapping.
    pub bpm_raw: f64,
    /// 0.0..=1.0: how clearly one tempo stands out from unrelated ones.
    pub confidence: f32,
    /// True when the two halves of the track disagree on tempo.
    pub variable_tempo: bool,
    pub candidates: Vec<TempoCandidate>,
}

pub fn estimate_tempo(
    samples: &[f32],
    sr: u32,
    cfg: &TempoConfig,
    cancel: &AtomicBool,
) -> Option<TempoResult> {
    let (envs, fps) = onset_envelopes(samples, sr, cancel)?;
    let env_len = envs[0].len();
    if env_len < 64 {
        return None;
    }
    let spectrum = TempoSpectrum::new(&envs.iter().map(|e| &e[..]).collect::<Vec<_>>(), fps);
    let pick = spectrum.pick(cfg)?;

    let bpm_raw = pick.bpm;
    let bpm = snap_bpm(bpm_raw);

    // Drift check: does each half of the track agree with the whole?
    let min_len = (MIN_SEGMENT_SECS * fps) as usize;
    let variable_tempo = if env_len >= 2 * min_len {
        let mid = env_len / 2;
        let first: Vec<&[f32]> = envs.iter().map(|e| &e[..mid]).collect();
        let second: Vec<&[f32]> = envs.iter().map(|e| &e[mid..]).collect();
        let a = TempoSpectrum::new(&first, fps).peak_near(bpm_raw, 0.05);
        let b = TempoSpectrum::new(&second, fps).peak_near(bpm_raw, 0.05);
        match (a, b) {
            (Some(a), Some(b)) => (a - b).abs() / bpm_raw > 0.008,
            _ => false,
        }
    } else {
        false
    };

    Some(TempoResult {
        bpm,
        bpm_raw,
        confidence: pick.confidence,
        variable_tempo,
        candidates: pick.candidates,
    })
}

/// Snap to the nearest 0.5 BPM when the measurement is within 0.1 of it.
pub fn snap_bpm(raw: f64) -> f64 {
    let nearest = (raw * 2.0).round() / 2.0;
    if (raw - nearest).abs() <= 0.1 {
        nearest
    } else {
        (raw * 100.0).round() / 100.0
    }
}

// ---------------------------------------------------------------------------
// Onset envelope
// ---------------------------------------------------------------------------

fn hz_to_mel(f: f32) -> f32 {
    2595.0 * (1.0 + f / 700.0).log10()
}

fn mel_to_hz(m: f32) -> f32 {
    700.0 * (10f32.powf(m / 2595.0) - 1.0)
}

/// Triangular mel filters over FFT bins: `filters[band] = (first_bin, weights)`.
fn mel_filterbank(sr: u32) -> Vec<(usize, Vec<f32>)> {
    let n_bins = N_FFT / 2 + 1;
    let bin_hz = sr as f32 / N_FFT as f32;
    let fmax = MEL_FMAX.min(sr as f32 / 2.0);
    let (m_lo, m_hi) = (hz_to_mel(MEL_FMIN), hz_to_mel(fmax));
    let edges: Vec<f32> = (0..N_MELS + 2)
        .map(|i| mel_to_hz(m_lo + (m_hi - m_lo) * i as f32 / (N_MELS + 1) as f32))
        .collect();

    (0..N_MELS)
        .map(|b| {
            let (lo, mid, hi) = (edges[b], edges[b + 1], edges[b + 2]);
            let first = ((lo / bin_hz).floor() as usize).min(n_bins - 1);
            let last = ((hi / bin_hz).ceil() as usize).min(n_bins - 1);
            let weights: Vec<f32> = (first..=last)
                .map(|k| {
                    let f = k as f32 * bin_hz;
                    if f <= lo || f >= hi {
                        0.0
                    } else if f <= mid {
                        (f - lo) / (mid - lo)
                    } else {
                        (hi - f) / (hi - mid)
                    }
                })
                .collect();
            (first, weights)
        })
        .collect()
}

/// Upper edges (Hz) of the frequency groups analysed separately. Each group
/// gets its own onset curve so that kicks (low) count as much as hats (high).
const GROUP_EDGES_HZ: [f32; 3] = [200.0, 1_500.0, 5_000.0];

/// Returns one onset-strength curve per frequency group, plus their frame
/// rate in Hz.
pub fn onset_envelopes(
    samples: &[f32],
    sr: u32,
    cancel: &AtomicBool,
) -> Option<(Vec<Vec<f32>>, f64)> {
    let n_frames = dsp::frame_count(samples.len(), N_FFT, HOP);
    if n_frames < 64 {
        return None;
    }
    let filters = mel_filterbank(sr);

    // Pass 1: mel power per frame.
    let mut mel = vec![0.0f32; n_frames * N_MELS];
    let mut max_power = 0.0f32;
    let finished = dsp::stft_magnitudes(samples, N_FFT, HOP, |i, mags| {
        if i % 256 == 0 && cancel.load(Ordering::Relaxed) {
            return false;
        }
        for (b, (first, weights)) in filters.iter().enumerate() {
            let mut p = 0.0f32;
            for (j, w) in weights.iter().enumerate() {
                let m = mags[first + j];
                p += w * m * m;
            }
            mel[i * N_MELS + b] = p;
            if p > max_power {
                max_power = p;
            }
        }
        true
    });
    if !finished || max_power <= 0.0 {
        return None; // cancelled or silent
    }

    // To decibels relative to the loudest band/frame, floored.
    for p in mel.iter_mut() {
        let rel = (*p / max_power).max(1e-10);
        *p = (10.0 * rel.log10()).max(DB_FLOOR);
    }

    // Which group does each mel band belong to? (by its centre frequency)
    let fmax = MEL_FMAX.min(sr as f32 / 2.0);
    let (m_lo, m_hi) = (hz_to_mel(MEL_FMIN), hz_to_mel(fmax));
    let n_groups = GROUP_EDGES_HZ.len() + 1;
    let group_of: Vec<usize> = (0..N_MELS)
        .map(|b| {
            let centre = mel_to_hz(m_lo + (m_hi - m_lo) * (b + 1) as f32 / (N_MELS + 1) as f32);
            GROUP_EDGES_HZ.iter().filter(|&&e| centre >= e).count()
        })
        .collect();
    let mut group_size = vec![0usize; n_groups];
    for &g in &group_of {
        group_size[g] += 1;
    }

    // Spectral flux against the max of neighbouring bands in the previous frame.
    let mut flux = vec![vec![0.0f32; n_frames]; n_groups];
    for t in 1..n_frames {
        let prev = &mel[(t - 1) * N_MELS..t * N_MELS];
        let cur = &mel[t * N_MELS..(t + 1) * N_MELS];
        for b in 0..N_MELS {
            let lo = b.saturating_sub(1);
            let hi = (b + 1).min(N_MELS - 1);
            let reference = prev[lo..=hi].iter().cloned().fold(f32::MIN, f32::max);
            let g = group_of[b];
            flux[g][t] += (cur[b] - reference).max(0.0) / group_size[g] as f32;
        }
    }

    // Remove the slow trend, keep the pulses.
    let envs = flux
        .iter()
        .filter(|f| f.iter().any(|v| *v > 0.0))
        .map(|f| detrend(f, DETREND_HALF_WIDTH))
        .collect::<Vec<_>>();
    if envs.is_empty() {
        return None;
    }
    Some((envs, sr as f64 / HOP as f64))
}

fn detrend(x: &[f32], half: usize) -> Vec<f32> {
    let n = x.len();
    let mut prefix = vec![0.0f64; n + 1];
    for i in 0..n {
        prefix[i + 1] = prefix[i] + x[i] as f64;
    }
    (0..n)
        .map(|i| {
            let lo = i.saturating_sub(half);
            let hi = (i + half + 1).min(n);
            let mean = (prefix[hi] - prefix[lo]) / (hi - lo) as f64;
            (x[i] as f64 - mean).max(0.0) as f32
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tempo spectrum
// ---------------------------------------------------------------------------

struct Pick {
    bpm: f64,
    confidence: f32,
    candidates: Vec<TempoCandidate>,
}

struct TempoSpectrum {
    mag: Vec<f32>,
    /// Same idea, but only the low and mid groups (kick, bass, snare, claps),
    /// scaled to be comparable with `mag`. These say where the BEAT is; the
    /// hats only describe subdivisions of it.
    body: Vec<f32>,
    /// BPM represented by one FFT bin.
    bpm_per_bin: f64,
}

impl TempoSpectrum {
    /// Builds one combined tempo spectrum from several onset curves. Each
    /// curve's spectrum is scaled by its own RMS so every frequency group
    /// votes equally; real beats line up across groups, noise does not.
    fn new(envs: &[&[f32]], fps: f64) -> Self {
        let n = envs[0].len();
        let window = dsp::hann(n);
        let size = (n * 8).next_power_of_two().max(1 << 16);
        let fft = FftPlanner::<f32>::new().plan_fft_forward(size);

        let mut combined = vec![0.0f32; size / 2];
        let mut body = vec![0.0f32; size / 2];
        let n_body = (envs.len() / 2).max(1);
        for (e_idx, env) in envs.iter().enumerate() {
            let mean = env.iter().sum::<f32>() / n as f32;
            let mut buf: Vec<Complex<f32>> = (0..size)
                .map(|i| {
                    if i < n {
                        Complex::new((env[i] - mean) * window[i], 0.0)
                    } else {
                        Complex::new(0.0, 0.0)
                    }
                })
                .collect();
            fft.process(&mut buf);
            let mags: Vec<f32> = buf[..size / 2].iter().map(|c| c.norm()).collect();
            let rms = (mags.iter().map(|m| m * m).sum::<f32>() / mags.len() as f32).sqrt();
            if rms > 0.0 {
                for (c, m) in combined.iter_mut().zip(&mags) {
                    *c += m / rms;
                }
                if e_idx < n_body {
                    let scale = envs.len() as f32 / n_body as f32;
                    for (c, m) in body.iter_mut().zip(&mags) {
                        *c += scale * m / rms;
                    }
                }
            }
        }
        Self {
            mag: combined,
            body,
            bpm_per_bin: 60.0 * fps / size as f64,
        }
    }

    fn bin_of(&self, bpm: f64) -> usize {
        (bpm / self.bpm_per_bin).round() as usize
    }

    /// Strongest value within a few bins of `idx` (spectral lines are wide).
    fn line(&self, idx: usize, spread: usize) -> f32 {
        let lo = idx.saturating_sub(spread);
        let hi = (idx + spread).min(self.mag.len() - 1);
        self.mag[lo..=hi].iter().cloned().fold(0.0, f32::max)
    }

    /// Harmonic-sum score for beat frequency bin `k`: the beat, its
    /// subdivisions above it, and the 2-beat / bar pulses below it.
    fn score(&self, k: usize) -> f32 {
        let mut s = 0.0;
        for (h, w) in HARMONIC_WEIGHTS.iter().enumerate() {
            let idx = k * (h + 1);
            if idx < self.mag.len() {
                // The beat itself is judged mostly on kick/snare energy.
                let m = if h == 0 {
                    BODY_SHARE * self.body[idx] + (1.0 - BODY_SHARE) * self.mag[idx]
                } else {
                    self.mag[idx]
                };
                s += w * m;
            }
        }
        for (div, w) in SUBHARMONIC_WEIGHTS {
            let idx = (k as f64 / div as f64).round() as usize;
            if idx >= 1 {
                s += w * self.line(idx, 3);
            }
        }
        s
    }

    fn refine(&self, scores: &[f32], k_lo: usize, k: usize) -> f64 {
        let i = k - k_lo;
        let (delta, _) = if i > 0 && i + 1 < scores.len() {
            dsp::parabolic(scores[i - 1], scores[i], scores[i + 1])
        } else {
            (0.0, scores[i])
        };
        (k as f64 + delta as f64) * self.bpm_per_bin
    }

    fn pick(&self, cfg: &TempoConfig) -> Option<Pick> {
        let k_lo = self.bin_of(cfg.min_bpm).max(2);
        let k_hi = self.bin_of(cfg.max_bpm).min(self.mag.len() - 2);
        if k_hi <= k_lo + 2 {
            return None;
        }

        let scores: Vec<f32> = (k_lo..=k_hi).map(|k| self.score(k)).collect();

        // Prefer tempos near the geometric middle of the allowed range.
        let centre = (cfg.min_bpm * cfg.max_bpm).sqrt();
        let weighted: Vec<f32> = scores
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let bpm = (k_lo + i) as f64 * self.bpm_per_bin;
                let octaves = (bpm / centre).log2();
                s * (-0.5 * (octaves / 0.9).powi(2)).exp() as f32
            })
            .collect();

        // Local maxima, best first, at least 3% apart.
        let mut peaks: Vec<usize> = (1..weighted.len() - 1)
            .filter(|&i| weighted[i] > weighted[i - 1] && weighted[i] >= weighted[i + 1])
            .collect();
        peaks.sort_by(|&a, &b| weighted[b].partial_cmp(&weighted[a]).unwrap());
        let best_i = *peaks.first()?;
        let best_bpm = self.refine(&scores, k_lo, k_lo + best_i);

        let mut chosen: Vec<(f64, f32)> = Vec::new();
        for &i in &peaks {
            let bpm = self.refine(&scores, k_lo, k_lo + i);
            if chosen.iter().all(|(b, _)| (bpm - b).abs() / b > 0.03) {
                chosen.push((bpm, weighted[i]));
            }
            if chosen.len() == 3 {
                break;
            }
        }
        let best_w = chosen[0].1.max(1e-12);
        let candidates = chosen
            .iter()
            .map(|(bpm, w)| TempoCandidate {
                bpm: (bpm * 100.0).round() / 100.0,
                score: (*w / best_w) as f64,
            })
            .collect();

        // Confidence: how far the best score is above the strongest tempo
        // that is NOT a simple multiple/fraction of it.
        let best_score = scores[best_i].max(1e-12);
        let related = [1.0 / 3.0, 0.5, 2.0 / 3.0, 0.75, 1.0, 4.0 / 3.0, 1.5, 2.0, 3.0];
        let mut second = 0.0f32;
        for (i, s) in scores.iter().enumerate() {
            let bpm = (k_lo + i) as f64 * self.bpm_per_bin;
            let ratio = bpm / best_bpm;
            if related.iter().all(|r| (ratio / r - 1.0).abs() > 0.03) && *s > second {
                second = *s;
            }
        }
        let confidence = (1.0 - second / best_score).clamp(0.0, 1.0);

        Some(Pick {
            bpm: best_bpm,
            confidence,
            candidates,
        })
    }

    /// Strongest tempo within `tol` (fraction) of `target`, if any.
    fn peak_near(&self, target: f64, tol: f64) -> Option<f64> {
        let k_lo = self.bin_of(target * (1.0 - tol)).max(2);
        let k_hi = self.bin_of(target * (1.0 + tol)).min(self.mag.len() - 2);
        if k_hi <= k_lo + 2 {
            return None;
        }
        let scores: Vec<f32> = (k_lo..=k_hi).map(|k| self.score(k)).collect();
        let (i, _) = scores
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())?;
        if i == 0 || i + 1 == scores.len() {
            return None;
        }
        Some(self.refine(&scores, k_lo, k_lo + i))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapping() {
        assert_eq!(snap_bpm(112.04), 112.0);
        assert_eq!(snap_bpm(97.56), 97.5);
        assert_eq!(snap_bpm(120.3), 120.3);
    }
}
