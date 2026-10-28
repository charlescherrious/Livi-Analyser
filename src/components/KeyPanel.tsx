import type { CSSProperties } from "react";
import type { KeyCandidate, KeyOutput } from "../types";

function camelotHue(code: string): number {
  const n = parseInt(code, 10) || 1;
  return ((n - 1) * 30 + 10) % 360;
}

function CamelotBadge({ code }: { code: string }) {
  return (
    <span className="camelot" style={{ "--hue": camelotHue(code) } as CSSProperties} title="Camelot wheel code">
      {code}
    </span>
  );
}

function confidenceText(c: number): string {
  if (c >= 0.66) return "Clear key";
  if (c >= 0.33) return "Likely key";
  return "Not sure about this key";
}

function tuningText(cents: number): string | null {
  const rounded = Math.round(Math.abs(cents));
  if (rounded < 10) return null;
  return `Recorded about ${rounded} cents ${cents > 0 ? "sharp" : "flat"} of A440.`;
}

export function KeyPanel({ keyInfo }: { keyInfo: KeyOutput }) {
  const alternatives: KeyCandidate[] = keyInfo.alternatives.slice(1);
  const tuning = tuningText(keyInfo.tuningCents);
  const unsure = keyInfo.confidence < 0.33;

  return (
    <section className="panel key" aria-labelledby="key-h">
      <h2 id="key-h" className="panel__title">
        Key
      </h2>
      <div className="key__readout">
        <span className="key__name">{keyInfo.name}</span>
        <CamelotBadge code={keyInfo.camelot} />
      </div>
      <p className={`status ${unsure ? "status--warn" : "status--ok"}`}>{confidenceText(keyInfo.confidence)}</p>
      {unsure && <p className="hint">Check these too, and trust your ears.</p>}
      {alternatives.length > 0 && (
        <ul className="key__alts" aria-label="Other possible keys">
          {alternatives.map((a) => (
            <li key={a.name}>
              <span>{a.name}</span>
              <CamelotBadge code={a.camelot} />
            </li>
          ))}
        </ul>
      )}
      {tuning && <p className="hint">{tuning}</p>}
    </section>
  );
}
