import { AlertCircle } from "lucide-react";
import { useT } from "@/i18n";

interface ErrorBannerProps {
  message: string | null;
  onDismiss?: () => void;
}

export function ErrorBanner({ message, onDismiss }: ErrorBannerProps) {
  const { t } = useT();
  if (!message) return null;

  return (
    <div className="flex items-start gap-2 bg-red-950/40 border border-red-800/50 rounded-lg p-3 text-sm text-red-300">
      <AlertCircle size={14} className="shrink-0 mt-0.5" />
      <div className="flex-1">{message}</div>
      {onDismiss && (
        <button
          onClick={onDismiss}
          className="text-red-400 hover:text-red-200 text-xs"
        >
          {t("common.dismiss")}
        </button>
      )}
    </div>
  );
}
