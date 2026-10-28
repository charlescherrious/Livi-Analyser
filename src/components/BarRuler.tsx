import { BEATS_PER_BAR, barMs, formatMs } from "../calculator";

interface Props {
  bpm: number;
  /** Selected delay time in ms. */
  ms: number;
  label: string;
}

const MAX_TICKS = 64;

/** One bar of 4/4 with the selected delay repeating across it. */
export function BarRuler({ bpm, ms, label }: Props) {
  const bar = barMs(bpm);
  const ticks: number[] = [];
  for (let t = ms; t < bar - 0.01 && ticks.length < MAX_TICKS; t += ms) ticks.push((t / bar) * 100);

  return (
    <figure className="ruler" aria-label={`One bar with ${label} repeats`}>
      <div className="ruler__track">
        {Array.from({ length: BEATS_PER_BAR }, (_, i) => (
          <span key={i} className="ruler__beat" style={{ left: `${(i / BEATS_PER_BAR) * 100}%` }}>
            <span className="ruler__num">{i + 1}</span>
          </span>
        ))}
        {ticks.map((left, i) => (
          <span key={i} className="ruler__tick" style={{ left: `${left}%` }} />
        ))}
      </div>
      <figcaption>
        {label}: {formatMs(ms)} ms, {ticks.length + 1} {ticks.length === 0 ? "hit" : "hits"} per bar
      </figcaption>
    </figure>
  );
}
