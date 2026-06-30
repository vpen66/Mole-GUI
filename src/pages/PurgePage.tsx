import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useT } from "@/i18n";
import { ProgressBar } from "@/components/shared/ProgressBar";
import { ErrorBanner } from "@/components/shared/ErrorBanner";
import { ConfirmDialog } from "@/components/shared/ConfirmDialog";
import { formatSize } from "@/types/common";
import { useTabStore } from "@/hooks/useTabStore";
import {
  FolderOpen,
  Play,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Package,
} from "lucide-react";
import type { PurgeResult } from "@/types/purge";

export function PurgePage() {
  const { t } = useT();
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [done, setDone] = useState(false);
  const [expandedProjects, setExpandedProjects] = useState<Set<string>>(
    new Set()
  );
  const [selectedNames, setSelectedNames] = useState<Set<string>>(new Set());

  const {
    status,
    progress,
    error,
    scanned,
    projects,
  } = useTabStore((s) => s.purge);
  const {
    setPurgeStatus,
    setPurgeError,
    setPurgeProgress,
    setPurgeScanned,
    setPurgeProjects,
  } = useTabStore();

  const scan = async () => {
    setPurgeStatus("scanning");
    setPurgeError(null);
    setPurgeProgress([]);

    try {
      const result = await invoke<PurgeResult>("purge_dry_run");
      setPurgeProjects(result.projects);
      setPurgeStatus("preview");
      setPurgeScanned(true);
    } catch (err) {
      setPurgeError(err instanceof Error ? err.message : String(err));
    }
  };

  const toggleProject = (name: string) => {
    setSelectedNames((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  };

  const toggleExpand = (name: string) => {
    setExpandedProjects((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  };

  const handleExecute = async () => {
    setConfirmOpen(false);
    const targets = projects.filter((p) => selectedNames.has(p.name)).map((p) => p.name);
    try {
      await invoke("purge_execute", { targets });
      setDone(true);
    } catch (err) {
      console.error(err);
    }
  };

  const selectedProjects = projects.filter((p) => selectedNames.has(p.name));
  const selectedSizeKb = selectedProjects.reduce(
    (sum, p) => sum + p.total_size_kb,
    0
  );

  const projectsWithSelection = projects.map((p) => ({
    ...p,
    selected: selectedNames.has(p.name),
  }));

  return (
    <div className="p-8 max-w-3xl space-y-6">
      <div>
        <h1 className="text-xl font-semibold flex items-center gap-2">
          <FolderOpen size={20} className="text-amber-400" />
          {t("purge.title")}
        </h1>
        <p className="text-sm text-surface-400 mt-1">
          {t("purge.subtitle")}
        </p>
      </div>

      <ErrorBanner message={error} onDismiss={() => setPurgeError(null)} />
      {status === "scanning" && <ProgressBar events={progress} />}

      {/* Initial state - no scan started yet */}
      {!scanned && status !== "scanning" && (
        <div className="py-12 flex flex-col items-center justify-center gap-4">
          <FolderOpen size={48} className="text-surface-500" />
          <div className="text-sm text-surface-400">
            {t("purge.subtitle")}
          </div>
          <button
            onClick={scan}
            className="flex items-center gap-2 px-4 py-2 text-sm font-medium bg-amber-600 hover:bg-amber-700 rounded-lg transition-colors"
          >
            <Play size={16} />
            {t("purge.startScan")}
          </button>
        </div>
      )}

      {/* Scanning indicator - shows during background scans */}
      {status === "scanning" && scanned && (
        <div className="flex items-center gap-2 text-xs text-amber-400">
          <div className="w-3 h-3 border-2 border-amber-400 border-t-transparent rounded-full animate-spin" />
          <span>{t("purge.scanningInBackground")}</span>
        </div>
      )}

      {status === "preview" && projects.length > 0 && (
        <>
          <div className="flex items-center justify-between text-xs text-surface-400 pb-1 border-b border-surface-700">
            <span>
              {t("purge.projectsFound", { count: projects.length })}
            </span>
            <span className="text-amber-400 font-medium">
              {formatSize(projects.reduce((s, p) => s + p.total_size_kb, 0))}{" "}
              {t("purge.total")}
            </span>
          </div>

          <div className="space-y-2 max-h-96 overflow-y-auto">
            {projectsWithSelection
              .sort((a, b) => b.total_size_kb - a.total_size_kb)
              .map((project) => (
                <div
                  key={project.name}
                  className={`rounded-lg border overflow-hidden transition-colors ${
                    selectedNames.has(project.name)
                      ? "bg-amber-950/20 border-amber-800/50"
                      : "bg-surface-800 border-surface-700"
                  }`}
                >
                  <div className="flex items-center gap-3 p-3">
                    <input
                      type="checkbox"
                      checked={selectedNames.has(project.name)}
                      onChange={() => toggleProject(project.name)}
                      className="w-4 h-4 accent-amber-500"
                    />
                    <button
                      onClick={() => toggleExpand(project.name)}
                      className="flex-1 flex items-center gap-2 text-left"
                    >
                      {expandedProjects.has(project.name) ? (
                        <ChevronDown
                          size={14}
                          className="text-surface-400 shrink-0"
                        />
                      ) : (
                        <ChevronRight
                          size={14}
                          className="text-surface-400 shrink-0"
                        />
                      )}
                      <div className="flex-1 min-w-0">
                        <div className="text-sm font-medium truncate">
                          {project.name}
                        </div>
                        <div className="text-xs text-surface-500 truncate">
                          {project.path}
                        </div>
                      </div>
                      <span className="text-xs text-amber-400 font-medium shrink-0">
                        {formatSize(project.total_size_kb)}
                      </span>
                    </button>
                  </div>

                  {expandedProjects.has(project.name) &&
                    project.artifacts.length > 0 && (
                      <div className="border-t border-surface-700 divide-y divide-surface-700">
                        {project.artifacts.map((artifact, idx) => (
                          <div
                            key={idx}
                            className="flex items-center justify-between px-10 py-2 text-xs"
                          >
                            <div className="flex items-center gap-2 text-surface-300">
                              <Package size={12} className="text-surface-500" />
                              <span>{artifact.name}</span>
                            </div>
                            <span className="text-surface-500">
                              {artifact.size_human}
                            </span>
                          </div>
                        ))}
                      </div>
                    )}
                </div>
              ))}
          </div>

          {selectedProjects.length > 0 && (
            <div className="flex items-center justify-between bg-surface-800 border border-surface-600 rounded-xl p-4">
              <div className="text-sm">
                <span className="text-surface-400">{t("common.selected")} </span>
                <span className="font-medium">
                  {t("purge.selectedProjects", { count: selectedProjects.length, size: formatSize(selectedSizeKb) })}
                </span>
              </div>
              <button
                onClick={() => setConfirmOpen(true)}
                className="flex items-center gap-2 px-4 py-2 bg-amber-600 hover:bg-amber-700 text-white text-sm font-medium rounded-lg transition-colors"
              >
                <Play size={14} />
                {t("purge.button")}
              </button>
            </div>
          )}
        </>
      )}

      {status === "preview" && projects.length === 0 && (
        <div className="text-sm text-surface-400 bg-surface-800 border border-surface-700 rounded-xl p-6 text-center">
          {t("purge.empty")}
        </div>
      )}

      {done && (
        <div className="bg-mole-950/40 border border-mole-800/50 rounded-xl p-4 flex items-center gap-3">
          <CheckCircle2 size={20} className="text-mole-400 shrink-0" />
          <div className="text-sm text-mole-300">
            {t("purge.complete")}
          </div>
        </div>
      )}

      <ConfirmDialog
        open={confirmOpen}
        title={t("purge.confirmTitle")}
        message={t("purge.confirmMessage", { count: selectedProjects.length })}
        totalSizeKb={selectedSizeKb}
        totalItems={selectedProjects.length}
        onConfirm={handleExecute}
        onCancel={() => setConfirmOpen(false)}
      />
    </div>
  );
}
