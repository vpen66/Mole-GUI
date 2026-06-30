import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useT } from "@/i18n";
import {
  Trash2,
  Download,
  FolderOpen,
  Zap,
  HardDrive,
  Clock,
  Cpu,
  MemoryStick,
  Activity,
  Gauge,
} from "lucide-react";
import { formatBytes } from "@/types/common";
import type { MoleVersion, SystemStatus } from "@/types/common";
import { useNavigate } from "react-router-dom";

const REFRESH_OPTIONS = [
  { value: 0, label: "Off" },
  { value: 1, label: "1s" },
  { value: 2, label: "2s" },
  { value: 5, label: "5s" },
  { value: 10, label: "10s" },
];

const REFRESH_INTERVAL_KEY = "mole-dashboard-refresh-interval";

function loadRefreshInterval(): number {
  const stored = localStorage.getItem(REFRESH_INTERVAL_KEY);
  if (stored !== null) {
    const val = parseInt(stored, 10);
    if (!isNaN(val) && REFRESH_OPTIONS.some((o) => o.value === val)) {
      return val;
    }
  }
  return 1;
}

const quickActions = [
  {
    icon: Trash2,
    labelKey: "dashboard.action.clean",
    descKey: "dashboard.action.cleanDesc",
    to: "/clean",
    color: "text-green-400",
    bg: "bg-green-950/30",
  },
  {
    icon: Download,
    labelKey: "dashboard.action.uninstall",
    descKey: "dashboard.action.uninstallDesc",
    to: "/uninstall",
    color: "text-blue-400",
    bg: "bg-blue-950/30",
  },
  {
    icon: FolderOpen,
    labelKey: "dashboard.action.purge",
    descKey: "dashboard.action.purgeDesc",
    to: "/purge",
    color: "text-amber-400",
    bg: "bg-amber-950/30",
  },
  {
    icon: Zap,
    labelKey: "dashboard.action.optimize",
    descKey: "dashboard.action.optimizeDesc",
    to: "/optimize",
    color: "text-purple-400",
    bg: "bg-purple-950/30",
  },
  {
    icon: HardDrive,
    labelKey: "dashboard.action.analyze",
    descKey: "dashboard.action.analyzeDesc",
    to: "/analyze",
    color: "text-cyan-400",
    bg: "bg-cyan-950/30",
  },
];

function getHealthColor(score: number): string {
  if (score >= 90) return "text-green-400";
  if (score >= 70) return "text-yellow-400";
  if (score >= 50) return "text-orange-400";
  return "text-red-400";
}

function getUsageColor(percent: number): string {
  if (percent < 60) return "bg-cyan-500";
  if (percent < 80) return "bg-yellow-500";
  if (percent < 90) return "bg-orange-500";
  return "bg-red-500";
}

export function DashboardPage() {
  const { t } = useT();
  const navigate = useNavigate();
  const [version, setVersion] = useState<MoleVersion | null>(null);
  const [status, setStatus] = useState<SystemStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshInterval, setRefreshIntervalRaw] = useState(loadRefreshInterval);
  const setRefreshInterval = (val: number) => {
    setRefreshIntervalRaw(val);
    localStorage.setItem(REFRESH_INTERVAL_KEY, String(val));
  };
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  const fetchStatus = useCallback(async () => {
    try {
      const data = await invoke<SystemStatus>("get_system_status");
      setStatus(data);
    } catch (err) {
      console.error("Failed to get system status:", err);
    }
  }, []);

  // Periodic refresh: start/stop based on refreshInterval
  useEffect(() => {
    if (timerRef.current) {
      clearInterval(timerRef.current);
      timerRef.current = null;
    }
    if (refreshInterval > 0) {
      timerRef.current = setInterval(fetchStatus, refreshInterval * 1000);
    }
    return () => {
      if (timerRef.current) {
        clearInterval(timerRef.current);
        timerRef.current = null;
      }
    };
  }, [refreshInterval, fetchStatus]);

  // Initial data load
  useEffect(() => {
    let mounted = true;

    async function fetchData() {
      try {
        const versionData = await invoke<MoleVersion>("get_mole_version");
        if (mounted) {
          setVersion(versionData);
        }
      } catch (err) {
        console.error("Failed to get Mole version:", err);
      }

      await fetchStatus();

      if (mounted) {
        setLoading(false);
      }
    }

    fetchData();

    // Re-fetch when page becomes visible
    const handleVisibilityChange = () => {
      if (!document.hidden) {
        fetchStatus();
      }
    };
    document.addEventListener("visibilitychange", handleVisibilityChange);

    return () => {
      mounted = false;
      document.removeEventListener("visibilitychange", handleVisibilityChange);
    };
  }, [fetchStatus]);

  const diskFreeKb = status ? status.disk_free / 1024 : null;

  return (
    <div className="p-8 space-y-6 max-w-4xl">
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-xl font-semibold">{t("dashboard.title")}</h1>
          <p className="text-sm text-surface-400 mt-1">
            {t("dashboard.subtitle")}
            {status && (
              <span className="ml-2 text-surface-500">
                {status.model} · {status.cpu_model} · {status.total_ram}
              </span>
            )}
          </p>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <span className="text-xs text-surface-500">{t("dashboard.refreshInterval")}</span>
          <div className="flex bg-surface-800 border border-surface-700 rounded-lg overflow-hidden">
            {REFRESH_OPTIONS.map((opt) => (
              <button
                key={opt.value}
                onClick={() => setRefreshInterval(opt.value)}
                className={`px-2 py-1 text-xs transition-colors ${
                  refreshInterval === opt.value
                    ? "bg-cyan-600 text-white"
                    : "text-surface-400 hover:text-surface-200 hover:bg-surface-700"
                }`}
              >
                {opt.value === 0 ? t("dashboard.refreshOff") : opt.label}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* System metrics grid */}
      <div className="grid grid-cols-2 gap-3">
        {/* Health Score */}
        <div className="bg-surface-800 border border-surface-700 rounded-xl p-4 flex items-center gap-3">
          <div className="w-10 h-10 bg-surface-700 rounded-lg flex items-center justify-center">
            <Gauge size={18} className={status ? getHealthColor(status.health_score) : "text-surface-300"} />
          </div>
          <div className="flex-1 min-w-0">
            <div className="text-xs text-surface-400">{t("dashboard.healthScore")}</div>
            <div className="flex items-center gap-2">
              <span className={`text-sm font-semibold ${status ? getHealthColor(status.health_score) : ""}`}>
                {status ? `${status.health_score}` : "--"}
              </span>
              {status && (
                <span className="text-xs text-surface-500">{status.health_score_msg}</span>
              )}
            </div>
          </div>
        </div>

        {/* Uptime */}
        <div className="bg-surface-800 border border-surface-700 rounded-xl p-4 flex items-center gap-3">
          <div className="w-10 h-10 bg-surface-700 rounded-lg flex items-center justify-center">
            <Clock size={18} className="text-surface-300" />
          </div>
          <div>
            <div className="text-xs text-surface-400">{t("dashboard.uptime")}</div>
            <div className="text-sm font-semibold">
              {status ? status.uptime : "--"}
            </div>
          </div>
        </div>

        {/* CPU Usage */}
        <div className="bg-surface-800 border border-surface-700 rounded-xl p-4 flex items-center gap-3">
          <div className="w-10 h-10 bg-surface-700 rounded-lg flex items-center justify-center">
            <Cpu size={18} className="text-blue-400" />
          </div>
          <div className="flex-1 min-w-0">
            <div className="text-xs text-surface-400">{t("dashboard.cpuUsage")}</div>
            <div className="flex items-center gap-2">
              <span className="text-sm font-semibold">
                {status ? `${Math.round(status.cpu_usage)}%` : "--"}
              </span>
              {status && (
                <span className="text-[10px] text-surface-500">
                  {status.cpu_core_count} {t("dashboard.cores")}
                </span>
              )}
            </div>
            {status && (
              <div className="mt-1 h-1 bg-surface-700 rounded-full overflow-hidden">
                <div
                  className={`h-full rounded-full transition-all ${getUsageColor(status.cpu_usage)}`}
                  style={{ width: `${Math.min(100, status.cpu_usage)}%` }}
                />
              </div>
            )}
          </div>
        </div>

        {/* Memory Usage */}
        <div className="bg-surface-800 border border-surface-700 rounded-xl p-4 flex items-center gap-3">
          <div className="w-10 h-10 bg-surface-700 rounded-lg flex items-center justify-center">
            <MemoryStick size={18} className="text-purple-400" />
          </div>
          <div className="flex-1 min-w-0">
            <div className="text-xs text-surface-400">{t("dashboard.memoryUsage")}</div>
            <div className="flex items-center gap-2">
              <span className="text-sm font-semibold">
                {status ? `${Math.round(status.memory_used_percent)}%` : "--"}
              </span>
              {status && (
                <span className="text-[10px] text-surface-500">
                  {formatBytes(status.memory_used)} / {formatBytes(status.memory_total)}
                </span>
              )}
            </div>
            {status && (
              <div className="mt-1 h-1 bg-surface-700 rounded-full overflow-hidden">
                <div
                  className={`h-full rounded-full transition-all ${getUsageColor(status.memory_used_percent)}`}
                  style={{ width: `${Math.min(100, status.memory_used_percent)}%` }}
                />
              </div>
            )}
          </div>
        </div>

        {/* Disk Free Space */}
        <div className="bg-surface-800 border border-surface-700 rounded-xl p-4 flex items-center gap-3">
          <div className="w-10 h-10 bg-surface-700 rounded-lg flex items-center justify-center">
            <HardDrive size={18} className="text-cyan-400" />
          </div>
          <div className="flex-1 min-w-0">
            <div className="text-xs text-surface-400">{t("dashboard.freeDiskSpace")}</div>
            <div className="flex items-center gap-2">
              <span className="text-sm font-semibold">
                {diskFreeKb !== null ? formatBytes(status!.disk_free) : "--"}
              </span>
              {status && (
                <span className="text-[10px] text-surface-500">
                  {formatBytes(status.disk_used)} / {formatBytes(status.disk_total)}
                </span>
              )}
            </div>
            {status && (
              <div className="mt-1 h-1 bg-surface-700 rounded-full overflow-hidden">
                <div
                  className={`h-full rounded-full transition-all ${getUsageColor(status.disk_used_percent)}`}
                  style={{ width: `${Math.min(100, status.disk_used_percent)}%` }}
                />
              </div>
            )}
          </div>
        </div>

        {/* Mole CLI Version */}
        <div className="bg-surface-800 border border-surface-700 rounded-xl p-4 flex items-center gap-3">
          <div className="w-10 h-10 bg-surface-700 rounded-lg flex items-center justify-center">
            <Activity size={18} className="text-green-400" />
          </div>
          <div>
            <div className="text-xs text-surface-400">{t("dashboard.moleCli")}</div>
            <div className="text-sm font-semibold">
              {loading ? (
                <span className="text-surface-500">{t("common.loading")}</span>
              ) : version?.installed ? (
                `v${version.version}`
              ) : (
                t("dashboard.notInstalled")
              )}
            </div>
          </div>
        </div>
      </div>

      {/* Quick actions */}
      <div>
        <h2 className="text-sm font-medium text-surface-300 mb-3">
          {t("dashboard.quickActions")}
        </h2>
        <div className="grid grid-cols-2 gap-3">
          {quickActions.map(({ icon: Icon, labelKey, descKey, to, color, bg }) => (
            <button
              key={to}
              onClick={() => navigate(to)}
              className="bg-surface-800 border border-surface-700 rounded-xl p-4 flex items-start gap-3 text-left hover:border-surface-500 hover:bg-surface-750 transition-all group"
            >
              <div
                className={`w-9 h-9 ${bg} rounded-lg flex items-center justify-center shrink-0`}
              >
                <Icon size={16} className={color} />
              </div>
              <div>
                <div className="text-sm font-medium group-hover:text-white transition-colors">
                  {t(labelKey)}
                </div>
                <div className="text-xs text-surface-400 mt-0.5">{t(descKey)}</div>
              </div>
            </button>
          ))}
        </div>
      </div>

      {!loading && !version?.installed && (
        <div className="bg-yellow-950/30 border border-yellow-800/40 rounded-xl p-4 text-sm text-yellow-300">
          <strong>{t("dashboard.cliNotFound")}</strong> {t("dashboard.cliNotFoundHint")}
          <code className="block mt-2 text-xs bg-surface-900 p-2 rounded font-mono">
            /bin/bash -c "$(curl -fsSL
            https://raw.githubusercontent.com/tw93/Mole/main/install.sh)"
          </code>
        </div>
      )}
    </div>
  );
}
