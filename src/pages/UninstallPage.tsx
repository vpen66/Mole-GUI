import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useT } from "@/i18n";
import { ProgressBar } from "@/components/shared/ProgressBar";
import { ErrorBanner } from "@/components/shared/ErrorBanner";
import { ConfirmDialog } from "@/components/shared/ConfirmDialog";
import { formatSize } from "@/types/common";
import { useTabStore } from "@/hooks/useTabStore";
import {
  Download,
  Play,
  CheckCircle2,
  Search,
  Square,
  CheckSquare,
  AlertTriangle,
  RefreshCw,
} from "lucide-react";
import type { AppInfo, UninstallPreviewResult } from "@/types/uninstall";

export function UninstallPage() {
  const { t } = useT();
  const [search, setSearch] = useState("");
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [done, setDone] = useState(false);
  const [selectedNames, setSelectedNames] = useState<Set<string>>(new Set());
  const [previewData, setPreviewData] = useState<UninstallPreviewResult | null>(null);

  const {
    status,
    progress,
    error,
    scanned,
    apps,
  } = useTabStore((s) => s.uninstall);
  const {
    setUninstallStatus,
    setUninstallError,
    setUninstallProgress,
    setUninstallScanned,
    setUninstallApps,
  } = useTabStore();

  const scan = async () => {
    setDone(false);
    setUninstallStatus("scanning");
    setUninstallError(null);
    setUninstallProgress([]);

    try {
      const result = await invoke<AppInfo[]>("uninstall_scan_apps");
      setUninstallApps(result);
      setUninstallStatus("preview");
      setUninstallScanned(true);
    } catch (err) {
      setUninstallError(err instanceof Error ? err.message : String(err));
    }
  };

  const toggleApp = (name: string) => {
    setSelectedNames((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  };

  const selectAll = () => {
    const allSelected = apps.every((a) => selectedNames.has(a.name));
    if (allSelected) {
      setSelectedNames(new Set());
    } else {
      setSelectedNames(new Set(apps.map((a) => a.name)));
    }
  };

  const appsWithSelection = apps.map((a) => ({
    ...a,
    selected: selectedNames.has(a.name),
  }));

  const handleShowPreview = async () => {
    const targets = apps.filter((a) => selectedNames.has(a.name)).map((a) => a.name);
    if (targets.length === 0) return;

    setUninstallStatus("scanning");
    setUninstallError(null);
    setUninstallProgress([
      {
        type: "progress",
        section: "uninstall",
        message: t("uninstall.scanningLeftovers"),
      },
    ]);

    try {
      const result = await invoke<UninstallPreviewResult>("uninstall_preview", { targets });
      setPreviewData(result);
      setUninstallStatus("confirming");
    } catch (err) {
      setUninstallError(err instanceof Error ? err.message : String(err));
      setUninstallStatus("preview");
    }
  };

  const handleBackToSelection = () => {
    setPreviewData(null);
    setUninstallStatus("preview");
  };

  const handleExecute = async () => {
    if (!previewData) return;
    setConfirmOpen(false);
    setUninstallStatus("scanning");
    setUninstallProgress([
      {
        type: "progress",
        section: "uninstall",
        message: t("common.deleting"),
      },
    ]);

    try {
      await invoke("uninstall_execute", { targets: previewData.targets });
      // Remove uninstalled apps from the list
      const remaining = apps.filter((a) => !selectedNames.has(a.name));
      setUninstallApps(remaining);
      setSelectedNames(new Set());
      setPreviewData(null);
      setDone(true);
      setUninstallStatus("preview");
    } catch (err) {
      setUninstallError(err instanceof Error ? err.message : String(err));
      setUninstallStatus("confirming");
    }
  };

  const selectedApps = apps.filter((a) => selectedNames.has(a.name));
  const selectedSizeKb = selectedApps.reduce((sum, a) => sum + a.size_kb, 0);

  const filteredApps = appsWithSelection
    .filter((a) => a.name.toLowerCase().includes(search.toLowerCase()))
    .sort((a, b) => a.name.localeCompare(b.name));

  return (
    <div className="p-8 max-w-3xl space-y-6">
      <div>
        <h1 className="text-xl font-semibold flex items-center gap-2">
          <Download size={20} className="text-blue-400" />
          {t("uninstall.title")}
        </h1>
        <p className="text-sm text-surface-400 mt-1">
          {t("uninstall.subtitle")}
        </p>
      </div>

      <ErrorBanner message={error} onDismiss={() => setUninstallError(null)} />
      {status === "scanning" && <ProgressBar events={progress} />}

      {/* Initial state - no scan started yet */}
      {!scanned && status !== "scanning" && (
        <div className="py-12 flex flex-col items-center justify-center gap-4">
          <Download size={48} className="text-surface-500" />
          <div className="text-sm text-surface-400">
            {t("uninstall.subtitle")}
          </div>
          <button
            onClick={scan}
            className="flex items-center gap-2 px-4 py-2 text-sm font-medium bg-blue-600 hover:bg-blue-700 rounded-lg transition-colors"
          >
            <Play size={16} />
            {t("uninstall.startScan")}
          </button>
        </div>
      )}

      {/* Scanning indicator - shows during background scans */}
      {status === "scanning" && scanned && (
        <div className="flex items-center gap-2 text-xs text-blue-400">
          <div className="w-3 h-3 border-2 border-blue-400 border-t-transparent rounded-full animate-spin" />
          <span>{t("uninstall.scanningInBackground")}</span>
        </div>
      )}

      {status === "preview" && apps.length > 0 && (
        <>
          {/* Search and select all */}
          <div className="flex items-center gap-3">
            <div className="relative flex-1">
              <Search
                size={14}
                className="absolute left-3 top-1/2 -translate-y-1/2 text-surface-400"
              />
              <input
                type="text"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder={t("uninstall.searchPlaceholder")}
                className="w-full bg-surface-800 border border-surface-600 rounded-lg pl-9 pr-3 py-2 text-sm text-surface-100 placeholder:text-surface-500 focus:outline-none focus:border-mole-600"
              />
            </div>
            <button
              onClick={scan}
              className="flex items-center gap-1.5 text-xs text-surface-400 hover:text-surface-200 transition-colors shrink-0"
              title={t("common.refresh")}
            >
              <RefreshCw size={14} />
              {t("common.refresh")}
            </button>
            <button
              onClick={selectAll}
              className="text-xs text-surface-400 hover:text-surface-200 transition-colors shrink-0"
            >
              {selectedNames.size === apps.length ? t("uninstall.deselectAll") : t("uninstall.selectAll")}
            </button>
          </div>

          {/* App list */}
          <div className="space-y-1 max-h-96 overflow-y-auto">
            {filteredApps.map((app) => (
              <button
                key={app.name}
                onClick={() => toggleApp(app.name)}
                className={`w-full flex items-center gap-3 p-3 rounded-lg text-left transition-colors ${
                  selectedNames.has(app.name)
                    ? "bg-blue-950/30 border border-blue-800/50"
                    : "bg-surface-800 border border-surface-700 hover:border-surface-500"
                }`}
              >
                <div className="shrink-0">
                  {selectedNames.has(app.name) ? (
                    <CheckSquare size={16} className="text-blue-400" />
                  ) : (
                    <Square size={16} className="text-surface-500" />
                  )}
                </div>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium truncate">
                      {app.name}
                    </span>
                    {app.is_running && (
                      <span className="text-[10px] bg-yellow-900/40 text-yellow-400 px-1.5 py-0.5 rounded uppercase font-medium">
                        {t("uninstall.running")}
                      </span>
                    )}
                    {app.is_blocked && (
                      <AlertTriangle
                        size={12}
                        className="text-red-400 shrink-0"
                      />
                    )}
                  </div>
                  <div className="text-xs text-surface-500 truncate">
                    {app.bundle_id}
                  </div>
                </div>
                <span className="text-xs text-surface-400 shrink-0">
                  {formatSize(app.size_kb)}
                </span>
              </button>
            ))}
          </div>

          {/* Execute button */}
          {selectedApps.length > 0 && (
            <div className="flex items-center justify-between bg-surface-800 border border-surface-600 rounded-xl p-4">
              <div className="text-sm">
                <span className="text-surface-400">{t("common.selected")} </span>
                <span className="font-medium">
                  {t("uninstall.selectedApps", { count: selectedApps.length, size: formatSize(selectedSizeKb) })}
                </span>
              </div>
              <button
                onClick={handleShowPreview}
                className="flex items-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium rounded-lg transition-colors"
              >
                <Play size={14} />
                {t("uninstall.button")}
              </button>
            </div>
          )}
        </>
      )}

      {/* Scanned files preview page */}
      {status === "confirming" && previewData && (
        <div className="space-y-4">
          <div className="flex items-center justify-between border-b border-surface-700 pb-2">
            <div>
              <h2 className="text-sm font-semibold text-surface-200">
                {t("uninstall.previewTitle")}
              </h2>
              <p className="text-xs text-surface-450 mt-0.5 font-sans">
                {t("uninstall.filesToBeRemoved")}
              </p>
            </div>
            <button
              onClick={handleBackToSelection}
              className="text-xs text-blue-400 hover:text-blue-300 transition-colors"
            >
              {t("uninstall.backToSelection")}
            </button>
          </div>

          <div className="space-y-3 max-h-[360px] overflow-y-auto pr-1">
            {previewData.removal_plan.map((app) => (
              <div
                key={app.app_name}
                className="bg-surface-800 border border-surface-700 rounded-lg p-3 space-y-2"
              >
                <div className="flex justify-between items-center border-b border-surface-750 pb-1.5">
                  <span className="text-xs font-semibold text-surface-200 truncate flex-1 pr-4">
                    {app.app_name}
                  </span>
                  <span className="text-[11px] text-mole-400 font-medium shrink-0">
                    {formatSize(app.total_size_kb)}
                  </span>
                </div>

                <div className="space-y-1.5 text-[11px] leading-relaxed max-h-[140px] overflow-y-auto pr-1 font-mono">
                  {/* App Path */}
                  {app.app_path && (
                    <div className="flex items-start gap-2 text-surface-300">
                      <span className="text-green-400 shrink-0 select-none">✓</span>
                      <span className="break-all">{app.app_path}</span>
                    </div>
                  )}

                  {/* User Files */}
                  {app.user_files.map((file, idx) => (
                    <div key={idx} className="flex items-start justify-between gap-3 text-surface-450 pl-3">
                      <div className="flex items-start gap-2">
                        <span className="text-green-500 shrink-0 select-none">✓</span>
                        <span className="break-all">{file.path}</span>
                      </div>
                      {file.size_kb > 0 && (
                        <span className="text-surface-600 shrink-0 select-none">
                          {formatSize(file.size_kb)}
                        </span>
                      )}
                    </div>
                  ))}

                  {/* System Files */}
                  {app.system_files.map((file, idx) => (
                    <div key={idx} className="flex items-start justify-between gap-3 text-amber-500/90 pl-3">
                      <div className="flex items-start gap-2">
                        <span className="text-amber-500 shrink-0 select-none">⚠</span>
                        <span className="break-all">
                          [{t("uninstall.systemFiles")}] {file.path}
                        </span>
                      </div>
                      {file.size_kb > 0 && (
                        <span className="text-surface-600 shrink-0 select-none">
                          {formatSize(file.size_kb)}
                        </span>
                      )}
                    </div>
                  ))}

                  {/* Review Only Files */}
                  {app.review_only_files.map((file, idx) => (
                    <div key={idx} className="flex items-start justify-between gap-3 text-surface-500 pl-3">
                      <div className="flex items-start gap-2">
                        <span className="text-surface-500 shrink-0 select-none">☞</span>
                        <span className="break-all">
                          [{t("uninstall.reviewOnlyFiles")}] {file.path}
                        </span>
                      </div>
                      {file.size_kb > 0 && (
                        <span className="text-surface-600 shrink-0 select-none">
                          {formatSize(file.size_kb)}
                        </span>
                      )}
                    </div>
                  ))}
                </div>
              </div>
            ))}
          </div>

          {/* Sudo warning banner */}
          {previewData.requires_sudo && (
            <div className="flex items-start gap-2 text-yellow-400 bg-yellow-950/20 border border-yellow-800/40 rounded-lg p-3">
              <span className="shrink-0 text-sm">⚠</span>
              <span className="text-xs">
                {t("dialog.sudoWarning")}
              </span>
            </div>
          )}

          {/* Execution action bar */}
          <div className="flex items-center justify-between bg-surface-800 border border-surface-600 rounded-xl p-4">
            <div className="text-sm">
              <span className="text-surface-400">{t("common.selected")} </span>
              <span className="font-medium">
                {t("uninstall.selectedApps", {
                  count: previewData.removal_plan.length,
                  size: formatSize(previewData.total_size_kb),
                })}
              </span>
            </div>
            <div className="flex gap-2">
              <button
                onClick={handleBackToSelection}
                className="px-4 py-2 border border-surface-600 hover:bg-surface-700 text-surface-300 text-sm font-medium rounded-lg transition-colors"
              >
                {t("common.cancel")}
              </button>
              <button
                onClick={() => setConfirmOpen(true)}
                className="flex items-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium rounded-lg transition-colors"
              >
                <Play size={14} />
                {t("uninstall.button")}
              </button>
            </div>
          </div>
        </div>
      )}

      {done && (
        <div className="bg-mole-950/40 border border-mole-800/50 rounded-xl p-4 flex items-center gap-3">
          <CheckCircle2 size={20} className="text-mole-400 shrink-0" />
          <div className="text-sm text-mole-300">
            {t("uninstall.complete")}
          </div>
        </div>
      )}

      <ConfirmDialog
        open={confirmOpen}
        title={t("uninstall.confirmTitle")}
        message={t("uninstall.confirmMessage", { count: previewData?.removal_plan.length || selectedApps.length })}
        totalSizeKb={previewData?.total_size_kb || selectedSizeKb}
        totalItems={previewData?.removal_plan.length || selectedApps.length}
        requiresSudo={previewData?.requires_sudo}
        onConfirm={handleExecute}
        onCancel={() => setConfirmOpen(false)}
      />
    </div>
  );
}
