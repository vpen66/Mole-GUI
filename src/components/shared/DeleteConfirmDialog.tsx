import { useState } from "react";
import { useT } from "@/i18n";
import { Trash2, X } from "lucide-react";

interface DeleteConfirmDialogProps {
  isOpen: boolean;
  itemCount: number;
  onConfirm: () => void;
  onCancel: () => void;
  title?: string;
  message?: string;
  confirmText?: string;
  cancelText?: string;
}

export function DeleteConfirmDialog({
  isOpen,
  itemCount,
  onConfirm,
  onCancel,
  title,
  message,
  confirmText,
  cancelText,
}: DeleteConfirmDialogProps) {
  const { t } = useT();
  const [isConfirming, setIsConfirming] = useState(false);

  if (!isOpen) return null;

  const resolvedTitle = title ?? t("delete.title");
  const resolvedConfirmText = confirmText ?? t("delete.moveToTrash");
  const resolvedCancelText = cancelText ?? t("common.cancel");
  const defaultMessage = itemCount === 1
    ? t("delete.confirmSingle")
    : t("delete.confirmMultiple", { count: itemCount });

  const handleConfirm = async () => {
    setIsConfirming(true);
    try {
      await onConfirm();
    } finally {
      setIsConfirming(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-sm">
      <div className="bg-surface-900 border border-surface-700 rounded-xl shadow-2xl max-w-md w-full mx-4 p-6 animate-in fade-in zoom-in duration-200">
        {/* Header */}
        <div className="flex items-start justify-between mb-4">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-red-500/10 rounded-lg">
              <Trash2 size={20} className="text-red-400" />
            </div>
            <h3 className="text-lg font-semibold text-white">{resolvedTitle}</h3>
          </div>
          <button
            onClick={onCancel}
            disabled={isConfirming}
            className="p-1 hover:bg-surface-800 rounded-lg transition-colors disabled:opacity-50"
          >
            <X size={18} className="text-surface-400" />
          </button>
        </div>

        {/* Message */}
        <div className="mb-6">
          <p className="text-sm text-surface-300 leading-relaxed">
            {message || defaultMessage}
          </p>
        </div>

        {/* Actions */}
        <div className="flex items-center gap-3 justify-end">
          <button
            onClick={onCancel}
            disabled={isConfirming}
            className="px-4 py-2 text-sm font-medium text-surface-300 bg-surface-800 border border-surface-700 rounded-lg hover:bg-surface-750 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {resolvedCancelText}
          </button>
          <button
            onClick={handleConfirm}
            disabled={isConfirming}
            className="px-4 py-2 text-sm font-medium text-white bg-red-600 rounded-lg hover:bg-red-700 transition-colors disabled:opacity-70 disabled:cursor-not-allowed flex items-center gap-2 min-w-[120px] justify-center"
          >
            {isConfirming ? (
              <>
                <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                {t("common.deleting")}
              </>
            ) : (
              <>
                <Trash2 size={14} />
                {resolvedConfirmText}
              </>
            )}
          </button>
        </div>
      </div>
    </div>
  );
}
