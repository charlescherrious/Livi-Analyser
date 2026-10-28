import type { CSSProperties } from "react";

interface Props {
  /** Smaller strip shown once a track is loaded. */
  compact: boolean;
  dragging: boolean;
  desktop: boolean;
  onChoose: () => void;
}

// A fixed, hand-shaped silhouette so the zone reads as "audio" at a glance.
const BARS = [
  18, 26, 38, 30, 52, 44, 70, 58, 84, 66, 92, 74, 100, 80, 94, 70, 88, 62, 76, 54, 66, 46, 52, 36, 42, 28, 32, 22, 26, 16,
];

export function DropZone({ compact, dragging, desktop, onChoose }: Props) {
  const cls = ["drop", compact ? "drop--compact" : "", dragging ? "drop--over" : ""].join(" ").trim();

  return (
    <button type="button" className={cls} onClick={onChoose} disabled={!desktop}>
      {!compact && (
        <span className="drop__bars" aria-hidden="true">
          {BARS.map((h, i) => (
            <span key={i} style={{ "--h": `${h}%` } as CSSProperties} />
          ))}
        </span>
      )}
      <span className="drop__text">
        {compact ? (
          <>
            <strong>Drop another track</strong> or choose a file
          </>
        ) : (
          <>
            <strong className="drop__title">{dragging ? "Release to analyse" : "Drop a track here"}</strong>
            <span className="drop__sub">MP3, WAV, FLAC or AIFF. The file stays on your computer.</span>
            <span className="drop__action">Choose a file</span>
          </>
        )}
      </span>
    </button>
  );
}
