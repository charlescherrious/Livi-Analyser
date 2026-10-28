import { useState, type CSSProperties } from "react";
import { double, formatBpm, halve, isThreeTwo, isValidBpm, roundBpm, threeHalves, twoThirds } from "../calculator";
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
  // A close runner-up that is exactly 3:2 (or 2:1) away is the classic
  // "triplet feel read as the beat" confusion, e.g. 135 vs 90.
  const rival = result.bpmCandidates.slice(1).find((c) => c.score >= 0.75 && isThreeTwo(c.bpm, result.bpmCandidates[0].bpm));

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
        <button type="button" className="btn" onClick={() => onBpm(twoThirds(bpm))} title="Use two thirds of this tempo, e.g. 135 to 90">
          ÷ 1.5
        </button>
        <button type="button" className="btn" onClick={() => onBpm(threeHalves(bpm))} title="Use one and a half times this tempo, e.g. 90 to 135">
          × 1.5
        </button>
        {edited && (
          <button type="button" className="btn btn--quiet" onClick={() => onBpm(null)}>
            Back to {formatBpm(result.bpm)}
          </button>
        )}
      </div>

      <p className={`status ${lowConfidence ? "status--warn" : "status--ok"}`}>{confidenceText(result.bpmConfidence)}</p>
      {lowConfidence && <p className="hint">Pick a genre above, or use Half or Double if it feels wrong.</p>}
      {rival && (
        <p className="hint hint--warn">
          Close call between {formatBpm(result.bpmCandidates[0].bpm)} and {formatBpm(rival.bpm)} BPM. Grooves with a
          triplet shuffle can read 1.5x too fast. Listen to the kick and snare, then click{" "}
          <button type="button" className="link" onClick={() => onBpm(roundBpm(rival.bpm))}>
            {formatBpm(rival.bpm)} BPM
          </button>{" "}
          if that is the pulse you tap.
        </p>
      )}
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
