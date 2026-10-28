// Genre presets set the BPM range the detector searches. A narrow range is
// the main defence against half-time / double-time mistakes (70 vs 140).
// Ranges are conventions: the producer's DAW tempo, not the "feel".

export interface Preset {
  id: string;
  label: string;
  min: number;
  max: number;
  note: string;
}

export const PRESETS: Preset[] = [
  { id: "auto", label: "Auto", min: 70, max: 180, note: "Searches 70 to 180 BPM" },
  { id: "afrobeats", label: "Afrobeats", min: 90, max: 125, note: "Searches 90 to 125 BPM" },
  { id: "dancehall", label: "Dancehall", min: 80, max: 115, note: "Searches 80 to 115 BPM" },
  { id: "amapiano", label: "Amapiano", min: 105, max: 125, note: "Searches 105 to 125 BPM" },
  { id: "trap", label: "Trap", min: 125, max: 175, note: "Searches 125 to 175 BPM" },
];

export const CUSTOM_ID = "custom";

export interface Range {
  min: number;
  max: number;
}

/** A range is usable when it is sensible and narrower than a doubling. */
export function validateRange(r: Range): string | null {
  if (!Number.isFinite(r.min) || !Number.isFinite(r.max)) return "Enter both numbers.";
  if (r.min < 40 || r.max > 300) return "Use values between 40 and 300.";
  if (r.max <= r.min) return "The maximum must be higher than the minimum.";
  if (r.max / r.min >= 2) return "Keep the range under a 2x spread so half and double tempo cannot both fit.";
  return null;
}
