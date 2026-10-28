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
cargo test -p livi-core -p livi-plugin --release   # audio engine + plugin
npm test                              # calculator (10 tests)
```

## Analyse a file from the command line (useful for benchmarking)

```
cargo run --release -p livi-core --example analyze -- "song.mp3"
cargo run --release -p livi-core --example analyze -- "song.mp3" 120 170   # restrict BPM range
```

It prints JSON, so you can loop over a folder and compare against your `labels.csv`.

## The plugin (VST3, by CC Audio)

`plugins/livi-plugin` is Livi Analyser as a plugin. It passes audio through untouched and shows up in your DAW under the manufacturer **CC Audio**.

How to use it:
1. Put it on the mixer track (or master) that plays the song.
2. Pick the genre, press **Listen to this track**, then press play in your DAW. After 8 seconds or more the **Analyse now** button turns on (it also stops by itself at 60 seconds).
3. Or press **Open file...** to analyse a song from your disk.
4. The BPM, key, Camelot code and the delay / reverb table appear. The Half, Double, ÷1.5 and ×1.5 buttons fix mix-ups, and **Use** next to "Project tempo" copies your DAW's tempo.

Note: dragging a file onto a plugin window is not supported yet (the plugin toolkit does not pass drops through), so use **Open file...**.

### Easiest: the one-click Windows installer

Download **LiviAnalyser-Setup-Windows** from the GitHub Actions run and unzip it to get `LiviAnalyser-Setup.exe`. Run it, then tick what you want:
- **Desktop app**
- **VST3 plugin** (installed into `C:\Program Files\Common Files\VST3\CC Audio\`)

Then rescan plugins in your DAW. Windows may warn "Windows protected your PC" because the installer is not signed yet: click **More info > Run anyway**. The desktop app is removed from Windows **Settings > Apps**; the plugin is removed by the Livi Analyser entry there too.

### Install the plugin by hand

Download the `livi-analyser-vst3-...` artifact from the GitHub Actions run (same place as the installers) and unzip it. Inside is a folder called **CC Audio**.

**Windows (FL Studio, Ableton, Reaper ...)**
1. Copy the whole **CC Audio** folder into `C:\Program Files\Common Files\VST3\` (click Continue if Windows asks for permission).
2. FL Studio: **Options > Manage plugins**. Make sure `C:\Program Files\Common Files\VST3` is in the plugin search paths, then click **Find installed plugins**.
3. In a mixer insert slot, click the arrow > pick the plugin from the list. It is grouped under **CC Audio**.

**macOS**
1. Copy the **CC Audio** folder into `/Library/Audio/Plug-Ins/VST3/` (or `~/Library/Audio/Plug-Ins/VST3/`).
2. The plugin is not signed yet, so macOS may say it is "damaged". Open **Terminal** and run: `xattr -cr "/Library/Audio/Plug-Ins/VST3/CC Audio"`
3. Rescan plugins in your DAW.

Build it yourself: `cargo xtask bundle livi-plugin --release` (output in `target/bundled/`).

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
