// Tempo-synced timing maths. Pure functions, no UI, fully unit tested.
// Assumes 4/4 (one bar = four quarter-note beats).

export type Variant = "straight" | "dotted" | "triplet";

export interface NoteValue {
  id: string;
  label: string;
  /** Length in quarter-note beats. */
  beats: number;
}

export const BEATS_PER_BAR = 4;

export const VARIANT_FACTOR: Record<Variant, number> = {
  straight: 1,
  dotted: 1.5,
  triplet: 2 / 3,
};

export const NOTE_VALUES: NoteValue[] = [
  { id: "1/1", label: "1/1", beats: 4 },
  { id: "1/2", label: "1/2", beats: 2 },
  { id: "1/4", label: "1/4", beats: 1 },
  { id: "1/8", label: "1/8", beats: 0.5 },
  { id: "1/16", label: "1/16", beats: 0.25 },
  { id: "1/32", label: "1/32", beats: 0.125 },
  { id: "1/64", label: "1/64", beats: 0.0625 },
];

export interface DelayRow {
  id: string;
  label: string;
  straight: number;
  dotted: number;
  triplet: number;
}

export interface ReverbRow {
  id: string;
  label: string;
  hint: string;
  ms: number;
}

/** Common starting points. Start here, then adjust by ear. */
const PRE_DELAY_SETTINGS = [
  { id: "pd-64", label: "1/64", beats: 1 / 16, hint: "Very tight" },
  { id: "pd-32", label: "1/32", beats: 1 / 8, hint: "Tight" },
  { id: "pd-16t", label: "1/16 triplet", beats: (1 / 4) * (2 / 3), hint: "Light gap" },
  { id: "pd-16", label: "1/16", beats: 1 / 4, hint: "Clear gap" },
];

const DECAY_SETTINGS = [
  { id: "rt-8", label: "1/8", beats: 0.5, hint: "Very short" },
  { id: "rt-4", label: "1/4", beats: 1, hint: "Short" },
  { id: "rt-2", label: "1/2", beats: 2, hint: "Medium" },
  { id: "rt-1bar", label: "1 bar", beats: 4, hint: "Long" },
  { id: "rt-2bar", label: "2 bars", beats: 8, hint: "Very long" },
];

export const MIN_BPM = 20;
export const MAX_BPM = 400;

export function isValidBpm(bpm: number): boolean {
  return Number.isFinite(bpm) && bpm >= MIN_BPM && bpm <= MAX_BPM;
}

/** Length of one quarter-note beat in milliseconds. */
export function beatMs(bpm: number): number {
  return 60000 / bpm;
}

export function barMs(bpm: number): number {
  return beatMs(bpm) * BEATS_PER_BAR;
}

export function noteMs(bpm: number, beats: number, variant: Variant = "straight"): number {
  return beatMs(bpm) * beats * VARIANT_FACTOR[variant];
}

export function msToHz(ms: number): number {
  return 1000 / ms;
}

export function delayRows(bpm: number): DelayRow[] {
  return NOTE_VALUES.map((n) => ({
    id: n.id,
    label: n.label,
    straight: noteMs(bpm, n.beats, "straight"),
    dotted: noteMs(bpm, n.beats, "dotted"),
    triplet: noteMs(bpm, n.beats, "triplet"),
  }));
}

export function preDelayRows(bpm: number): ReverbRow[] {
  return PRE_DELAY_SETTINGS.map((s) => ({
    id: s.id,
    label: s.label,
    hint: s.hint,
    ms: beatMs(bpm) * s.beats,
  }));
}

export function decayRows(bpm: number): ReverbRow[] {
  return DECAY_SETTINGS.map((s) => ({
    id: s.id,
    label: s.label,
    hint: s.hint,
    ms: beatMs(bpm) * s.beats,
  }));
}

/** Round to two decimals, the precision most delay plugins accept. */
export function roundBpm(bpm: number): number {
  return Math.round(bpm * 100) / 100;
}

export function halve(bpm: number): number {
  return roundBpm(bpm / 2);
}

export function double(bpm: number): number {
  return roundBpm(bpm * 2);
}

/** 135 -> 90. Fixes the "triplet feel read as the beat" mistake. */
export function twoThirds(bpm: number): number {
  return roundBpm((bpm * 2) / 3);
}

/** 90 -> 135. The reverse correction. */
export function threeHalves(bpm: number): number {
  return roundBpm(bpm * 1.5);
}

/** True when two tempos are a 3:2 pair (within 3%), e.g. 90 and 135. */
export function isThreeTwo(a: number, b: number): boolean {
  const r = Math.max(a, b) / Math.min(a, b);
  return Math.abs(r / 1.5 - 1) <= 0.03;
}

export function formatMs(ms: number): string {
  return ms.toFixed(2);
}

export function formatHz(hz: number): string {
  return hz.toFixed(3);
}

export function formatSeconds(ms: number): string {
  return (ms / 1000).toFixed(2);
}

export function formatBpm(bpm: number): string {
  return Number.isInteger(bpm) ? String(bpm) : bpm.toFixed(2).replace(/0+$/, "").replace(/\.$/, "");
}
