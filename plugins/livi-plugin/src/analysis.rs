//! Background analysis jobs. Each runs on its own thread so neither the audio
//! thread nor the GUI ever waits for them.

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use livi_core::{analyze_file, analyze_live};

use crate::shared::{Shared, Status};

fn finish(shared: &Shared, outcome: Result<livi_core::AnalysisResult, livi_core::AnalysisError>) {
    let Ok(mut ui) = shared.ui.lock() else { return };
    match outcome {
        Ok(result) => {
            ui.result = Some(result);
            ui.bpm_override = None;
            ui.status = Status::Done;
        }
        Err(livi_core::AnalysisError::Cancelled) => ui.status = Status::Idle,
        Err(e) => ui.status = Status::Failed(e.to_string()),
    }
}

/// Analyse what the plugin has heard so far.
pub fn analyse_recording(shared: &Arc<Shared>) {
    shared.stop_listening();
    let samples = shared.snapshot();
    let sr = shared.sample_rate.load(Ordering::Relaxed);
    let options = shared.range();
    if let Ok(mut ui) = shared.ui.lock() {
        ui.status = Status::Analysing("the recording".into());
    }
    shared.cancel.store(false, Ordering::SeqCst);

    let shared = shared.clone();
    std::thread::spawn(move || {
        let outcome = analyze_live(&samples, sr, &options, &shared.cancel, &|_| {});
        finish(&shared, outcome);
    });
}

/// Analyse an audio file chosen by the user.
#[cfg_attr(not(any(target_os = "windows", target_os = "macos")), allow(dead_code))]
pub fn analyse_file_at(shared: &Arc<Shared>, path: PathBuf) {
    shared.stop_listening();
    let options = shared.range();
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "the file".into());
    if let Ok(mut ui) = shared.ui.lock() {
        ui.status = Status::Analysing(name);
    }
    shared.cancel.store(false, Ordering::SeqCst);

    let shared = shared.clone();
    std::thread::spawn(move || {
        let outcome = analyze_file(&path, &options, &shared.cancel, &|_| {});
        finish(&shared, outcome);
    });
}

/// Ask the user for a file. Windows runs the dialog on a helper thread so the
/// host is not blocked; macOS needs the main thread, which is the GUI thread.
pub fn pick_and_analyse(shared: &Arc<Shared>) {
    #[cfg(target_os = "windows")]
    {
        let shared = shared.clone();
        std::thread::spawn(move || {
            if let Some(path) = pick() {
                analyse_file_at(&shared, path);
            }
        });
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(path) = pick() {
            analyse_file_at(shared, path);
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = shared; // no file dialog on this platform
    }
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
fn pick() -> Option<PathBuf> {
    rfd::FileDialog::new()
        .set_title("Choose a song to analyse")
        .add_filter("Audio", &["mp3", "wav", "flac", "aif", "aiff"])
        .pick_file()
}

pub const FILE_DIALOG_AVAILABLE: bool = cfg!(any(target_os = "windows", target_os = "macos"));
