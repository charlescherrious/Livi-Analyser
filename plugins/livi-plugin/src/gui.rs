//! The plugin window (egui).

use std::sync::Arc;

use livi_core::calculator as calc;
use nih_plug_egui::egui::{self, Color32, RichText};

use crate::analysis;
use crate::shared::{Shared, Status, MAX_LISTEN_SECS, MIN_LISTEN_SECS, PRESETS};

const ACCENT: Color32 = Color32::from_rgb(140, 120, 255);
const WARN: Color32 = Color32::from_rgb(240, 180, 70);
const OK: Color32 = Color32::from_rgb(110, 210, 140);
const DIM: Color32 = Color32::from_gray(150);

fn confidence_text(c: f32) -> (&'static str, Color32) {
    if c >= 0.6 {
        ("Clear beat", OK)
    } else if c >= 0.35 {
        ("Fairly clear beat", OK)
    } else {
        ("Not sure about this tempo", WARN)
    }
}

pub fn setup(ctx: &egui::Context) {
    ctx.set_visuals(egui::Visuals::dark());
}

pub fn draw(ctx: &egui::Context, shared: &Arc<Shared>) {
    // Read everything first, so the lock is not held while drawing.
    let (status, result, over, preset) = match shared.ui.lock() {
        Ok(ui) => (ui.status.clone(), ui.result.clone(), ui.bpm_override, ui.preset),
        Err(_) => return,
    };

    let listening = shared.listening.load(std::sync::atomic::Ordering::Relaxed);
    let listened = shared.listened_secs();

    // Recording that reached its maximum length is analysed automatically.
    if listening && listened >= MAX_LISTEN_SECS - 0.5 {
        analysis::analyse_recording(shared);
    }
    if listening {
        ctx.request_repaint();
    }

    let mut new_preset: Option<usize> = None;
    let mut new_override: Option<Option<f64>> = None;
    let mut start_listen = false;
    let mut analyse_now = false;
    let mut open_file = false;
    let mut cancel = false;

    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Livi Analyser").size(22.0).strong());
                ui.label(RichText::new("by CC Audio").color(DIM));
            });
            ui.add_space(6.0);

            // --- Input ---------------------------------------------------
            ui.horizontal_wrapped(|ui| {
                egui::ComboBox::from_id_salt("genre")
                    .selected_text(PRESETS[preset].label)
                    .show_ui(ui, |ui| {
                        for (i, p) in PRESETS.iter().enumerate() {
                            if ui.selectable_label(i == preset, p.label).clicked() {
                                new_preset = Some(i);
                            }
                        }
                    });
                if listening {
                    let ready = listened >= MIN_LISTEN_SECS;
                    if ui.add_enabled(ready, egui::Button::new("Analyse now")).clicked() {
                        analyse_now = true;
                    }
                    if ui.button("Stop").clicked() {
                        cancel = true;
                    }
                } else if ui.button("Listen to this track").clicked() {
                    start_listen = true;
                }
                if analysis::FILE_DIALOG_AVAILABLE && ui.button("Open file...").clicked() {
                    open_file = true;
                }
            });
            ui.label(
                RichText::new("Pick the genre first: it stops half-time and triplet mix-ups.")
                    .small()
                    .color(DIM),
            );
            ui.add_space(4.0);

            // --- Status --------------------------------------------------
            match &status {
                Status::Idle if result.is_none() => {
                    ui.label("Press Listen, then play the song in your DAW (8 seconds or more).");
                }
                Status::Listening | Status::Idle | Status::Done if listening => {}
                Status::Analysing(what) => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(format!("Analysing {what}..."));
                    });
                }
                Status::Failed(msg) => {
                    ui.colored_label(WARN, msg);
                }
                _ => {}
            }
            if listening {
                let playing = shared.host_playing.load(std::sync::atomic::Ordering::Relaxed);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::ProgressBar::new((listened / MAX_LISTEN_SECS).min(1.0))
                            .desired_width(220.0)
                            .text(format!("Listening {listened:.0} s")),
                    );
                    if !playing {
                        ui.colored_label(WARN, "Press play in your DAW");
                    } else if listened < MIN_LISTEN_SECS {
                        ui.label(RichText::new("Keep playing...").color(DIM));
                    } else {
                        ui.colored_label(OK, "Enough to analyse");
                    }
                });
            }

            // --- Results -------------------------------------------------
            let Some(r) = &result else { return };
            ui.separator();

            let bpm = over.unwrap_or(r.bpm);
            let edited = over.is_some();

            ui.label(RichText::new("TEMPO").small().color(DIM));
            ui.horizontal(|ui| {
                let mut v = bpm;
                let resp = ui.add(
                    egui::DragValue::new(&mut v)
                        .speed(0.05)
                        .range(calc::MIN_BPM..=calc::MAX_BPM)
                        .max_decimals(2),
                );
                if resp.changed() && calc::is_valid_bpm(v) {
                    new_override = Some(Some(calc::round_bpm(v)));
                }
                ui.label(RichText::new("BPM").color(DIM));
            });
            ui.horizontal_wrapped(|ui| {
                if ui.button("Half").on_hover_text("Use half this tempo").clicked() {
                    new_override = Some(Some(calc::halve(bpm)));
                }
                if ui.button("Double").on_hover_text("Use double this tempo").clicked() {
                    new_override = Some(Some(calc::double(bpm)));
                }
                if edited && ui.button(format!("Back to {}", r.bpm)).clicked() {
                    new_override = Some(None);
                }
            });
            let (text, colour) = confidence_text(r.bpm_confidence);
            ui.colored_label(colour, text);
            if r.variable_tempo {
                ui.colored_label(WARN, "The tempo changes during this track, so this is an average.");
            }
            {
                let others: Vec<_> = r
                    .bpm_candidates
                    .iter()
                    .skip(1)
                    .filter(|c| c.score >= 0.6)
                    .collect();
                if !others.is_empty() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("Also found:").color(DIM));
                        for c in others {
                            if ui.small_button(format!("{:.1}", c.bpm)).clicked() {
                                new_override = Some(Some(calc::round_bpm(c.bpm)));
                            }
                        }
                    });
                }
            }
            if let Some(host) = shared.host_tempo() {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Project tempo: {host:.2}")).color(DIM));
                    if ui.small_button("Use").clicked() {
                        new_override = Some(Some(calc::round_bpm(host)));
                    }
                });
            }

            ui.add_space(8.0);
            ui.label(RichText::new("KEY").small().color(DIM));
            ui.horizontal(|ui| {
                ui.label(RichText::new(&r.key.name).size(22.0).strong());
                ui.label(RichText::new(&r.key.camelot).size(18.0).color(ACCENT).strong());
            });
            let (ktext, kcol) = if r.key.confidence >= 0.5 {
                ("Clear key", OK)
            } else if r.key.confidence >= 0.25 {
                ("Fairly clear key", OK)
            } else {
                ("Key is ambiguous: check the runner-ups", WARN)
            };
            ui.colored_label(kcol, ktext);
            if !r.key.alternatives.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Also possible:").color(DIM));
                    for a in r.key.alternatives.iter().take(3) {
                        ui.label(format!("{} ({})", a.name, a.camelot));
                    }
                });
            }

            // --- Calculator -----------------------------------------------
            ui.separator();
            ui.label(RichText::new(format!("DELAY TIMES at {bpm} BPM (ms)")).small().color(DIM));
            egui::Grid::new("delay").striped(true).num_columns(5).show(ui, |ui| {
                ui.label(RichText::new("Note").strong());
                ui.label(RichText::new("Straight").strong());
                ui.label(RichText::new("Dotted").strong());
                ui.label(RichText::new("Triplet").strong());
                ui.label(RichText::new("Hz").strong());
                ui.end_row();
                for row in calc::delay_rows(bpm) {
                    ui.label(row.label);
                    ui.label(calc::format_ms(row.straight));
                    ui.label(calc::format_ms(row.dotted));
                    ui.label(calc::format_ms(row.triplet));
                    ui.label(format!("{:.2}", calc::ms_to_hz(row.straight)));
                    ui.end_row();
                }
            });

            ui.add_space(8.0);
            ui.label(RichText::new("REVERB PRE-DELAY (ms)").small().color(DIM));
            egui::Grid::new("pre").striped(true).num_columns(3).show(ui, |ui| {
                for row in calc::pre_delay_rows(bpm) {
                    ui.label(row.label);
                    ui.label(calc::format_ms(row.ms));
                    ui.label(RichText::new(row.hint).color(DIM));
                    ui.end_row();
                }
            });
            ui.add_space(8.0);
            ui.label(RichText::new("REVERB DECAY / RT60 (ms)").small().color(DIM));
            egui::Grid::new("decay").striped(true).num_columns(3).show(ui, |ui| {
                for row in calc::decay_rows(bpm) {
                    ui.label(row.label);
                    ui.label(calc::format_ms(row.ms));
                    ui.label(RichText::new(row.hint).color(DIM));
                    ui.end_row();
                }
            });
        });
    });

    // Apply what the user did.
    if let Some(i) = new_preset {
        if let Ok(mut ui) = shared.ui.lock() {
            ui.preset = i;
        }
    }
    if let Some(o) = new_override {
        if let Ok(mut ui) = shared.ui.lock() {
            ui.bpm_override = o;
        }
    }
    if start_listen {
        if let Ok(mut ui) = shared.ui.lock() {
            ui.status = Status::Listening;
        }
        shared.start_listening();
    }
    if analyse_now {
        analysis::analyse_recording(shared);
    }
    if cancel {
        shared.stop_listening();
        if let Ok(mut ui) = shared.ui.lock() {
            ui.status = Status::Idle;
        }
    }
    if open_file {
        analysis::pick_and_analyse(shared);
    }
}
