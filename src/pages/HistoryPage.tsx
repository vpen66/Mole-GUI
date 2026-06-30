import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useT } from "@/i18n";
import { History, Trash2, Download, FolderOpen, Zap } from "lucide-react";

interface HistorySession {
  command: string;
  started_at: string;
  ended_at: string;
  items: number;
  size: string;
  operation_count: number;
  actions: {
    removed: number;
    trashed: number;
    skipped: number;
    failed: number;
  };
}

interface HistoryData {
  sessions: HistorySession[];
  total_sessions: number;
}

const commandIcons: Record<string, React.ElementType> = {
  clean: Trash2,
  uninstall: Download,
  purge: FolderOpen,
  optimize: Zap,
};

export function HistoryPage() {
  const { t } = useT();
  const [sessions, setSessions] = useState<HistorySession[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    invoke<HistoryData>("get_history", { limit: 50 })
      .then((data) => {
        setSessions(data.sessions ?? []);
        setLoading(false);
      })
      .catch(() => setLoading(false));
  }, []);

  return (
    <div className="p-8 max-w-3xl space-y-6">
      <div>
        <h1 className="text-xl font-semibold flex items-center gap-2">
          <History size={20} className="text-surface-300" />
          {t("history.title")}
        </h1>
        <p className="text-sm text-surface-400 mt-1">
          {t("history.subtitle")}
        </p>
      </div>

      {loading && (
        <div className="text-sm text-surface-400">{t("history.loading")}</div>
      )}

      {!loading && sessions.length === 0 && (
        <div className="text-sm text-surface-400 bg-surface-800 border border-surface-700 rounded-xl p-6 text-center">
          {t("history.empty")}
        </div>
      )}

      {sessions.length > 0 && (
        <div className="space-y-2">
          {sessions.map((session, idx) => {
            const Icon = commandIcons[session.command] ?? History;
            return (
              <div
                key={idx}
                className="bg-surface-800 border border-surface-700 rounded-lg p-3 flex items-center gap-3"
              >
                <div className="w-8 h-8 bg-surface-700 rounded-lg flex items-center justify-center shrink-0">
                  <Icon size={14} className="text-surface-300" />
                </div>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium capitalize">
                      {session.command}
                    </span>
                    <span className="text-xs text-surface-500">
                      {session.started_at}
                    </span>
                  </div>
                  <div className="text-xs text-surface-400 mt-0.5">
                    {t("history.itemsOps", { items: session.items, ops: session.operation_count })}
                  </div>
                </div>
                <div className="text-right shrink-0">
                  <div className="text-sm font-medium text-mole-400">
                    {session.size}
                  </div>
                  <div className="text-xs text-surface-500">
                    {t("history.trashedRemoved", { trashed: session.actions.trashed, removed: session.actions.removed })}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
