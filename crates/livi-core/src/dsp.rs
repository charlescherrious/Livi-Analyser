//! Small DSP helpers shared by the tempo and key detectors.

use rustfft::{num_complex::Complex, FftPlanner};

/// Periodic Hann window.
pub fn hann(n: usize) -> Vec<f32> {
    (0..n)
        .map(|i| 0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / n as f32).cos())
        .collect()
}

/// Number of full frames a signal of `len` samples yields.
pub fn frame_count(len: usize, n_fft: usize, hop: usize) -> usize {
    if len < n_fft {
        0
    } else {
        (len - n_fft) / hop + 1
    }
}

/// Short-time Fourier transform. Calls `f(frame_index, magnitudes)` for every
/// frame (magnitudes has `n_fft / 2 + 1` bins). Frame `i` covers samples
/// `[i * hop, i * hop + n_fft)`. `f` returns `false` to stop early; the
/// function then returns `false` too.
pub fn stft_magnitudes<F>(samples: &[f32], n_fft: usize, hop: usize, mut f: F) -> bool
where
    F: FnMut(usize, &[f32]) -> bool,
{
    let window = hann(n_fft);
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(n_fft);
    let mut scratch = vec![Complex::new(0.0, 0.0); fft.get_inplace_scratch_len()];
    let mut buf = vec![Complex::new(0.0, 0.0); n_fft];
    let mut mags = vec![0.0f32; n_fft / 2 + 1];

    for i in 0..frame_count(samples.len(), n_fft, hop) {
        let start = i * hop;
        for (j, slot) in buf.iter_mut().enumerate() {
            *slot = Complex::new(samples[start + j] * window[j], 0.0);
        }
        fft.process_with_scratch(&mut buf, &mut scratch);
        for (k, m) in mags.iter_mut().enumerate() {
            *m = buf[k].norm();
        }
        if !f(i, &mags) {
            return false;
        }
    }
    true
}

/// Parabolic interpolation through three equally spaced points
/// `(-1, ym1)`, `(0, y0)`, `(1, yp1)`. Returns `(offset, peak_value)`.
pub fn parabolic(ym1: f32, y0: f32, yp1: f32) -> (f32, f32) {
    let denom = ym1 - 2.0 * y0 + yp1;
    if denom.abs() < 1e-12 {
        return (0.0, y0);
    }
    let delta = (0.5 * (ym1 - yp1) / denom).clamp(-0.5, 0.5);
    let peak = y0 - 0.25 * (ym1 - yp1) * delta;
    (delta, peak)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parabolic_finds_vertex() {
        // y = -(x - 0.3)^2
        let f = |x: f32| -(x - 0.3) * (x - 0.3);
        let (d, p) = parabolic(f(-1.0), f(0.0), f(1.0));
        assert!((d - 0.3).abs() < 1e-5);
        assert!(p.abs() < 1e-5);
    }

    #[test]
    fn stft_locates_a_sine() {
        let sr = 22_050.0f32;
        let freq = 1_000.0f32;
        let x: Vec<f32> = (0..8192)
            .map(|n| (2.0 * std::f32::consts::PI * freq * n as f32 / sr).sin())
            .collect();
        let mut peak_bin = 0;
        stft_magnitudes(&x, 2048, 1024, |_, m| {
            peak_bin = m
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .unwrap()
                .0;
            true
        });
        let expected = (freq / (sr / 2048.0)).round() as usize;
        assert_eq!(peak_bin, expected);
    }
}
