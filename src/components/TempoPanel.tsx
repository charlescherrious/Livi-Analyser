import { useState, type CSSProperties } from "react";
import { double, formatBpm, halve, isValidBpm, roundBpm } from "../calculator";
import type { AnalysisResult } from "../types";

interface Props {
  result: AnalysisResult;
  bpm: number;
  edited: boolean;
  onBpm: (bpm: number | null) => void;
}

function confidenceText(c: number): string {
  if (c >= 0.6) return "Clear beat";
  if (c >= 0.35) return "Fairly clear beat";
  return "Not sure about this tempo";
}

export function TempoPanel({ result, bpm, edited, onBpm }: Props) {
  const [draft, setDraft] = useState<string | null>(null);
  const lowConfidence = result.bpmConfidence < 0.35;
  const others = result.bpmCandidates.slice(1).filter((c) => c.score >= 0.6);

  return (
    <section className="panel tempo" aria-labelledby="tempo-h">
      <h2 id="tempo-h" className="panel__title">
        Tempo
      </h2>

      <div className="tempo__readout">
        <span
          className="lamp"
          style={{ "--beat": `${60 / bpm}s` } as CSSProperties}
          title="Blinks once per beat"
          aria-hidden="true"
        />
        <input
          className="tempo__number"
          type="number"
          inputMode="decimal"
          step="0.1"
          min="20"
          max="400"
          aria-label="BPM"
          value={draft ?? formatBpm(bpm)}
          onChange={(e) => {
            setDraft(e.target.value);
            const v = Number(e.target.value);
            if (e.target.value !== "" && isValidBpm(v)) onBpm(roundBpm(v));
          }}
          onBlur={() => setDraft(null)}
        />
        <span className="tempo__unit">BPM</span>
      </div>

      <div className="tempo__controls">
        <button type="button" className="btn" onClick={() => onBpm(halve(bpm))} title="Use half this tempo">
          Half
        </button>
        <button type="button" className="btn" onClick={() => onBpm(double(bpm))} title="Use double this tempo">
          Double
        </button>
        {edited && (
          <button type="button" className="btn btn--quiet" onClick={() => onBpm(null)}>
            Back to {formatBpm(result.bpm)}
          </button>
        )}
      </div>

      <p className={`status ${lowConfidence ? "status--warn" : "status--ok"}`}>{confidenceText(result.bpmConfidence)}</p>
      {lowConfidence && <p className="hint">Pick a genre above, or use Half or Double if it feels wrong.</p>}
      {result.variableTempo && (
        <p className="hint hint--warn">The tempo changes during this track, so this is an average.</p>
      )}
      {others.length > 0 && (
        <p className="hint">
          Also found:{" "}
          {others.map((c) => (
            <button key={c.bpm} type="button" className="link" onClick={() => onBpm(roundBpm(c.bpm))}>
              {formatBpm(c.bpm)} BPM
            </button>
          ))}
        </p>
      )}
    </section>
  );
}
