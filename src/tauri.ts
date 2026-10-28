// The only file that talks to the desktop shell. Everything else stays plain React.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import type { AnalysisResult, AnalyzeRequest, ProgressEvent } from "./types";

export const AUDIO_EXTENSIONS = ["mp3", "wav", "flac", "aif", "aiff"];

/** False when the page is opened in a normal browser instead of the app. */
export function isDesktopApp(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

export function hasAudioExtension(path: string): boolean {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  return AUDIO_EXTENSIONS.includes(ext);
}

export function baseName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

export async function chooseAudioFile(): Promise<string | null> {
  const picked = await open({
    multiple: false,
    directory: false,
    filters: [{ name: "Audio", extensions: AUDIO_EXTENSIONS }],
  });
  return typeof picked === "string" ? picked : null;
}

export function analyzeFile(request: AnalyzeRequest): Promise<AnalysisResult> {
  return invoke<AnalysisResult>("analyze_audio_file", { request });
}

export function cancelAnalysis(): Promise<void> {
  return invoke<void>("cancel_analysis");
}

export function onAnalysisProgress(handler: (e: ProgressEvent) => void): Promise<UnlistenFn> {
  return listen<ProgressEvent>("analysis-progress", (event) => handler(event.payload));
}

/** Files dragged onto the window. Gives real disk paths (browser drops do not). */
export function onFileDrop(handlers: {
  onHover: (over: boolean) => void;
  onDrop: (paths: string[]) => void;
}): Promise<UnlistenFn> {
  return getCurrentWebview().onDragDropEvent((event) => {
    const p = event.payload;
    if (p.type === "enter" || p.type === "over") handlers.onHover(true);
    else if (p.type === "leave") handlers.onHover(false);
    else if (p.type === "drop") {
      handlers.onHover(false);
      handlers.onDrop(p.paths);
    }
  });
}
