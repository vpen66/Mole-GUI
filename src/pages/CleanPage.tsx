import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useT } from "@/i18n";
import { ProgressBar } from "@/components/shared/ProgressBar";
import { ErrorBanner } from "@/components/shared/ErrorBanner";
import { ConfirmDialog } from "@/components/shared/ConfirmDialog";
import { formatSize } from "@/types/common";
import type { ItemEvent, SummaryEvent, MoleEvent } from "@/types/common";
import { useTabStore, type GroupedSection } from "@/hooks/useTabStore";
import {
  Trash2,
  Play,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
} from "lucide-react";
import type { CleanResult } from "@/types/clean";

export function CleanPage() {
  const { t } = useT();
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [done, setDone] = useState(false);
  const [expandedSections, setExpandedSections] = useState<Set<string>>(
    new Set()
  );

  const {
    status,
    progress,
    error,
    scanned,
    sections,
  } = useTabStore((s) => s.clean);
  const {
    setCleanStatus,
    setCleanError,
    setCleanProgress,
    setCleanScanned,
    setCleanSections,
    setCleanSummary,
  } = useTabStore();

  const scan = async () => {
    setCleanStatus("scanning");
    setCleanError(null);
    setCleanProgress([]);
    setCleanSections([]);
    setCleanSummary(null);

    const tempSections: GroupedSection[] = [];

    const unlisten = await listen<MoleEvent>("mole-clean_dry_run-event", (event) => {
      const payload = event.payload;
      if (payload.type === "progress") {
        setCleanProgress([payload as any]);
      } else if (payload.type === "item") {
        const item = payload as ItemEvent;
        const existing = tempSections.find((s) => s.name === item.section);
        if (existing) {
          existing.items.push(item);
          existing.totalKb += item.size_kb;
        } else {
          tempSections.push({ name: item.section, items: [item], totalKb: item.size_kb });
        }
        setCleanSections([...tempSections]);
      } else if (payload.type === "summary") {
        setCleanSummary(payload as SummaryEvent);
      }
    });

    try {
      await invoke<CleanResult>("clean_dry_run");
      setCleanStatus("preview");
      setCleanScanned(true);
    } catch (err) {
      setCleanError(err instanceof Error ? err.message : String(err));
    } finally {
      unlisten();
    }
  };

  const toggleSection = (name: string) => {
    setExpandedSections((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  };

  const handleExecute = async () => {
    setConfirmOpen(false);

    try {
      await invoke<CleanResult>("clean_execute");
      setDone(true);
    } catch (err) {
      console.error(err);
    }
  };

  const totalSizeKb = sections.reduce((sum, s) => sum + s.totalKb, 0);
  const totalItems = sections.reduce((sum, s) => sum + s.items.length, 0);

  return (
    <div className="p-8 max-w-3xl space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-xl font-semibold flex items-center gap-2">
            <Trash2 size={20} className="text-green-400" />
            {t("clean.title")}
          </h1>
          <p className="text-sm text-surface-400 mt-1">
            {t("clean.subtitle")}
          </p>
        </div>
        {status === "preview" && !done && (
          <button
            onClick={() => setConfirmOpen(true)}
            disabled={totalSizeKb === 0}
            className="flex items-center gap-2 px-4 py-2 bg-mole-600 hover:bg-mole-700 disabled:opacity-40 disabled:cursor-not-allowed text-white text-sm font-medium rounded-lg transition-colors"
          >
            <Play size={14} />
            {t("clean.execute")}
          </button>
        )}
      </div>

      <ErrorBanner message={error} onDismiss={() => setCleanError(null)} />

      {status === "scanning" && <ProgressBar events={progress} />}

      {done && (
        <div className="bg-mole-950/40 border border-mole-800/50 rounded-xl p-4 flex items-center gap-3">
          <CheckCircle2 size={20} className="text-mole-400 shrink-0" />
          <div>
            <div className="text-sm font-medium text-mole-300">
              {t("clean.complete")}
            </div>
            <div className="text-xs text-surface-400 mt-0.5">
              {t("clean.completeHint")}
            </div>
          </div>
        </div>
      )}

      {/* Initial state - no scan started yet */}
      {!scanned && status !== "scanning" && (
        <div className="py-12 flex flex-col items-center justify-center gap-4">
          <Trash2 size={48} className="text-surface-500" />
          <div className="text-sm text-surface-400">
            {t("clean.subtitle")}
          </div>
          <button
            onClick={scan}
            className="flex items-center gap-2 px-4 py-2 text-sm font-medium bg-green-600 hover:bg-green-700 rounded-lg transition-colors"
          >
            <Play size={16} />
            {t("clean.startScan")}
          </button>
        </div>
      )}

      {/* Scanning indicator - shows during background scans */}
      {status === "scanning" && scanned && (
        <div className="flex items-center gap-2 text-xs text-green-400">
          <div className="w-3 h-3 border-2 border-green-400 border-t-transparent rounded-full animate-spin" />
          <span>{t("clean.scanningInBackground")}</span>
        </div>
      )}

      {/* Sections list */}
      {sections.length > 0 && (
        <div className="space-y-2">
          <div className="flex items-center justify-between text-xs text-surface-400 pb-1 border-b border-surface-700">
            <span>
              {t("clean.categoriesItems", { count: sections.length, items: totalItems })}
            </span>
            <span className="text-mole-400 font-medium">
              {formatSize(totalSizeKb)} {t("clean.reclaimable")}
            </span>
          </div>

          {sections.map((section) => (
            <div
              key={section.name}
              className="bg-surface-800 border border-surface-700 rounded-lg overflow-hidden"
            >
              <button
                onClick={() => toggleSection(section.name)}
                className="w-full flex items-center justify-between p-3 hover:bg-surface-750 transition-colors"
              >
                <div className="flex items-center gap-2">
                  {expandedSections.has(section.name) ? (
                    <ChevronDown size={14} className="text-surface-400" />
                  ) : (
                    <ChevronRight size={14} className="text-surface-400" />
                  )}
                  <span className="text-sm">{section.name}</span>
                  <span className="text-xs text-surface-500">
                    ({section.items.length})
                  </span>
                </div>
                <span className="text-xs font-medium text-mole-400">
                  {formatSize(section.totalKb)}
                </span>
              </button>

              {expandedSections.has(section.name) && (
                <div className="border-t border-surface-700 divide-y divide-surface-700">
                  {section.items.map((item, idx) => (
                    <div
                      key={idx}
                      className="flex items-center justify-between px-8 py-2 text-xs"
                    >
                      <span className="text-surface-300 truncate flex-1">
                        {item.description}
                      </span>
                      <div className="flex items-center gap-2 ml-3 shrink-0">
                        <span className="text-surface-500">
                          {item.size_human}
                        </span>
                        {item.status === "dry_run" && (
                          <span className="text-yellow-400 text-[10px] uppercase font-medium">
                            dry
                          </span>
                        )}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      )}

      <ConfirmDialog
        open={confirmOpen}
        title={t("clean.confirmTitle")}
        message={t("clean.confirmMessage")}
        totalSizeKb={totalSizeKb}
        totalItems={totalItems}
        onConfirm={handleExecute}
        onCancel={() => setConfirmOpen(false)}
      />
    </div>
  );
}
