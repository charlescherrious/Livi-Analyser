// Mirrors the JSON the Rust side sends (serde camelCase).

export type Stage = "decoding" | "tempo" | "key" | "done";

export interface ProgressEvent {
  requestId: number;
  stage: Stage;
  fraction: number;
}

export interface TempoCandidate {
  bpm: number;
  score: number;
}

export interface KeyCandidate {
  tonic: number;
  mode: "major" | "minor";
  name: string;
  short: string;
  camelot: string;
  score: number;
}

export interface KeyOutput {
  tonic: number;
  mode: "major" | "minor";
  name: string;
  short: string;
  camelot: string;
  confidence: number;
  strength: number;
  tuningCents: number;
  alternatives: KeyCandidate[];
}

export interface AnalysisResult {
  fileName: string;
  durationSecs: number;
  sourceSampleRate: number;
  channels: number;
  bpm: number;
  bpmRaw: number;
  bpmConfidence: number;
  variableTempo: boolean;
  bpmCandidates: TempoCandidate[];
  key: KeyOutput;
  elapsedMs: number;
}

export interface AnalyzeRequest {
  requestId: number;
  path: string;
  minBpm: number;
  maxBpm: number;
}
