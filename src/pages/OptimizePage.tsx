import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useT } from "@/i18n";
import { ProgressBar } from "@/components/shared/ProgressBar";
import { ErrorBanner } from "@/components/shared/ErrorBanner";
import { ConfirmDialog } from "@/components/shared/ConfirmDialog";
import { useTabStore } from "@/hooks/useTabStore";
import {
  Zap,
  Play,
  CheckCircle2,
  Shield,
  Cpu,
  HardDrive,
  MemoryStick,
  Clock,
} from "lucide-react";
import type { OptimizeResult, SystemHealth } from "@/types/optimize";

function HealthCard({
  icon: Icon,
  label,
  value,
  sub,
}: {
  icon: React.ElementType;
  label: string;
  value: string;
  sub?: string;
}) {
  return (
    <div className="bg-surface-800 border border-surface-700 rounded-xl p-3 flex items-center gap-3">
      <div className="w-8 h-8 bg-surface-700 rounded-lg flex items-center justify-center">
        <Icon size={14} className="text-surface-300" />
      </div>
      <div>
        <div className="text-[10px] text-surface-500 uppercase tracking-wide">
          {label}
        </div>
        <div className="text-sm font-semibold">{value}</div>
        {sub && <div className="text-[10px] text-surface-400">{sub}</div>}
      </div>
    </div>
  );
}

export function OptimizePage() {
  const { t } = useT();
  const [health] = useState<SystemHealth | null>(null);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [done, setDone] = useState(false);
  const [enabledActions, setEnabledActions] = useState<Set<string>>(new Set());

  const {
    status,
    progress,
    error,
    scanned,
    items,
  } = useTabStore((s) => s.optimize);
  const {
    setOptimizeStatus,
    setOptimizeError,
    setOptimizeProgress,
    setOptimizeScanned,
    setOptimizeItems,
  } = useTabStore();

  const scan = async () => {
    setOptimizeStatus("scanning");
    setOptimizeError(null);
    setOptimizeProgress([]);

    try {
      const result = await invoke<OptimizeResult>("optimize_dry_run");
      setOptimizeItems(result.optimizations);
      setOptimizeStatus("preview");
      setOptimizeScanned(true);
    } catch (err) {
      setOptimizeError(err instanceof Error ? err.message : String(err));
    }
  };

  const toggleItem = (action: string) => {
    setEnabledActions((prev) => {
      const next = new Set(prev);
      if (next.has(action)) next.delete(action);
      else next.add(action);
      return next;
    });
  };

  const itemsWithEnabled = items.map((i) => ({
    ...i,
    enabled: enabledActions.has(i.action),
  }));

  const handleExecute = async () => {
    setConfirmOpen(false);
    try {
      const enabled = items.filter((i) => enabledActions.has(i.action)).map((i) => i.action);
      await invoke("optimize_execute", { actions: enabled });
      setDone(true);
    } catch (err) {
      console.error(err);
    }
  };

  const enabledItems = items.filter((i) => enabledActions.has(i.action));
  const requiresSudo = enabledItems.some((i) => i.requires_sudo);

  return (
    <div className="p-8 max-w-3xl space-y-6">
      <div>
        <h1 className="text-xl font-semibold flex items-center gap-2">
          <Zap size={20} className="text-purple-400" />
          {t("optimize.title")}
        </h1>
        <p className="text-sm text-surface-400 mt-1">
          {t("optimize.subtitle")}
        </p>
      </div>

      <ErrorBanner message={error} onDismiss={() => setOptimizeError(null)} />
      {status === "scanning" && <ProgressBar events={progress} />}

      {/* Initial state - no scan started yet */}
      {!scanned && status !== "scanning" && (
        <div className="py-12 flex flex-col items-center justify-center gap-4">
          <Zap size={48} className="text-surface-500" />
          <div className="text-sm text-surface-400">
            {t("optimize.subtitle")}
          </div>
          <button
            onClick={scan}
            className="flex items-center gap-2 px-4 py-2 text-sm font-medium bg-purple-600 hover:bg-purple-700 rounded-lg transition-colors"
          >
            <Play size={16} />
            {t("optimize.startScan")}
          </button>
        </div>
      )}

      {/* Scanning indicator - shows during background scans */}
      {status === "scanning" && scanned && (
        <div className="flex items-center gap-2 text-xs text-purple-400">
          <div className="w-3 h-3 border-2 border-purple-400 border-t-transparent rounded-full animate-spin" />
          <span>{t("optimize.scanningInBackground")}</span>
        </div>
      )}

      {/* System Health */}
      {health && (
        <div className="grid grid-cols-2 gap-3">
          <HealthCard
            icon={MemoryStick}
            label={t("optimize.memory")}
            value={`${health.memory_used_gb.toFixed(1)}GB`}
            sub={`of ${health.memory_total_gb}GB`}
          />
          <HealthCard
            icon={HardDrive}
            label={t("optimize.disk")}
            value={t("optimize.diskUsed", { used: String(health.disk_used_gb) })}
            sub={`of ${health.disk_total_gb}GB`}
          />
          <HealthCard
            icon={Clock}
            label={t("optimize.uptime")}
            value={t("optimize.uptimeDays", { days: String(health.uptime_days) })}
          />
          <HealthCard icon={Cpu} label={t("optimize.status")} value={t("optimize.healthy")} />
        </div>
      )}

      {/* Optimization items */}
      {items.length > 0 && (
        <div className="space-y-2">
          <h2 className="text-sm font-medium text-surface-300">
            {t("optimize.available")}
          </h2>
          {itemsWithEnabled.map((item) => (
            <div
              key={item.action}
              className={`flex items-center gap-3 p-3 rounded-lg border transition-colors cursor-pointer ${
                item.enabled
                  ? "bg-purple-950/20 border-purple-800/50"
                  : "bg-surface-800 border-surface-700 hover:border-surface-500"
              }`}
              onClick={() => toggleItem(item.action)}
            >
              <input
                type="checkbox"
                checked={item.enabled}
                onChange={() => toggleItem(item.action)}
                className="w-4 h-4 accent-purple-500"
              />
              <div className="flex-1">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-medium">{item.name}</span>
                  {!item.safe && (
                    <Shield size={12} className="text-yellow-400" />
                  )}
                  {item.requires_sudo && (
                    <span className="text-[10px] bg-surface-700 text-surface-300 px-1.5 py-0.5 rounded uppercase font-medium">
                      sudo
                    </span>
                  )}
                </div>
                <div className="text-xs text-surface-400 mt-0.5">
                  {item.description}
                </div>
              </div>
              {item.status && item.status !== "pending" && (
                <span
                  className={`text-xs ${
                    item.status === "applied"
                      ? "text-mole-400"
                      : item.status === "failed"
                        ? "text-red-400"
                        : "text-surface-400"
                  }`}
                >
                  {item.status}
                </span>
              )}
            </div>
          ))}
        </div>
      )}

      {enabledItems.length > 0 && !done && (
        <div className="flex items-center justify-between bg-surface-800 border border-surface-600 rounded-xl p-4">
          <div className="text-sm">
            <span className="text-surface-400">{t("common.selected")} </span>
            <span className="font-medium">
              {t("optimize.selectedCount", { count: enabledItems.length })}
            </span>
          </div>
          <button
            onClick={() => setConfirmOpen(true)}
            className="flex items-center gap-2 px-4 py-2 bg-purple-600 hover:bg-purple-700 text-white text-sm font-medium rounded-lg transition-colors"
          >
            <Play size={14} />
            {t("common.apply")}
          </button>
        </div>
      )}

      {done && (
        <div className="bg-mole-950/40 border border-mole-800/50 rounded-xl p-4 flex items-center gap-3">
          <CheckCircle2 size={20} className="text-mole-400 shrink-0" />
          <div className="text-sm text-mole-300">
            {t("optimize.complete")}
          </div>
        </div>
      )}

      <ConfirmDialog
        open={confirmOpen}
        title={t("optimize.confirmTitle")}
        message={t("optimize.confirmMessage", { count: enabledItems.length })}
        totalItems={enabledItems.length}
        requiresSudo={requiresSudo}
        onConfirm={handleExecute}
        onCancel={() => setConfirmOpen(false)}
      />
    </div>
  );
}
