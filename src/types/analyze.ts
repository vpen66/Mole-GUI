// Types for analyze --json output from Mole CLI

export interface AnalyzeEntry {
  name: string;
  path: string;
  size: number;
  is_dir: boolean;
  insight?: boolean;
  cleanable?: boolean;
  last_access?: string;
}

export interface AnalyzeLargeFile {
  name: string;
  path: string;
  size: number;
}

export interface AnalyzeResult {
  path: string;
  overview: boolean;
  entries: AnalyzeEntry[];
  large_files?: AnalyzeLargeFile[];
  total_size: number;
  total_files?: number;
}

// NDJSON streaming event types from analyze --json
export type AnalyzeStreamEvent =
  | AnalyzeProgressEvent
  | AnalyzeEntryEvent
  | AnalyzeLargeFileEvent
  | AnalyzeSummaryEvent;

export interface AnalyzeProgressEvent {
  type: "progress";
  message: string;
}

export interface AnalyzeEntryEvent {
  type: "entry";
  name: string;
  path: string;
  size: number;
  is_dir: boolean;
  insight?: boolean;
  cleanable?: boolean;
  last_access?: string;
}

export interface AnalyzeLargeFileEvent {
  type: "large_file";
  name: string;
  path: string;
  size: number;
}

export interface AnalyzeSummaryEvent {
  type: "summary";
  path: string;
  overview: boolean;
  total_size: number;
  total_files?: number;
}

export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes}B`;
  if (bytes < 1024 * 1024) {
    const kb = bytes / 1024;
    return kb < 10 ? `${kb.toFixed(1)}KB` : `${Math.round(kb)}KB`;
  }
  if (bytes < 1024 * 1024 * 1024) {
    const mb = bytes / (1024 * 1024);
    return mb < 10 ? `${mb.toFixed(1)}MB` : `${Math.round(mb)}MB`;
  }
  const gb = bytes / (1024 * 1024 * 1024);
  return gb < 10 ? `${gb.toFixed(1)}GB` : `${Math.round(gb)}GB`;
}
