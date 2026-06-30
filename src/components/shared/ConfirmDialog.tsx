import { AlertTriangle, X } from "lucide-react";
import { useT } from "@/i18n";
import { formatSize } from "@/types/common";

interface ConfirmDialogProps {
  open: boolean;
  title: string;
  message: string;
  totalSizeKb?: number;
  totalItems?: number;
  requiresSudo?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmDialog({
  open,
  title,
  message,
  totalSizeKb,
  totalItems,
  requiresSudo,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  const { t } = useT();
  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
      <div className="bg-surface-800 border border-surface-600 rounded-xl shadow-2xl max-w-md w-full mx-4">
        <div className="flex items-center justify-between p-4 border-b border-surface-700">
          <h3 className="font-semibold text-sm">{title}</h3>
          <button
            onClick={onCancel}
            className="text-surface-400 hover:text-surface-200 transition-colors"
          >
            <X size={16} />
          </button>
        </div>

        <div className="p-4 space-y-3">
          <p className="text-sm text-surface-300">{message}</p>

          {totalSizeKb !== undefined && totalSizeKb > 0 && (
            <div className="bg-surface-900 rounded-lg p-3 flex items-center justify-between">
              <span className="text-xs text-surface-400">{t("dialog.spaceToFree")}</span>
              <span className="text-sm font-medium text-mole-400">
                {formatSize(totalSizeKb)}
              </span>
            </div>
          )}

          {totalItems !== undefined && totalItems > 0 && (
            <div className="bg-surface-900 rounded-lg p-3 flex items-center justify-between">
              <span className="text-xs text-surface-400">{t("common.items")}</span>
              <span className="text-sm font-medium">{totalItems}</span>
            </div>
          )}

          {requiresSudo && (
            <div className="flex items-start gap-2 text-yellow-400 bg-yellow-950/30 border border-yellow-800/40 rounded-lg p-3">
              <AlertTriangle size={14} className="shrink-0 mt-0.5" />
              <span className="text-xs">
                {t("dialog.sudoWarning")}
              </span>
            </div>
          )}
        </div>

        <div className="flex gap-2 p-4 border-t border-surface-700">
          <button
            onClick={onCancel}
            className="flex-1 px-4 py-2 text-sm rounded-lg border border-surface-600 text-surface-300 hover:bg-surface-700 transition-colors"
          >
            {t("common.cancel")}
          </button>
          <button
            onClick={onConfirm}
            className="flex-1 px-4 py-2 text-sm rounded-lg bg-mole-600 text-white font-medium hover:bg-mole-700 transition-colors"
          >
            {t("common.execute")}
          </button>
        </div>
      </div>
    </div>
  );
}
