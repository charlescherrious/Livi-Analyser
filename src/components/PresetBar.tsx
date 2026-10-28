import { CUSTOM_ID, PRESETS, validateRange } from "../presets";

interface Props {
  presetId: string;
  customMin: string;
  customMax: string;
  onPreset: (id: string) => void;
  onCustomChange: (min: string, max: string) => void;
  onCustomApply: () => void;
}

export function PresetBar({ presetId, customMin, customMax, onPreset, onCustomChange, onCustomApply }: Props) {
  const active = PRESETS.find((p) => p.id === presetId);
  const rangeError =
    presetId === CUSTOM_ID ? validateRange({ min: Number(customMin), max: Number(customMax) }) : null;

  return (
    <div className="presets">
      <div className="presets__row" role="group" aria-label="Genre range">
        {PRESETS.map((p) => (
          <button
            key={p.id}
            type="button"
            className={`chip ${p.id === presetId ? "chip--on" : ""}`}
            aria-pressed={p.id === presetId}
            onClick={() => onPreset(p.id)}
          >
            {p.label}
          </button>
        ))}
        <button
          type="button"
          className={`chip ${presetId === CUSTOM_ID ? "chip--on" : ""}`}
          aria-pressed={presetId === CUSTOM_ID}
          onClick={() => onPreset(CUSTOM_ID)}
        >
          Custom
        </button>
      </div>

      {presetId === CUSTOM_ID ? (
        <form
          className="presets__custom"
          onSubmit={(e) => {
            e.preventDefault();
            if (!rangeError) onCustomApply();
          }}
        >
          <label>
            From
            <input
              type="number"
              inputMode="decimal"
              value={customMin}
              onChange={(e) => onCustomChange(e.target.value, customMax)}
            />
          </label>
          <label>
            to
            <input
              type="number"
              inputMode="decimal"
              value={customMax}
              onChange={(e) => onCustomChange(customMin, e.target.value)}
            />
          </label>
          <span className="presets__unit">BPM</span>
          <button type="submit" className="btn" disabled={rangeError !== null}>
            Apply range
          </button>
          {rangeError && <span className="presets__error">{rangeError}</span>}
        </form>
      ) : (
        <p className="presets__note">{active?.note}. A narrower range avoids half-time and double-time mistakes.</p>
      )}
    </div>
  );
}
