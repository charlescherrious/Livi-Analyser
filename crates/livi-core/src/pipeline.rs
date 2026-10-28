//! The public entry points: decode a file (or take raw samples), run tempo and
//! key detection in parallel, and return one serialisable result.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use serde::Serialize;

use crate::decode::{self, ANALYSIS_SR};
use crate::key::{self, KeyCandidate};
use crate::tempo::{self, TempoCandidate, TempoConfig};

/// Shortest clip (seconds) we will analyse.
pub const MIN_DURATION_SECS: f32 = 5.0;

#[derive(Debug, thiserror::Error)]
pub enum AnalysisError {
    #[error("could not open the file: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not read this audio file: {0}")]
    Decode(String),
    #[error("no audio found in this file")]
    NoAudio,
    #[error("the audio is too short to analyse (needs at least {0} seconds)")]
    TooShort(f32),
    #[error("could not find a steady beat or tonal content (is the file silent?)")]
    NothingDetected,
    #[error("cancelled")]
    Cancelled,
}

#[derive(Debug, Clone, Copy)]
pub struct AnalysisOptions {
    pub min_bpm: f64,
    pub max_bpm: f64,
}

impl Default for AnalysisOptions {
    fn default() -> Self {
        Self {
            min_bpm: 70.0,
            max_bpm: 180.0,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Decoding,
    Tempo,
    Key,
    Done,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub stage: Stage,
    /// 0.0..=1.0 within the stage (tempo and key run side by side).
    pub fraction: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyOutput {
    pub tonic: u8,
    pub mode: crate::camelot::Mode,
    pub name: String,
    pub short: String,
    pub camelot: String,
    pub confidence: f32,
    pub strength: f32,
    pub tuning_cents: f32,
    pub alternatives: Vec<KeyCandidate>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisResult {
    pub file_name: String,
    pub duration_secs: f64,
    pub source_sample_rate: u32,
    pub channels: usize,
    pub bpm: f64,
    pub bpm_raw: f64,
    pub bpm_confidence: f32,
    pub variable_tempo: bool,
    pub bpm_candidates: Vec<TempoCandidate>,
    pub key: KeyOutput,
    pub elapsed_ms: u64,
}

/// Decode and analyse a file. `cancel` can be set from another thread to stop
/// early (the call then returns [`AnalysisError::Cancelled`]).
pub fn analyze_file(
    path: &Path,
    options: &AnalysisOptions,
    cancel: &AtomicBool,
    on_progress: &(dyn Fn(Progress) + Sync),
) -> Result<AnalysisResult, AnalysisError> {
    let started = Instant::now();
    on_progress(Progress {
        stage: Stage::Decoding,
        fraction: 0.0,
    });
    let decoded = decode::decode_to_mono(path, cancel, &|f| {
        on_progress(Progress {
            stage: Stage::Decoding,
            fraction: f,
        })
    })?;

    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut result = analyze_samples(&decoded.samples, ANALYSIS_SR, options, cancel, on_progress)?;
    result.file_name = file_name;
    result.duration_secs = decoded.duration_secs;
    result.source_sample_rate = decoded.source_sample_rate;
    result.channels = decoded.source_channels;
    result.elapsed_ms = started.elapsed().as_millis() as u64;
    on_progress(Progress {
        stage: Stage::Done,
        fraction: 1.0,
    });
    Ok(result)
}

/// Analyse mono samples that are already decoded (used by tests and tools).
pub fn analyze_samples(
    samples: &[f32],
    sr: u32,
    options: &AnalysisOptions,
    cancel: &AtomicBool,
    on_progress: &(dyn Fn(Progress) + Sync),
) -> Result<AnalysisResult, AnalysisError> {
    let started = Instant::now();
    if (samples.len() as f32) < MIN_DURATION_SECS * sr as f32 {
        return Err(AnalysisError::TooShort(MIN_DURATION_SECS));
    }

    let cfg = TempoConfig {
        min_bpm: options.min_bpm,
        max_bpm: options.max_bpm,
    };

    // Tempo and key do not depend on each other: run them side by side.
    let (tempo_res, key_res) = std::thread::scope(|scope| {
        let tempo_handle = scope.spawn(|| {
            let r = tempo::estimate_tempo(samples, sr, &cfg, cancel);
            on_progress(Progress {
                stage: Stage::Tempo,
                fraction: 1.0,
            });
            r
        });
        let key_handle = scope.spawn(|| {
            let r = key::estimate_key(samples, sr, cancel);
            on_progress(Progress {
                stage: Stage::Key,
                fraction: 1.0,
            });
            r
        });
        (
            tempo_handle.join().expect("tempo thread panicked"),
            key_handle.join().expect("key thread panicked"),
        )
    });

    if cancel.load(Ordering::Relaxed) {
        return Err(AnalysisError::Cancelled);
    }
    let (tempo_res, key_res) = match (tempo_res, key_res) {
        (Some(t), Some(k)) => (t, k),
        _ => return Err(AnalysisError::NothingDetected),
    };

    Ok(AnalysisResult {
        file_name: String::new(),
        duration_secs: samples.len() as f64 / sr as f64,
        source_sample_rate: sr,
        channels: 1,
        bpm: tempo_res.bpm,
        bpm_raw: tempo_res.bpm_raw,
        bpm_confidence: tempo_res.confidence,
        variable_tempo: tempo_res.variable_tempo,
        bpm_candidates: tempo_res.candidates,
        key: KeyOutput {
            tonic: key_res.best.tonic,
            mode: key_res.best.mode,
            name: key_res.best.name.clone(),
            short: key_res.best.short.clone(),
            camelot: key_res.best.camelot.clone(),
            confidence: key_res.confidence,
            strength: key_res.best.score,
            tuning_cents: key_res.tuning_cents,
            alternatives: key_res.candidates,
        },
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}
