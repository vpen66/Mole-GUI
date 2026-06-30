import { useEffect, useState, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useT, SUPPORTED_LOCALES } from "@/i18n";
import { Settings2, FolderOpen, RotateCcw, Check, AlertCircle, Languages } from "lucide-react";

interface MolePathConfig {
  custom_path: string;
  resolved_path: string;
}

export function SettingsPage() {
  const { t, locale, setLocale } = useT();
  const [config, setConfig] = useState<MolePathConfig | null>(null);
  const [inputPath, setInputPath] = useState("");
  const [saving, setSaving] = useState(false);
  const [saveStatus, setSaveStatus] = useState<"idle" | "success" | "error">("idle");
  const [errorMsg, setErrorMsg] = useState("");

  const loadConfig = useCallback(async () => {
    try {
      const cfg = await invoke<MolePathConfig>("get_mole_path_config");
      setConfig(cfg);
      setInputPath(cfg.custom_path);
    } catch (err) {
      console.error("Failed to load settings:", err);
    }
  }, []);

  useEffect(() => {
    loadConfig();
  }, [loadConfig]);

  const handleSave = async () => {
    setSaving(true);
    setSaveStatus("idle");
    setErrorMsg("");
    try {
      const cfg = await invoke<MolePathConfig>("set_mole_path_config", {
        path: inputPath.trim(),
      });
      setConfig(cfg);
      setSaveStatus("success");
      setTimeout(() => setSaveStatus("idle"), 2000);
    } catch (err) {
      setSaveStatus("error");
      setErrorMsg(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  const handleReset = async () => {
    setInputPath("");
    setSaving(true);
    setSaveStatus("idle");
    setErrorMsg("");
    try {
      const cfg = await invoke<MolePathConfig>("set_mole_path_config", {
        path: "",
      });
      setConfig(cfg);
      setSaveStatus("success");
      setTimeout(() => setSaveStatus("idle"), 2000);
    } catch (err) {
      setSaveStatus("error");
      setErrorMsg(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="p-8 space-y-8 max-w-2xl">
      <div className="flex items-center gap-3">
        <Settings2 size={20} className="text-surface-300" />
        <div>
          <h1 className="text-xl font-semibold">{t("settings.title")}</h1>
          <p className="text-sm text-surface-400 mt-0.5">
            {t("settings.subtitle")}
          </p>
        </div>
      </div>

      {/* Language Selection */}
      <div className="bg-surface-800 border border-surface-700 rounded-xl p-5 space-y-4">
        <div className="flex items-center gap-2">
          <Languages size={16} className="text-mole-400" />
          <h2 className="text-sm font-medium">{t("settings.language")}</h2>
        </div>

        <p className="text-xs text-surface-400">
          {t("settings.languageDesc")}
        </p>

        <div className="flex gap-2">
          {SUPPORTED_LOCALES.map((loc) => (
            <button
              key={loc.code}
              onClick={() => setLocale(loc.code)}
              className={`px-4 py-2 text-sm rounded-lg border transition-colors ${
                locale === loc.code
                  ? "bg-mole-600 border-mole-500 text-white"
                  : "border-surface-600 text-surface-300 hover:bg-surface-700"
              }`}
            >
              {loc.nativeLabel}
            </button>
          ))}
        </div>
      </div>

      {/* Mole CLI Path Configuration */}
      <div className="bg-surface-800 border border-surface-700 rounded-xl p-5 space-y-4">
        <div className="flex items-center gap-2">
          <FolderOpen size={16} className="text-mole-400" />
          <h2 className="text-sm font-medium">{t("settings.cliPath")}</h2>
        </div>

        <p className="text-xs text-surface-400">
          {t("settings.cliPathDesc")}
        </p>

        {/* Current resolved path */}
        {config?.resolved_path && (
          <div className="bg-surface-900 rounded-lg p-3">
            <div className="text-xs text-surface-500 mb-1">{t("settings.currentlyUsing")}</div>
            <div className="text-sm font-mono text-surface-200 break-all">
              {config.resolved_path}
            </div>
          </div>
        )}

        {/* Custom path input */}
        <div className="space-y-2">
          <label className="text-xs text-surface-400">
            {t("settings.customPathLabel")}
          </label>
          <div className="flex gap-2">
            <input
              type="text"
              value={inputPath}
              onChange={(e) => setInputPath(e.target.value)}
              placeholder={t("settings.pathPlaceholder")}
              className="flex-1 bg-surface-900 border border-surface-600 rounded-lg px-3 py-2 text-sm font-mono text-surface-200 placeholder:text-surface-600 focus:outline-none focus:border-mole-500 focus:ring-1 focus:ring-mole-500/30"
            />
            <button
              onClick={handleSave}
              disabled={saving}
              className="px-4 py-2 bg-mole-600 hover:bg-mole-500 disabled:opacity-50 text-white text-sm font-medium rounded-lg transition-colors"
            >
              {saving ? t("common.saving") : t("common.save")}
            </button>
          </div>
        </div>

        {/* Status messages */}
        {saveStatus === "success" && (
          <div className="flex items-center gap-2 text-sm text-green-400">
            <Check size={14} />
            {t("settings.saveSuccess")}
          </div>
        )}
        {saveStatus === "error" && (
          <div className="flex items-center gap-2 text-sm text-red-400">
            <AlertCircle size={14} />
            {errorMsg}
          </div>
        )}

        {/* Reset button */}
        {config?.custom_path && (
          <button
            onClick={handleReset}
            disabled={saving}
            className="flex items-center gap-2 text-xs text-surface-400 hover:text-surface-200 transition-colors"
          >
            <RotateCcw size={12} />
            {t("settings.resetToAutoDetect")}
          </button>
        )}
      </div>

      {/* About section */}
      <div className="bg-surface-800 border border-surface-700 rounded-xl p-5 space-y-3">
        <h2 className="text-sm font-medium">{t("settings.about")}</h2>
        <div className="grid grid-cols-2 gap-y-2 text-sm">
          <span className="text-surface-400">Mole GUI</span>
          <span className="text-surface-200">v1.0.0</span>
          <span className="text-surface-400">Mole CLI</span>
          <span className="text-surface-200 font-mono text-xs">
            {config?.resolved_path || t("settings.notFound")}
          </span>
        </div>
      </div>
    </div>
  );
}
