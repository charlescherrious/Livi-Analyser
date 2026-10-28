import { useState } from "react";
import {
  NOTE_VALUES,
  barMs,
  beatMs,
  decayRows,
  delayRows,
  formatHz,
  formatMs,
  formatSeconds,
  msToHz,
  noteMs,
  preDelayRows,
  type Variant,
} from "../calculator";
import { BarRuler } from "./BarRuler";

interface Props {
  bpm: number;
  notify: (message: string) => void;
}

type Unit = "ms" | "hz";

const VARIANTS: { id: Variant; label: string }[] = [
  { id: "straight", label: "Straight" },
  { id: "dotted", label: "Dotted" },
  { id: "triplet", label: "Triplet" },
];

async function copy(text: string, notify: (m: string) => void, unit: string) {
  try {
    await navigator.clipboard.writeText(text);
    notify(`Copied ${text} ${unit}`);
  } catch {
    notify("Could not copy. Select the number and copy it by hand.");
  }
}

export function Calculator({ bpm, notify }: Props) {
  const [unit, setUnit] = useState<Unit>("ms");
  const [selected, setSelected] = useState<{ id: string; variant: Variant }>({ id: "1/8", variant: "straight" });

  const rows = delayRows(bpm);
  const note = NOTE_VALUES.find((n) => n.id === selected.id) ?? NOTE_VALUES[3];
  const selectedMs = noteMs(bpm, note.beats, selected.variant);
  const selectedLabel = `${note.label} ${selected.variant}`;

  const show = (ms: number) => (unit === "ms" ? formatMs(ms) : formatHz(msToHz(ms)));

  return (
    <section className="panel calc" aria-labelledby="calc-h">
      <div className="calc__head">
        <h2 id="calc-h" className="panel__title">
          Delay and reverb times
        </h2>
        <div className="seg" role="group" aria-label="Unit">
          <button type="button" className={unit === "ms" ? "seg__on" : ""} aria-pressed={unit === "ms"} onClick={() => setUnit("ms")}>
            ms
          </button>
          <button type="button" className={unit === "hz" ? "seg__on" : ""} aria-pressed={unit === "hz"} onClick={() => setUnit("hz")}>
            Hz
          </button>
        </div>
      </div>

      <dl className="calc__stats">
        <div>
          <dt>Beat</dt>
          <dd>{formatMs(beatMs(bpm))} ms</dd>
        </div>
        <div>
          <dt>Bar (4/4)</dt>
          <dd>{formatMs(barMs(bpm))} ms</dd>
        </div>
      </dl>

      <BarRuler bpm={bpm} ms={selectedMs} label={selectedLabel} />

      <h3 className="calc__sub">Delay</h3>
      <table className="grid">
        <thead>
          <tr>
            <th scope="col">Note</th>
            {VARIANTS.map((v) => (
              <th key={v.id} scope="col" className="num">
                {v.label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((r) => (
            <tr key={r.id}>
              <th scope="row">{r.label}</th>
              {VARIANTS.map((v) => {
                const ms = r[v.id];
                const on = selected.id === r.id && selected.variant === v.id;
                return (
                  <td key={v.id} className="num">
                    <button
                      type="button"
                      className={`cell ${on ? "cell--on" : ""}`}
                      title="Click to show on the ruler and copy"
                      onClick={() => {
                        setSelected({ id: r.id, variant: v.id });
                        void copy(show(ms), notify, unit);
                      }}
                    >
                      {show(ms)}
                    </button>
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>

      <div className="calc__reverb">
        <div>
          <h3 className="calc__sub">Reverb pre-delay</h3>
          <ReverbTable rows={preDelayRows(bpm)} unit={unit} show={show} notify={notify} />
        </div>
        <div>
          <h3 className="calc__sub">Reverb decay (RT60)</h3>
          <ReverbTable rows={decayRows(bpm)} unit={unit} show={show} notify={notify} showSeconds />
        </div>
      </div>

      <p className="hint">Click any time to copy it. Pre-delay and decay are starting points: adjust by ear.</p>
    </section>
  );
}

interface ReverbTableProps {
  rows: ReturnType<typeof preDelayRows>;
  unit: Unit;
  show: (ms: number) => string;
  notify: (m: string) => void;
  showSeconds?: boolean;
}

function ReverbTable({ rows, unit, show, notify, showSeconds }: ReverbTableProps) {
  return (
    <table className="grid">
      <tbody>
        {rows.map((r) => (
          <tr key={r.id}>
            <th scope="row">
              {r.label}
              <span className="grid__hint">{r.hint}</span>
            </th>
            <td className="num">
              <button type="button" className="cell" onClick={() => void copy(show(r.ms), notify, unit)}>
                {show(r.ms)}
              </button>
              {showSeconds && unit === "ms" && <span className="grid__aside">{formatSeconds(r.ms)} s</span>}
            </td>
          </tr>
        ))}
      </tbody>
    </table>
  );
}
