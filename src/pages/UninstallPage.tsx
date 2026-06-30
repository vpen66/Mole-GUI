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
} from "lucide-react";
import type { AppInfo } from "@/types/uninstall";

export function UninstallPage() {
  const { t } = useT();
  const [search, setSearch] = useState("");
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [done, setDone] = useState(false);
  const [selectedNames, setSelectedNames] = useState<Set<string>>(new Set());

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

  const handleExecute = async () => {
    setConfirmOpen(false);
    const targets = apps.filter((a) => selectedNames.has(a.name)).map((a) => a.name);
    try {
      await invoke("uninstall_execute", { targets });
      setDone(true);
    } catch (err) {
      console.error(err);
    }
  };

  const selectedApps = apps.filter((a) => selectedNames.has(a.name));
  const selectedSizeKb = selectedApps.reduce((sum, a) => sum + a.size_kb, 0);

  const filteredApps = appsWithSelection.filter((a) =>
    a.name.toLowerCase().includes(search.toLowerCase())
  );

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
                onClick={() => setConfirmOpen(true)}
                className="flex items-center gap-2 px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium rounded-lg transition-colors"
              >
                <Play size={14} />
                {t("uninstall.button")}
              </button>
            </div>
          )}
        </>
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
        message={t("uninstall.confirmMessage", { count: selectedApps.length })}
        totalSizeKb={selectedSizeKb}
        totalItems={selectedApps.length}
        onConfirm={handleExecute}
        onCancel={() => setConfirmOpen(false)}
      />
    </div>
  );
}
