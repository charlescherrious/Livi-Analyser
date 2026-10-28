//! Command line check:
//!   cargo run --release -p livi-core --example analyze -- song.mp3 [min_bpm max_bpm]
//! Prints the result as JSON. Handy for batch benchmarking later.

use std::path::Path;
use std::sync::atomic::AtomicBool;

use livi_core::{analyze_file, AnalysisOptions};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: analyze <audio file> [min_bpm max_bpm]");
        std::process::exit(2);
    }
    let mut options = AnalysisOptions::default();
    if args.len() >= 3 {
        options.min_bpm = args[1].parse().expect("min_bpm must be a number");
        options.max_bpm = args[2].parse().expect("max_bpm must be a number");
    }
    let cancel = AtomicBool::new(false);
    match analyze_file(Path::new(&args[0]), &options, &cancel, &|_| {}) {
        Ok(result) => println!("{}", format_json(&result)),
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}

// Minimal JSON without pulling in serde_json: print the fields people care about.
fn format_json(r: &livi_core::AnalysisResult) -> String {
    format!(
        "{{\"file\":\"{}\",\"bpm\":{},\"bpmRaw\":{:.3},\"bpmConfidence\":{:.2},\"variableTempo\":{},\"key\":\"{}\",\"camelot\":\"{}\",\"keyConfidence\":{:.2},\"tuningCents\":{:.1},\"durationSecs\":{:.1},\"elapsedMs\":{}}}",
        r.file_name.replace('"', "'"),
        r.bpm,
        r.bpm_raw,
        r.bpm_confidence,
        r.variable_tempo,
        r.key.name,
        r.key.camelot,
        r.key.confidence,
        r.key.tuning_cents,
        r.duration_secs,
        r.elapsed_ms
    )
}
