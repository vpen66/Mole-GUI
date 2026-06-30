import { Loader2 } from "lucide-react";
import { useT } from "@/i18n";
import type { ProgressEvent } from "@/types/common";

interface ProgressBarProps {
  events: ProgressEvent[];
  label?: string;
}

export function ProgressBar({ events, label }: ProgressBarProps) {
  const { t } = useT();
  const latestEvent = events[events.length - 1];
  const percent = latestEvent?.percent;
  const message = latestEvent?.message ?? label ?? t("common.working");
  const section = latestEvent?.section ?? "";

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between text-xs text-surface-400">
        <div className="flex items-center gap-2">
          <Loader2 size={12} className="animate-spin text-mole-400" />
          <span>{section ? `${section}: ${message}` : message}</span>
        </div>
        {percent !== undefined && <span>{Math.round(percent)}%</span>}
      </div>
      <div className="h-1.5 bg-surface-700 rounded-full overflow-hidden">
        <div
          className="h-full bg-mole-500 rounded-full transition-all duration-300"
          style={{ width: percent !== undefined ? `${percent}%` : "60%" }}
        />
      </div>
    </div>
  );
}
