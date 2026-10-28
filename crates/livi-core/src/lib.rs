//! Livi Analyser core: offline decoding, BPM detection and key detection.
//!
//! Everything here is plain Rust with no UI or network code, so it can be
//! tested on its own (`cargo test -p livi-core --release`) and reused from a
//! command line tool or a benchmark script.

pub mod calculator;
pub mod camelot;
pub mod decode;
pub mod dsp;
pub mod key;
pub mod pipeline;
pub mod tempo;

pub use pipeline::{
    analyze_file, analyze_live, analyze_samples, AnalysisError, AnalysisOptions, AnalysisResult, Progress, Stage,
};
