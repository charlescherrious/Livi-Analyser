use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use livi_core::{analyze_file, AnalysisOptions, AnalysisResult, Progress};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

/// Holds the cancel flag of the analysis that is currently running, so a new
/// drop (or the Cancel button) can stop it.
#[derive(Default)]
struct AnalysisState {
    current: Mutex<Option<Arc<AtomicBool>>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnalyzeRequest {
    request_id: u64,
    path: String,
    min_bpm: Option<f64>,
    max_bpm: Option<f64>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct ProgressEvent {
    request_id: u64,
    #[serde(flatten)]
    progress: Progress,
}

/// Decodes and analyses one file on a background thread. The UI thread stays
/// free; progress arrives as `analysis-progress` events.
#[tauri::command]
async fn analyze_audio_file(
    app: AppHandle,
    state: State<'_, AnalysisState>,
    request: AnalyzeRequest,
) -> Result<AnalysisResult, String> {
    let token = Arc::new(AtomicBool::new(false));
    {
        let mut slot = state.current.lock().map_err(|e| e.to_string())?;
        if let Some(previous) = slot.replace(token.clone()) {
            previous.store(true, Ordering::SeqCst);
        }
    }

    let defaults = AnalysisOptions::default();
    let options = AnalysisOptions {
        min_bpm: request.min_bpm.unwrap_or(defaults.min_bpm),
        max_bpm: request.max_bpm.unwrap_or(defaults.max_bpm),
    };
    if !(options.min_bpm > 20.0 && options.max_bpm > options.min_bpm * 1.05) {
        return Err("The BPM range is not valid: the maximum must be above the minimum.".into());
    }

    let request_id = request.request_id;
    let path = PathBuf::from(request.path);

    let outcome = tauri::async_runtime::spawn_blocking(move || {
        analyze_file(&path, &options, &token, &|progress| {
            let _ = app.emit("analysis-progress", ProgressEvent { request_id, progress });
        })
    })
    .await
    .map_err(|e| e.to_string())?;

    outcome.map_err(|e| e.to_string())
}

#[tauri::command]
fn cancel_analysis(state: State<'_, AnalysisState>) {
    if let Ok(slot) = state.current.lock() {
        if let Some(token) = slot.as_ref() {
            token.store(true, Ordering::SeqCst);
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AnalysisState::default())
        .invoke_handler(tauri::generate_handler![analyze_audio_file, cancel_analysis])
        .run(tauri::generate_context!())
        .expect("error while running Livi Analyser");
}
