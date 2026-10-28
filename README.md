# Livi Analyser

Offline desktop app (Windows + macOS). Drop in an MP3, WAV, FLAC or AIFF file and it works out the **BPM** and **musical key** (standard + Camelot notation), then shows a **delay / reverb timing table** in milliseconds. Nothing is uploaded; no internet is needed.

- UI: React + TypeScript (`src/`)
- Core engine: Rust (`crates/livi-core`): Symphonia decoding, rustfft analysis
- Shell: Tauri 2 (`src-tauri/`)

Open source. Tempo convention: the reported BPM is the **producer / DAW BPM** (a trap beat at 140 shows 140, not 70). Use the ½× / 2× buttons if a track is read at the wrong octave.

---

## Setup from zero (do this once per computer)

### Windows 10/11

1. **Microsoft C++ Build Tools.** Download "Build Tools for Visual Studio" from visualstudio.microsoft.com/visual-cpp-build-tools. In the installer tick **"Desktop development with C++"** and click Install. Restart when asked.
2. **WebView2.** Already installed on Windows 11. On Windows 10, get the "Evergreen Bootstrapper" from Microsoft's WebView2 page.
3. **Rust.** Go to rustup.rs, download and run `rustup-init.exe`, press Enter to accept the defaults. Close and reopen your terminal.
4. **Node.js.** Download the **LTS** version from nodejs.org and install with defaults.
5. Check everything. Open **PowerShell** and run:
   ```
   rustc --version
   node --version
   npm --version
   ```
   Each should print a version number.

### macOS

1. Open **Terminal** and run `xcode-select --install`, then click Install.
2. Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`, press Enter for the default, then close and reopen Terminal.
3. Install Node.js LTS from nodejs.org.
4. Check with `rustc --version` and `node --version`.

---

## Run it

In a terminal, inside this folder:

```
npm install
npm run tauri dev
```

The first run compiles for a few minutes. After that, a window opens. Drag an audio file onto it.

## Build the installer

```
npm run tauri build
```

The installer appears in `src-tauri/target/release/bundle/` (or `target/release/bundle/` at the project root). You can only build the Windows installer on Windows and the macOS one on a Mac. Giving the macOS app to other people needs Apple code-signing and notarization (an Apple Developer account).

## Run the tests

```
cargo test -p livi-core --release     # audio engine (22 tests)
npm test                              # calculator (10 tests)
```

## Analyse a file from the command line (useful for benchmarking)

```
cargo run --release -p livi-core --example analyze -- "song.mp3"
cargo run --release -p livi-core --example analyze -- "song.mp3" 120 170   # restrict BPM range
```

It prints JSON, so you can loop over a folder and compare against your `labels.csv`.

## How it works

```
drop file → Tauri drag-drop event (real path) → analyze_audio_file command
  → background thread: decode (Symphonia) → mono → resample → tempo + key
  → "analysis-progress" events → progress bar
  → result { bpm, confidence, key, camelot, ... } → UI
  → calculator.ts recomputes delay / pre-delay / RT60 instantly on any BPM change
```

A new drop cancels the analysis in progress.

## Roadmap

1. Build the genre benchmark (afrobeats, dancehall, amapiano, trap) with your own labeled tracks, and tune the detectors on it.
2. If the benchmark shows a gap, add a second detector (Beat This! via ONNX, or Essentia) behind the same interface and vote.
