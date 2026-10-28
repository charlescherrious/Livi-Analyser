//! Synthetic audio for tests: drum loops with a known BPM and chord
//! progressions in a known key. Deterministic (fixed noise seed).

#![allow(dead_code)]

pub const SR: u32 = 22_050;

pub struct Noise(u64);
impl Noise {
    pub fn new(seed: u64) -> Self {
        Noise(seed)
    }
    pub fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f32 / (1u64 << 31) as f32) * 2.0 - 1.0
    }
}

fn add(out: &mut [f32], start: usize, event: &[f32], gain: f32) {
    for (i, s) in event.iter().enumerate() {
        if let Some(o) = out.get_mut(start + i) {
            *o += s * gain;
        }
    }
}

fn kick() -> Vec<f32> {
    let n = (0.22 * SR as f32) as usize;
    let mut phase = 0.0f32;
    (0..n)
        .map(|i| {
            let t = i as f32 / SR as f32;
            let f = 50.0 + 90.0 * (-t / 0.03).exp();
            phase += 2.0 * std::f32::consts::PI * f / SR as f32;
            phase.sin() * (-t / 0.09).exp()
        })
        .collect()
}

fn snare(noise: &mut Noise) -> Vec<f32> {
    let n = (0.18 * SR as f32) as usize;
    (0..n)
        .map(|i| {
            let t = i as f32 / SR as f32;
            let tone = (2.0 * std::f32::consts::PI * 190.0 * t).sin() * 0.4;
            (noise.next() * 0.8 + tone) * (-t / 0.05).exp()
        })
        .collect()
}

fn hat(noise: &mut Noise, decay: f32) -> Vec<f32> {
    let n = (0.06 * SR as f32) as usize;
    let mut prev = 0.0;
    (0..n)
        .map(|i| {
            let t = i as f32 / SR as f32;
            let x = noise.next();
            let hp = x - prev; // crude high-pass
            prev = x;
            hp * (-t / decay).exp()
        })
        .collect()
}

fn log_drum(freq: f32) -> Vec<f32> {
    let n = (0.35 * SR as f32) as usize;
    (0..n)
        .map(|i| {
            let t = i as f32 / SR as f32;
            (2.0 * std::f32::consts::PI * freq * (1.0 - 0.15 * t) * t).sin() * (-t / 0.12).exp()
        })
        .collect()
}

#[derive(Clone, Copy)]
pub enum Style {
    /// Four on the floor, backbeat snare, straight 8th hats.
    Straight,
    /// Afrobeats-like: syncopated kick, claps on 2 and 4, 16th shaker.
    Afrobeats,
    /// Amapiano-like: sparse kick, swung 16th shaker, log drum on offbeats.
    Amapiano,
    /// Trap-like: snare only on beat 3 of each bar, 16th hats with a roll.
    Trap,
    /// Triplet-feel groove (shuffled shaker on every triplet 8th, kick on 1
    /// and 3, snare on 2 and 4). The triplet pulse is 1.5x the real tempo.
    TripletLoud,
    /// Same groove but the triplet shaker is faint and the backbeat is clear.
    TripletSoft,
}

/// `positions` are in beats; one bar = 4 beats.
pub fn drum_loop(bpm: f64, seconds: f64, style: Style) -> Vec<f32> {
    let total = (seconds * SR as f64) as usize;
    let mut out = vec![0.0f32; total];
    let mut noise = Noise::new(7);
    let beat_secs = 60.0 / bpm;
    let at = |beats: f64| ((beats * beat_secs) * SR as f64).round() as usize;
    let k = kick();
    let n_beats = (seconds / beat_secs) as usize + 1;

    for bar in 0..(n_beats / 4 + 1) {
        let b0 = bar as f64 * 4.0;
        match style {
            Style::Straight => {
                for beat in 0..4 {
                    add(&mut out, at(b0 + beat as f64), &k, 0.9);
                }
                for beat in [1.0, 3.0] {
                    add(&mut out, at(b0 + beat), &snare(&mut noise), 0.6);
                }
                for e in 0..8 {
                    add(&mut out, at(b0 + e as f64 * 0.5), &hat(&mut noise, 0.015), 0.25);
                }
            }
            Style::Afrobeats => {
                for pos in [0.0, 1.5, 2.0, 3.25] {
                    add(&mut out, at(b0 + pos), &k, 0.9);
                }
                for beat in [1.0, 3.0] {
                    add(&mut out, at(b0 + beat), &snare(&mut noise), 0.55);
                }
                for s in 0..16 {
                    let g = if s % 2 == 1 { 0.22 } else { 0.12 };
                    add(&mut out, at(b0 + s as f64 * 0.25), &hat(&mut noise, 0.012), g);
                }
            }
            Style::Amapiano => {
                for pos in [0.0, 2.0] {
                    add(&mut out, at(b0 + pos), &k, 0.85);
                }
                for (i, pos) in [0.75, 1.5, 2.75, 3.5].iter().enumerate() {
                    add(&mut out, at(b0 + pos), &log_drum(70.0 + 8.0 * i as f32), 0.7);
                }
                for s in 0..16 {
                    let swing = if s % 2 == 1 { 0.06 } else { 0.0 };
                    add(
                        &mut out,
                        at(b0 + s as f64 * 0.25 + swing),
                        &hat(&mut noise, 0.01),
                        0.2,
                    );
                }
                add(&mut out, at(b0 + 1.0), &snare(&mut noise), 0.35);
                add(&mut out, at(b0 + 3.0), &snare(&mut noise), 0.35);
            }
            Style::TripletLoud | Style::TripletSoft => {
                let (hat_gain, snare_gain) = match style {
                    Style::TripletLoud => (0.55, 0.45),
                    _ => (0.2, 0.6),
                };
                for pos in [0.0, 2.0, 2.667] {
                    add(&mut out, at(b0 + pos), &k, 0.9);
                }
                for beat in [1.0, 3.0] {
                    add(&mut out, at(b0 + beat), &snare(&mut noise), snare_gain);
                }
                for s in 0..12 {
                    let g = if s % 3 == 0 { hat_gain * 0.7 } else { hat_gain };
                    add(&mut out, at(b0 + s as f64 * (2.0 / 3.0)), &hat(&mut noise, 0.012), g);
                }
            }
            Style::Trap => {
                for pos in [0.0, 1.75, 2.5, 3.5] {
                    add(&mut out, at(b0 + pos), &k, 0.95);
                }
                add(&mut out, at(b0 + 2.0), &snare(&mut noise), 0.7);
                for s in 0..16 {
                    add(&mut out, at(b0 + s as f64 * 0.25), &hat(&mut noise, 0.01), 0.2);
                }
                if bar % 2 == 1 {
                    for r in 0..6 {
                        add(&mut out, at(b0 + 3.5 + r as f64 * 0.0833), &hat(&mut noise, 0.008), 0.25);
                    }
                }
            }
        }
    }
    normalise(&mut out);
    out
}

fn normalise(x: &mut [f32]) {
    let peak = x.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak > 0.0 {
        for s in x.iter_mut() {
            *s *= 0.8 / peak;
        }
    }
}

fn note(midi: f32, secs: f32, cents: f32) -> Vec<f32> {
    let f0 = 440.0 * 2f32.powf((midi - 69.0 + cents / 100.0) / 12.0);
    let n = (secs * SR as f32) as usize;
    let fade = (0.03 * SR as f32) as usize;
    (0..n)
        .map(|i| {
            let t = i as f32 / SR as f32;
            let mut s = 0.0;
            for h in 1..=6 {
                let f = f0 * h as f32;
                if f < SR as f32 / 2.0 {
                    s += (2.0 * std::f32::consts::PI * f * t).sin() / h as f32;
                }
            }
            let env = (i.min(fade) as f32 / fade as f32).min((n - i).min(fade) as f32 / fade as f32);
            s * env
        })
        .collect()
}

/// Each chord is a list of MIDI notes held for `chord_secs`; the sequence is
/// repeated until `seconds`.
pub fn chords(progression: &[&[i32]], chord_secs: f32, seconds: f32, cents: f32) -> Vec<f32> {
    let total = (seconds * SR as f32) as usize;
    let mut out = vec![0.0f32; total];
    let mut pos = 0usize;
    let mut idx = 0usize;
    while pos < total {
        for &m in progression[idx % progression.len()] {
            add(&mut out, pos, &note(m as f32, chord_secs, cents), 0.3);
        }
        pos += (chord_secs * SR as f32) as usize;
        idx += 1;
    }
    normalise(&mut out);
    out
}

pub fn mix(a: &[f32], b: &[f32], gain_b: f32) -> Vec<f32> {
    let mut out: Vec<f32> = a.to_vec();
    for (o, s) in out.iter_mut().zip(b) {
        *o += s * gain_b;
    }
    normalise(&mut out);
    out
}
