import { useCallback, useEffect, useRef, useState } from "react";
import { Calculator } from "./components/Calculator";
import { DropZone } from "./components/DropZone";
import { KeyPanel } from "./components/KeyPanel";
import { PresetBar } from "./components/PresetBar";
import { TempoPanel } from "./components/TempoPanel";
import { CUSTOM_ID, PRESETS, validateRange, type Range } from "./presets";
import {
  analyzeFile,
  baseName,
  cancelAnalysis,
  chooseAudioFile,
  hasAudioExtension,
  isDesktopApp,
  onAnalysisProgress,
  onFileDrop,
} from "./tauri";
import type { AnalysisResult } from "./types";

type View =
  | { status: "idle" }
  | { status: "analysing"; path: string; decode: number; tempoDone: boolean; keyDone: boolean }
  | { status: "done"; path: string; result: AnalysisResult }
  | { status: "error"; path?: string; message: string };

function formatDuration(secs: number): string {
  const m = Math.floor(secs / 60);
  const s = Math.round(secs % 60);
  return `${m}:${String(s).padStart(2, "0")}`;
}

function describeSource(r: AnalysisResult): string {
  const khz = (r.sourceSampleRate / 1000).toFixed(r.sourceSampleRate % 1000 === 0 ? 0 : 1);
  const ch = r.channels === 1 ? "mono" : r.channels === 2 ? "stereo" : `${r.channels} channels`;
  return `${formatDuration(r.durationSecs)} long, ${khz} kHz, ${ch}`;
}

export default function App() {
  const desktop = isDesktopApp();
  const [view, setView] = useState<View>({ status: "idle" });
  const [dragging, setDragging] = useState(false);
  const [presetId, setPresetId] = useState("auto");
  const [customMin, setCustomMin] = useState("100");
  const [customMax, setCustomMax] = useState("140");
  const [bpmOverride, setBpmOverride] = useState<number | null>(null);
  const [toast, setToast] = useState<string | null>(null);

  const seq = useRef(0);
  const toastTimer = useRef<number | undefined>(undefined);

  const notify = useCallback((message: string) => {
    setToast(message);
    window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 1800);
  }, []);

  const currentRange = useCallback(
    (id: string = presetId, min: string = customMin, max: string = customMax): Range | null => {
      if (id === CUSTOM_ID) {
        const r = { min: Number(min), max: Number(max) };
        return validateRange(r) === null ? r : null;
      }
      const p = PRESETS.find((x) => x.id === id) ?? PRESETS[0];
      return { min: p.min, max: p.max };
    },
    [presetId, customMin, customMax],
  );

  // Latest values for the long-lived drop listener.
  const rangeRef = useRef<Range | null>(currentRange());
  rangeRef.current = currentRange();

  const run = useCallback(async (path: string, range: Range) => {
    const id = ++seq.current;
    setView({ status: "analysing", path, decode: 0, tempoDone: false, keyDone: false });
    try {
      const result = await analyzeFile({ requestId: id, path, minBpm: range.min, maxBpm: range.max });
      if (id !== seq.current) return; // a newer file replaced this one
      setBpmOverride(null);
      setView({ status: "done", path, result });
    } catch (e) {
      if (id !== seq.current) return;
      const message = String(e);
      if (message.includes("cancelled")) return;
      setView({ status: "error", path, message });
    }
  }, []);

  const startFile = useCallback(
    (path: string) => {
      if (!hasAudioExtension(path)) {
        setView({
          status: "error",
          message: `${baseName(path)} is not a supported file. Use MP3, WAV, FLAC or AIFF.`,
        });
        return;
      }
      const range = rangeRef.current;
      if (!range) {
        notify("Fix the BPM range first.");
        return;
      }
      void run(path, range);
    },
    [notify, run],
  );

  const startFileRef = useRef(startFile);
  startFileRef.current = startFile;

  // Desktop events: dropped files and progress from the Rust side.
  useEffect(() => {
    if (!desktop) return;
    let disposed = false;
    const unlisteners: Array<() => void> = [];
    const keep = (p: Promise<() => void>) =>
      p.then((un) => (disposed ? un() : unlisteners.push(un))).catch(() => undefined);

    keep(
      onFileDrop({
        onHover: setDragging,
        onDrop: (paths) => {
          if (paths.length > 0) startFileRef.current(paths[0]);
        },
      }),
    );
    keep(
      onAnalysisProgress((e) => {
        if (e.requestId !== seq.current) return;
        setView((v) => {
          if (v.status !== "analysing") return v;
          if (e.stage === "decoding") return { ...v, decode: e.fraction };
          if (e.stage === "tempo") return { ...v, decode: 1, tempoDone: true };
          if (e.stage === "key") return { ...v, decode: 1, keyDone: true };
          return v;
        });
      }),
    );
    return () => {
      disposed = true;
      unlisteners.forEach((un) => un());
    };
  }, [desktop]);

  const choose = async () => {
    try {
      const path = await chooseAudioFile();
      if (path) startFile(path);
    } catch (e) {
      setView({ status: "error", message: String(e) });
    }
  };

  const cancel = () => {
    seq.current += 1; // ignore anything the old run still reports
    void cancelAnalysis();
    setView({ status: "idle" });
  };

  const currentPath = view.status === "idle" ? undefined : view.path;

  const rerunWith = (range: Range | null) => {
    if (range && currentPath) void run(currentPath, range);
  };

  const onPreset = (id: string) => {
    setPresetId(id);
    if (id !== CUSTOM_ID) rerunWith(currentRange(id));
  };

  const activeBpm = view.status === "done" ? (bpmOverride ?? view.result.bpm) : null;
  const busy = view.status === "analysing";
  const loaded = view.status !== "idle";

  return (
    <div className="app">
      <header className="top">
        <h1 className="brand">Livi Analyser</h1>
        <p className="top__tag">Tempo, key and delay times for any track, fully offline.</p>
      </header>

      {!desktop && (
        <p className="banner" role="alert">
          This page is running in a web browser, where files cannot be analysed. Start the desktop app with{" "}
          <code>npm run tauri dev</code>.
        </p>
      )}

      <PresetBar
        presetId={presetId}
        customMin={customMin}
        customMax={customMax}
        onPreset={onPreset}
        onCustomChange={(min, max) => {
          setCustomMin(min);
          setCustomMax(max);
        }}
        onCustomApply={() => rerunWith(currentRange(CUSTOM_ID))}
      />

      <DropZone compact={loaded} dragging={dragging} desktop={desktop} onChoose={choose} />

      {view.status === "analysing" && (
        <div className="progress" role="status" aria-live="polite">
          <div className="progress__line">
            <span>
              {view.decode < 1 ? "Reading audio" : "Finding tempo and key"}: {baseName(view.path)}
            </span>
            <button type="button" className="btn btn--quiet" onClick={cancel}>
              Cancel
            </button>
          </div>
          <div className="progress__bar" aria-hidden="true">
            <span
              style={{
                width: `${Math.round((0.7 * view.decode + 0.15 * Number(view.tempoDone) + 0.15 * Number(view.keyDone)) * 100)}%`,
              }}
            />
          </div>
        </div>
      )}

      {view.status === "error" && (
        <div className="error" role="alert">
          <strong>Could not analyse this file.</strong>
          <span>{view.message}</span>
        </div>
      )}

      {view.status === "done" && activeBpm !== null && (
        <>
          <div className="file" aria-live="polite" data-busy={busy}>
            <span className="file__name" title={view.path}>
              {view.result.fileName}
            </span>
            <span className="file__meta">{describeSource(view.result)}</span>
          </div>
          <div className="results">
            <div className="results__left">
              <TempoPanel
                result={view.result}
                bpm={activeBpm}
                edited={bpmOverride !== null}
                onBpm={(b) => setBpmOverride(b)}
              />
              <KeyPanel keyInfo={view.result.key} />
            </div>
            <Calculator bpm={activeBpm} notify={notify} />
          </div>
        </>
      )}

      <div className="toast" role="status" aria-live="polite">
        {toast}
      </div>
    </div>
  );
}
