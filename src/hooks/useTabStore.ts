import { create } from "zustand";
import type { CommandStatus, ProgressEvent, ItemEvent, SummaryEvent } from "@/types/common";
import type { AppInfo } from "@/types/uninstall";
import type { PurgeProject } from "@/types/purge";
import type { OptimizeItem } from "@/types/optimize";

// GroupedSection mirrors the local type in CleanPage
export interface GroupedSection {
  name: string;
  items: ItemEvent[];
  totalKb: number;
}

// ── Per-tab state shape ──

interface CleanTab {
  status: CommandStatus;
  error: string | null;
  progress: ProgressEvent[];
  scanned: boolean;
  sections: GroupedSection[];
  summary: SummaryEvent | null;
}

interface UninstallTab {
  status: CommandStatus;
  error: string | null;
  progress: ProgressEvent[];
  scanned: boolean;
  apps: AppInfo[];
}

interface PurgeTab {
  status: CommandStatus;
  error: string | null;
  progress: ProgressEvent[];
  scanned: boolean;
  projects: PurgeProject[];
}

interface OptimizeTab {
  status: CommandStatus;
  error: string | null;
  progress: ProgressEvent[];
  scanned: boolean;
  items: OptimizeItem[];
}

// ── Store interface ──

interface TabStore {
  clean: CleanTab;
  uninstall: UninstallTab;
  purge: PurgeTab;
  optimize: OptimizeTab;

  // Clean actions
  setCleanStatus: (status: CommandStatus) => void;
  setCleanError: (error: string | null) => void;
  setCleanProgress: (progress: ProgressEvent[]) => void;
  setCleanScanned: (scanned: boolean) => void;
  setCleanSections: (sections: GroupedSection[]) => void;
  setCleanSummary: (summary: SummaryEvent | null) => void;
  resetClean: () => void;

  // Uninstall actions
  setUninstallStatus: (status: CommandStatus) => void;
  setUninstallError: (error: string | null) => void;
  setUninstallProgress: (progress: ProgressEvent[]) => void;
  setUninstallScanned: (scanned: boolean) => void;
  setUninstallApps: (apps: AppInfo[]) => void;
  resetUninstall: () => void;

  // Purge actions
  setPurgeStatus: (status: CommandStatus) => void;
  setPurgeError: (error: string | null) => void;
  setPurgeProgress: (progress: ProgressEvent[]) => void;
  setPurgeScanned: (scanned: boolean) => void;
  setPurgeProjects: (projects: PurgeProject[]) => void;
  resetPurge: () => void;

  // Optimize actions
  setOptimizeStatus: (status: CommandStatus) => void;
  setOptimizeError: (error: string | null) => void;
  setOptimizeProgress: (progress: ProgressEvent[]) => void;
  setOptimizeScanned: (scanned: boolean) => void;
  setOptimizeItems: (items: OptimizeItem[]) => void;
  resetOptimize: () => void;
}

// ── Defaults ──

const defaultClean: CleanTab = {
  status: "idle",
  error: null,
  progress: [],
  scanned: false,
  sections: [],
  summary: null,
};

const defaultUninstall: UninstallTab = {
  status: "idle",
  error: null,
  progress: [],
  scanned: false,
  apps: [],
};

const defaultPurge: PurgeTab = {
  status: "idle",
  error: null,
  progress: [],
  scanned: false,
  projects: [],
};

const defaultOptimize: OptimizeTab = {
  status: "idle",
  error: null,
  progress: [],
  scanned: false,
  items: [],
};

// ── Store ──

export const useTabStore = create<TabStore>((set) => ({
  clean: { ...defaultClean },
  uninstall: { ...defaultUninstall },
  purge: { ...defaultPurge },
  optimize: { ...defaultOptimize },

  // Clean
  setCleanStatus: (status) => set((s) => ({ clean: { ...s.clean, status } })),
  setCleanError: (error) => set((s) => ({ clean: { ...s.clean, error, status: error ? "error" : s.clean.status } })),
  setCleanProgress: (progress) => set((s) => ({ clean: { ...s.clean, progress } })),
  setCleanScanned: (scanned) => set((s) => ({ clean: { ...s.clean, scanned } })),
  setCleanSections: (sections) => set((s) => ({ clean: { ...s.clean, sections } })),
  setCleanSummary: (summary) => set((s) => ({ clean: { ...s.clean, summary } })),
  resetClean: () => set({ clean: { ...defaultClean } }),

  // Uninstall
  setUninstallStatus: (status) => set((s) => ({ uninstall: { ...s.uninstall, status } })),
  setUninstallError: (error) => set((s) => ({ uninstall: { ...s.uninstall, error, status: error ? "error" : s.uninstall.status } })),
  setUninstallProgress: (progress) => set((s) => ({ uninstall: { ...s.uninstall, progress } })),
  setUninstallScanned: (scanned) => set((s) => ({ uninstall: { ...s.uninstall, scanned } })),
  setUninstallApps: (apps) => set((s) => ({ uninstall: { ...s.uninstall, apps } })),
  resetUninstall: () => set({ uninstall: { ...defaultUninstall } }),

  // Purge
  setPurgeStatus: (status) => set((s) => ({ purge: { ...s.purge, status } })),
  setPurgeError: (error) => set((s) => ({ purge: { ...s.purge, error, status: error ? "error" : s.purge.status } })),
  setPurgeProgress: (progress) => set((s) => ({ purge: { ...s.purge, progress } })),
  setPurgeScanned: (scanned) => set((s) => ({ purge: { ...s.purge, scanned } })),
  setPurgeProjects: (projects) => set((s) => ({ purge: { ...s.purge, projects } })),
  resetPurge: () => set({ purge: { ...defaultPurge } }),

  // Optimize
  setOptimizeStatus: (status) => set((s) => ({ optimize: { ...s.optimize, status } })),
  setOptimizeError: (error) => set((s) => ({ optimize: { ...s.optimize, error, status: error ? "error" : s.optimize.status } })),
  setOptimizeProgress: (progress) => set((s) => ({ optimize: { ...s.optimize, progress } })),
  setOptimizeScanned: (scanned) => set((s) => ({ optimize: { ...s.optimize, scanned } })),
  setOptimizeItems: (items) => set((s) => ({ optimize: { ...s.optimize, items } })),
  resetOptimize: () => set({ optimize: { ...defaultOptimize } }),
}));
