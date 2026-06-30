import { create } from "zustand";
import type { AnalyzeResult, AnalyzeEntry, AnalyzeLargeFile } from "@/types/analyze";

interface ScanState {
  // Current scan result for each path
  results: Map<string, AnalyzeResult>;
  
  // Loading state for each path
  loading: Map<string, boolean>;
  
  // Error state for each path
  errors: Map<string, string | null>;
  
  // Entry count for each path (for progress tracking)
  entryCounts: Map<string, number>;
  
  // Currently scanning paths (can be multiple if background scans are running)
  activeScans: Set<string>;
  
  // Navigation path stack (persisted across tab switches)
  pathStack: string[];
}

interface ScanActions {
  // Start a new scan
  startScan: (path: string | null) => void;
  
  // Update scan result incrementally
  updateResult: (
    path: string | null,
    result: Partial<AnalyzeResult>,
    entries?: AnalyzeEntry[],
    largeFiles?: AnalyzeLargeFile[]
  ) => void;
  
  // Mark scan as complete
  completeScan: (path: string | null, result: AnalyzeResult) => void;
  
  // Set loading state
  setLoading: (path: string | null, loading: boolean) => void;
  
  // Set error state
  setError: (path: string | null, error: string | null) => void;
  
  // Update entry count
  setEntryCount: (path: string | null, count: number) => void;
  
  // Add to active scans
  addActiveScan: (path: string | null) => void;
  
  // Remove from active scans
  removeActiveScan: (path: string | null) => void;
  
  // Clear all data for a path
  clearPath: (path: string | null) => void;
  
  // Get current result for a path
  getResult: (path: string | null) => AnalyzeResult | undefined;
  
  // Check if a path is loading
  isLoading: (path: string | null) => boolean;
  
  // Check if a path has an error
  getError: (path: string | null) => string | null;
  
  // Get entry count for a path
  getEntryCount: (path: string | null) => number;
  
  // Check if path is actively scanning
  isActiveScan: (path: string | null) => boolean;
  
  // Path stack navigation
  getPathStack: () => string[];
  setPathStack: (stack: string[]) => void;
  pushPath: (path: string) => void;
  popPath: () => string | undefined;
  clearPaths: () => void;
}

type ScanStore = ScanState & ScanActions;

// Helper to normalize path key
const getPathKey = (path: string | null): string => {
  return path ?? "__overview__";
};

export const useScanStore = create<ScanStore>((set, get) => ({
  results: new Map(),
  loading: new Map(),
  errors: new Map(),
  entryCounts: new Map(),
  activeScans: new Set(),
  pathStack: [],

  startScan: (path) => {
    const key = getPathKey(path);
    set((state) => ({
      loading: new Map(state.loading).set(key, true),
      errors: new Map(state.errors).set(key, null),
      activeScans: new Set(state.activeScans).add(key),
    }));
  },

  updateResult: (path, partial, entries, largeFiles) => {
    const key = getPathKey(path);
    set((state) => {
      const existing = state.results.get(key);
      const updated: AnalyzeResult = {
        ...existing,
        ...partial,
        path: partial.path ?? existing?.path ?? "/",
        overview: partial.overview ?? existing?.overview ?? false,
        entries: entries ?? existing?.entries ?? [],
        large_files: largeFiles ?? existing?.large_files,
        total_size: partial.total_size ?? existing?.total_size ?? 0,
        total_files: partial.total_files ?? existing?.total_files,
      };
      
      return {
        results: new Map(state.results).set(key, updated),
      };
    });
  },

  completeScan: (path, result) => {
    const key = getPathKey(path);
    set((state) => ({
      results: new Map(state.results).set(key, result),
      loading: new Map(state.loading).set(key, false),
      activeScans: (() => {
        const newSet = new Set(state.activeScans);
        newSet.delete(key);
        return newSet;
      })(),
    }));
  },

  setLoading: (path, loading) => {
    const key = getPathKey(path);
    set((state) => ({
      loading: new Map(state.loading).set(key, loading),
    }));
  },

  setError: (path, error) => {
    const key = getPathKey(path);
    set((state) => ({
      errors: new Map(state.errors).set(key, error),
      loading: new Map(state.loading).set(key, false),
      activeScans: (() => {
        const newSet = new Set(state.activeScans);
        newSet.delete(key);
        return newSet;
      })(),
    }));
  },

  setEntryCount: (path, count) => {
    const key = getPathKey(path);
    set((state) => ({
      entryCounts: new Map(state.entryCounts).set(key, count),
    }));
  },

  addActiveScan: (path) => {
    const key = getPathKey(path);
    set((state) => ({
      activeScans: new Set(state.activeScans).add(key),
    }));
  },

  removeActiveScan: (path) => {
    const key = getPathKey(path);
    set((state) => ({
      activeScans: (() => {
        const newSet = new Set(state.activeScans);
        newSet.delete(key);
        return newSet;
      })(),
    }));
  },

  clearPath: (path) => {
    const key = getPathKey(path);
    set((state) => ({
      results: (() => {
        const newMap = new Map(state.results);
        newMap.delete(key);
        return newMap;
      })(),
      loading: (() => {
        const newMap = new Map(state.loading);
        newMap.delete(key);
        return newMap;
      })(),
      errors: (() => {
        const newMap = new Map(state.errors);
        newMap.delete(key);
        return newMap;
      })(),
      entryCounts: (() => {
        const newMap = new Map(state.entryCounts);
        newMap.delete(key);
        return newMap;
      })(),
      activeScans: (() => {
        const newSet = new Set(state.activeScans);
        newSet.delete(key);
        return newSet;
      })(),
    }));
  },

  getResult: (path) => {
    const key = getPathKey(path);
    return get().results.get(key);
  },

  isLoading: (path) => {
    const key = getPathKey(path);
    return get().loading.get(key) ?? false;
  },

  getError: (path) => {
    const key = getPathKey(path);
    return get().errors.get(key) ?? null;
  },

  getEntryCount: (path) => {
    const key = getPathKey(path);
    return get().entryCounts.get(key) ?? 0;
  },

  isActiveScan: (path) => {
    const key = getPathKey(path);
    return get().activeScans.has(key);
  },
  
  getPathStack: () => get().pathStack,
  setPathStack: (stack) => set({ pathStack: stack }),
  pushPath: (path) => set((state) => ({ pathStack: [...state.pathStack, path] })),
  popPath: () => {
    const stack = get().pathStack;
    if (stack.length === 0) return undefined;
    const popped = stack[stack.length - 1];
    set({ pathStack: stack.slice(0, -1) });
    return popped;
  },
  clearPaths: () => set({ pathStack: [] }),
}));
