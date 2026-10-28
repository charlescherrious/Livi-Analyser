//! Livi Analyser as a VST3 plugin, by CC Audio.
//!
//! The plugin passes audio through untouched. While you press "Listen" and
//! play the song, it records a mono copy, then hands it to the same analysis
//! engine the desktop app uses (`livi-core`) on a background thread.

mod analysis;
mod gui;
mod shared;

use std::num::NonZeroU32;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, EguiState};

use shared::Shared;

pub struct LiviAnalyser {
    params: Arc<LiviParams>,
    shared: Arc<Shared>,
}

#[derive(Params)]
struct LiviParams {
    /// Window size, saved with the project.
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,
}

impl Default for LiviAnalyser {
    fn default() -> Self {
        Self {
            params: Arc::new(LiviParams {
                editor_state: EguiState::from_size(640, 760),
            }),
            shared: Arc::new(Shared::new()),
        }
    }
}

impl Plugin for LiviAnalyser {
    const NAME: &'static str = "Livi Analyser";
    // The vendor name is what DAWs show as the manufacturer.
    const VENDOR: &'static str = "CC Audio";
    const URL: &'static str = "https://github.com/charlescherrious/Livi-Analyser";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            ..AudioIOLayout::const_default()
        },
    ];

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let shared = self.shared.clone();
        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            |ctx, _| gui::setup(ctx),
            move |ctx, _setter, _| gui::draw(ctx, &shared),
        )
    }

    fn initialize(
        &mut self,
        _layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.shared
            .sample_rate
            .store(buffer_config.sample_rate as u32, Ordering::Relaxed);
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let transport = context.transport();
        self.shared.host_playing.store(transport.playing, Ordering::Relaxed);
        self.shared.set_host_tempo(transport.tempo);

        // Only record while the song is actually playing. The audio itself is
        // left exactly as it came in.
        if transport.playing && self.shared.listening.load(Ordering::Relaxed) {
            for channels in buffer.iter_samples() {
                let n = channels.len().max(1) as f32;
                let mut sum = 0.0;
                for s in channels {
                    sum += *s;
                }
                self.shared.push(sum / n);
            }
        }
        ProcessStatus::Normal
    }
}

impl Vst3Plugin for LiviAnalyser {
    // Must stay the same forever once released: DAWs use it to find projects' plugins.
    const VST3_CLASS_ID: [u8; 16] = *b"CCAudioLiviAnlsr";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Analyzer];
}

nih_export_vst3!(LiviAnalyser);
