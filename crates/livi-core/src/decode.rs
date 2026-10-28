//! Audio file decoding: MP3, WAV, FLAC and AIFF via Symphonia (pure Rust).
//!
//! The decoder streams the file, mixes it down to mono and resamples it to
//! [`ANALYSIS_SR`] on the fly, so a long file never needs a full-rate copy in
//! memory.

use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use rubato::{FftFixedIn, Resampler};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::pipeline::AnalysisError;

/// All analysis runs on mono audio at this sample rate.
pub const ANALYSIS_SR: u32 = 22_050;

const RESAMPLE_CHUNK: usize = 4096;

#[derive(Debug)]
pub struct Decoded {
    /// Mono samples at [`ANALYSIS_SR`].
    pub samples: Vec<f32>,
    pub source_sample_rate: u32,
    pub source_channels: usize,
    pub duration_secs: f64,
}

/// Decode `path` to mono 22.05 kHz. `on_progress` receives 0.0..=1.0 when the
/// file length is known. Returns [`AnalysisError::Cancelled`] if `cancel` is set.
pub fn decode_to_mono(
    path: &Path,
    cancel: &AtomicBool,
    on_progress: &dyn Fn(f32),
) -> Result<Decoded, AnalysisError> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| AnalysisError::Decode(format!("unrecognised audio format ({e})")))?;
    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or(AnalysisError::NoAudio)?;
    let track_id = track.id;
    let codec_params = track.codec_params.clone();
    let source_sr = codec_params
        .sample_rate
        .ok_or_else(|| AnalysisError::Decode("file does not state its sample rate".into()))?;
    let total_frames = codec_params.n_frames;

    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .map_err(|e| AnalysisError::Decode(format!("unsupported codec ({e})")))?;

    let mut resampler = if source_sr != ANALYSIS_SR {
        Some(
            FftFixedIn::<f32>::new(
                source_sr as usize,
                ANALYSIS_SR as usize,
                RESAMPLE_CHUNK,
                2,
                1,
            )
            .map_err(|e| AnalysisError::Decode(format!("resampler setup failed ({e})")))?,
        )
    } else {
        None
    };

    let mut out: Vec<f32> = Vec::new();
    let mut pending: Vec<f32> = Vec::with_capacity(RESAMPLE_CHUNK * 2);
    let mut sbuf: Option<SampleBuffer<f32>> = None;
    let mut sbuf_frames = 0usize;
    let mut channels_seen = 0usize;
    let mut decoded_frames: u64 = 0;
    let mut last_reported = -1.0f32;

    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(AnalysisError::Cancelled);
        }

        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(SymError::ResetRequired) => break,
            Err(e) => return Err(AnalysisError::Decode(e.to_string())),
        };
        if packet.track_id() != track_id {
            continue;
        }

        let audio = match decoder.decode(&packet) {
            Ok(a) => a,
            // A corrupt frame: skip it and keep going.
            Err(SymError::DecodeError(_)) => continue,
            Err(SymError::IoError(_)) => continue,
            Err(e) => return Err(AnalysisError::Decode(e.to_string())),
        };

        let spec = *audio.spec();
        let channels = spec.channels.count().max(1);
        channels_seen = channels;
        if sbuf.is_none() || audio.capacity() > sbuf_frames {
            sbuf_frames = audio.capacity();
            sbuf = Some(SampleBuffer::<f32>::new(sbuf_frames as u64, spec));
        }
        let buf = sbuf.as_mut().unwrap();
        buf.copy_interleaved_ref(audio);

        let inv = 1.0 / channels as f32;
        let before = pending.len();
        for frame in buf.samples().chunks_exact(channels) {
            pending.push(frame.iter().sum::<f32>() * inv);
        }
        decoded_frames += (pending.len() - before) as u64;

        drain_pending(&mut pending, &mut resampler, &mut out, false)?;

        if let Some(total) = total_frames {
            if total > 0 {
                let frac = (decoded_frames as f32 / total as f32).min(1.0);
                if frac - last_reported >= 0.01 {
                    last_reported = frac;
                    on_progress(frac);
                }
            }
        }
    }

    // Flush whatever is left in the resampler input.
    drain_pending(&mut pending, &mut resampler, &mut out, true)?;

    // The resampler pads its last chunk, so trim to the exact expected length.
    if resampler.is_some() {
        let expected = (decoded_frames as f64 * ANALYSIS_SR as f64 / source_sr as f64).round()
            as usize;
        out.truncate(expected);
    }

    if out.is_empty() {
        return Err(AnalysisError::NoAudio);
    }
    on_progress(1.0);

    Ok(Decoded {
        duration_secs: decoded_frames as f64 / source_sr as f64,
        samples: out,
        source_sample_rate: source_sr,
        source_channels: channels_seen,
    })
}

/// Resamples mono audio from `source_sr` to [`ANALYSIS_SR`]. Used by the
/// plugin, which receives audio at whatever rate the host runs at.
pub fn resample_mono(samples: &[f32], source_sr: u32) -> Result<Vec<f32>, AnalysisError> {
    if source_sr == ANALYSIS_SR {
        return Ok(samples.to_vec());
    }
    let mut resampler = Some(
        FftFixedIn::<f32>::new(source_sr as usize, ANALYSIS_SR as usize, RESAMPLE_CHUNK, 2, 1)
            .map_err(|e| AnalysisError::Decode(format!("resampler setup failed ({e})")))?,
    );
    let mut pending = samples.to_vec();
    let mut out = Vec::with_capacity(samples.len() * ANALYSIS_SR as usize / source_sr as usize + 1024);
    drain_pending(&mut pending, &mut resampler, &mut out, true)?;
    let expected = (samples.len() as f64 * ANALYSIS_SR as f64 / source_sr as f64).round() as usize;
    out.truncate(expected);
    Ok(out)
}

/// Move whole chunks from `pending` through the resampler (or straight across
/// when no resampling is needed). With `flush`, the final partial chunk is
/// zero-padded and processed too.
fn drain_pending(
    pending: &mut Vec<f32>,
    resampler: &mut Option<FftFixedIn<f32>>,
    out: &mut Vec<f32>,
    flush: bool,
) -> Result<(), AnalysisError> {
    let Some(rs) = resampler.as_mut() else {
        out.append(pending);
        return Ok(());
    };

    let mut consumed = 0;
    while pending.len() - consumed >= RESAMPLE_CHUNK {
        let chunk = &pending[consumed..consumed + RESAMPLE_CHUNK];
        let res = rs
            .process(&[chunk], None)
            .map_err(|e| AnalysisError::Decode(format!("resampling failed ({e})")))?;
        out.extend_from_slice(&res[0]);
        consumed += RESAMPLE_CHUNK;
    }
    pending.drain(..consumed);

    if flush && !pending.is_empty() {
        pending.resize(RESAMPLE_CHUNK, 0.0);
        let res = rs
            .process(&[&pending[..]], None)
            .map_err(|e| AnalysisError::Decode(format!("resampling failed ({e})")))?;
        out.extend_from_slice(&res[0]);
        pending.clear();
    }
    Ok(())
}
