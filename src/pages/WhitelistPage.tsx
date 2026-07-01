import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useT } from "@/i18n";
import {
  ShieldAlert,
  Plus,
  Trash2,
  Check,
  FolderPlus,
  FolderOpen,
} from "lucide-react";
import { ErrorBanner } from "@/components/shared/ErrorBanner";

// Predefined cache items from mole's get_all_cache_items
interface PredefinedCleanItem {
  name: string;
  nameEn: string;
  pattern: string;
  category: "ide_cache" | "compiler_cache" | "package_manager" | "ai_ml_cache" | "system_cache" | "browser_cache" | "network_tools";
}

const PREDEFINED_CLEAN_ITEMS: PredefinedCleanItem[] = [
  // System Caches
  { name: "Finder 缓存元数据 (.DS_Store)", nameEn: "Finder metadata, .DS_Store", pattern: "FINDER_METADATA", category: "system_cache" },
  { name: "字体注册缓存", nameEn: "Font cache", pattern: "~/Library/Caches/com.apple.FontRegistry*", category: "system_cache" },
  { name: "Spotlight 索引缓存", nameEn: "Spotlight metadata cache", pattern: "~/Library/Caches/com.apple.spotlight*", category: "system_cache" },
  { name: "CloudKit 缓存文件", nameEn: "CloudKit cache", pattern: "~/Library/Caches/CloudKit*", category: "system_cache" },
  { name: "系统邮件缓存", nameEn: "Apple Mail cache", pattern: "~/Library/Caches/com.apple.mail/*", category: "system_cache" },
  { name: "废纸篓", nameEn: "Trash", pattern: "~/.Trash", category: "system_cache" },
  
  // IDE / Editor Caches
  { name: "Gradle 守护进程缓存", nameEn: "Gradle daemon processes cache", pattern: "~/.gradle/daemon/*", category: "ide_cache" },
  { name: "Gradle 依赖构建缓存", nameEn: "Gradle build cache", pattern: "~/.gradle/caches/build-cache-*/*", category: "ide_cache" },
  { name: "Gradle 工作器缓存", nameEn: "Gradle worker cache", pattern: "~/.gradle/workers/*", category: "ide_cache" },
  { name: "Xcode 索引与编译产物 (DerivedData)", nameEn: "Xcode DerivedData", pattern: "~/Library/Developer/Xcode/DerivedData/*", category: "ide_cache" },
  { name: "Xcode 内部临时缓存", nameEn: "Xcode internal cache files", pattern: "~/Library/Caches/com.apple.dt.Xcode/*", category: "ide_cache" },
  { name: "Xcode iOS 设备符号支持缓存", nameEn: "Xcode iOS device support symbols", pattern: "~/Library/Developer/Xcode/iOS DeviceSupport/*/Symbols/System/Library/Caches/*", category: "ide_cache" },
  { name: "Maven 本地依赖库", nameEn: "Maven local repository", pattern: "~/.m2/repository/*", category: "ide_cache" },
  { name: "JetBrains IDE 配置文件与数据 (IntelliJ, PyCharm等)", nameEn: "JetBrains IDEs data", pattern: "~/Library/Application Support/JetBrains*", category: "ide_cache" },
  { name: "JetBrains IDE 运行缓存", nameEn: "JetBrains IDEs cache", pattern: "~/Library/Caches/JetBrains*", category: "ide_cache" },
  { name: "Android Studio 索引与缓存", nameEn: "Android Studio cache and indexes", pattern: "~/Library/Caches/Google/AndroidStudio*/*", category: "ide_cache" },
  { name: "VS Code 编辑器运行缓存", nameEn: "VS Code runtime cache", pattern: "~/Library/Application Support/Code/Cache/*", category: "ide_cache" },
  { name: "VS Code 插件扩展缓存", nameEn: "VS Code extension and update cache", pattern: "~/Library/Application Support/Code/CachedData/*", category: "ide_cache" },
  { name: "VS Code 系统底层缓存", nameEn: "VS Code system cache", pattern: "~/Library/Caches/com.microsoft.VSCode/*", category: "ide_cache" },
  { name: "Cursor 编辑器运行缓存", nameEn: "Cursor editor cache", pattern: "~/Library/Caches/com.todesktop.230313mzl4w4u92/*", category: "ide_cache" },

  // Compiler / Build Tools
  { name: "Bazel 编译构建缓存", nameEn: "Bazel build cache", pattern: "~/.cache/bazel/*", category: "compiler_cache" },
  { name: "Go 语言编译构建缓存", nameEn: "Go build cache", pattern: "~/Library/Caches/go-build/*", category: "compiler_cache" },
  { name: "Go Module 模块依赖包", nameEn: "Go module cache", pattern: "~/go/pkg/mod/*", category: "compiler_cache" },
  { name: "Rust Cargo 依赖注册表缓存", nameEn: "Rust Cargo registry cache", pattern: "~/.cargo/registry/cache/*", category: "compiler_cache" },
  { name: "SBT (Scala) 依赖构建缓存", nameEn: "SBT Scala build cache", pattern: "~/.sbt/*", category: "compiler_cache" },
  { name: "Vite 热构建缓存", nameEn: "Vite build cache", pattern: "~/.vite/*", category: "compiler_cache" },
  { name: "Next.js 项目构建缓存", nameEn: "Next.js build cache", pattern: "~/.next/*", category: "compiler_cache" },

  // Package Managers
  { name: "CocoaPods 本地依赖包缓存 (iOS)", nameEn: "CocoaPods cache", pattern: "~/Library/Caches/CocoaPods/*", category: "package_manager" },
  { name: "npm 全局依赖包缓存", nameEn: "npm package cache", pattern: "~/.npm/_cacache/*", category: "package_manager" },
  { name: "pip Python 依赖包缓存", nameEn: "pip Python package cache", pattern: "~/.cache/pip/*", category: "package_manager" },
  { name: "uv 高速 Python 缓存", nameEn: "uv Python package cache", pattern: "~/.cache/uv/*", category: "package_manager" },
  { name: "pnpm 共享依赖包仓库", nameEn: "pnpm package store", pattern: "~/Library/pnpm/store/*", category: "package_manager" },
  { name: "Homebrew 下载软件安装包缓存", nameEn: "Homebrew downloaded packages", pattern: "~/Library/Caches/Homebrew/*", category: "package_manager" },

  // AI / ML Models
  { name: "HuggingFace 模型与数据集缓存", nameEn: "HuggingFace models and datasets", pattern: "~/.cache/huggingface/*", category: "ai_ml_cache" },
  { name: "Playwright 无头浏览器环境内核", nameEn: "Playwright browser binaries", pattern: "~/Library/Caches/ms-playwright*", category: "ai_ml_cache" },
  { name: "Ollama 本地大模型文件", nameEn: "Ollama local AI models", pattern: "~/.ollama/models/*", category: "ai_ml_cache" },

  // Browsers & Proxy
  { name: "Safari 浏览器运行缓存", nameEn: "Safari web browser cache", pattern: "~/Library/Caches/com.apple.Safari/*", category: "browser_cache" },
  { name: "Chrome 谷歌浏览器运行缓存", nameEn: "Chrome browser cache", pattern: "~/Library/Caches/Google/Chrome/*", category: "browser_cache" },
  { name: "Surge 代理加速缓存", nameEn: "Surge proxy cache", pattern: "~/Library/Caches/com.nssurge.surge-mac/*", category: "network_tools" },
  { name: "Surge 配置数据与网络日志", nameEn: "Surge configuration and data", pattern: "~/Library/Application Support/com.nssurge.surge-mac/*", category: "network_tools" },
];

// Predefined optimize ignore items from mole's get_optimize_whitelist_items
interface PredefinedOptimizeItem {
  id: string;
  name: string;
  nameEn: string;
  description: string;
  descriptionEn: string;
}

const PREDEFINED_OPTIMIZE_ITEMS: PredefinedOptimizeItem[] = [
  { id: "system_maintenance", name: "DNS 与 Spotlight 检查", nameEn: "DNS & Spotlight Check", description: "跳过 DNS 及系统 Spotlight 服务运行状态检查", descriptionEn: "Skip checking DNS and system Spotlight service status" },
  { id: "cache_refresh", name: "Finder 缓存刷新", nameEn: "Finder Cache Refresh", description: "跳过 Finder 缓存及挂载图标清理", descriptionEn: "Skip clearing Finder caches and icon databases" },
  { id: "saved_state_cleanup", name: "应用窗口状态清理", nameEn: "App State Cleanup", description: "跳过对应用上一次退出时保存的状态文件的清理", descriptionEn: "Skip removing window saved states left by closed apps" },
  { id: "fix_broken_configs", name: "破损配置修复", nameEn: "Broken Config Repair", description: "跳过系统破损 .plist 配置文件的校验与修复", descriptionEn: "Skip checking and repairing corrupted PLIST configs" },
  { id: "network_optimization", name: "网络缓存刷新", nameEn: "Network Cache Refresh", description: "跳过系统网络接口与连接缓存的重置", descriptionEn: "Skip flushing network interface and connection caches" },
  { id: "sqlite_vacuum", name: "数据库收缩优化", nameEn: "Database Optimization", description: "跳过对偏合设置 SQLite 数据库的整理压缩", descriptionEn: "Skip vacuuming and optimizing configuration databases" },
  { id: "launch_services_rebuild", name: "LaunchServices 注册修复", nameEn: "LaunchServices Repair", description: "跳过系统打开方式 (LaunchServices) 数据库的重建", descriptionEn: "Skip rebuilding LaunchServices Open With databases" },
  { id: "dock_refresh", name: "Dock 栏刷新", nameEn: "Dock Refresh", description: "跳过 Dock 重启刷新及位置缓存清理", descriptionEn: "Skip restarting Dock and clearing position caches" },
  { id: "prevent_network_dsstore", name: "禁用网络路径 .DS_Store", nameEn: "Prevent Finder .DS_Store", description: "跳过在网络共享或外接磁盘上创建 .DS_Store 文件的禁用设置", descriptionEn: "Skip disabling .DS_Store creation on network drives" },
  { id: "memory_pressure_relief", name: "内存压力释放", nameEn: "Memory Optimization", description: "跳过执行系统内存整理以释放未使用内存", descriptionEn: "Skip purging inactive RAM to relieve memory pressure" },
  { id: "network_stack_optimize", name: "网络栈参数优化", nameEn: "Network Stack Refresh", description: "跳过 TCP/IP 堆栈内核参数的性能微调", descriptionEn: "Skip fine-tuning TCP/IP kernel stack configurations" },
  { id: "disk_permissions_repair", name: "磁盘权限修复", nameEn: "Permission Repair", description: "跳过对特定系统关键目录的权限检验和修复", descriptionEn: "Skip checking and repairing system directory permissions" },
  { id: "spotlight_index_optimize", name: "Spotlight 索引优化", nameEn: "Spotlight Optimization", description: "跳过重建或整理全局 Spotlight 搜索引擎索引", descriptionEn: "Skip optimizing and indexing globally in Spotlight" },
  { id: "spotlight_orphan_rules_cleanup", name: "Spotlight 孤立规则清理", nameEn: "Spotlight Orphan Rules", description: "跳过已卸载应用的 Spotlight 索引规则清理", descriptionEn: "Skip cleaning orphan search filters of uninstalled apps" },
  { id: "periodic_maintenance", name: "系统周期维护脚本", nameEn: "Periodic Maintenance", description: "跳过执行 macOS 每日/每周/每月定期日常清理脚本", descriptionEn: "Skip running daily, weekly, or monthly system maintenance" },
  { id: "shared_file_list_repair", name: "常用文件列表修复", nameEn: "Shared File Lists", description: "跳过 Finder 最近使用文件列表缓存清理", descriptionEn: "Skip clearing and repairing Shared File Lists cache" },
  { id: "disk_verify", name: "磁盘健康状态检验", nameEn: "Disk Health", description: "跳过使用 DiskUtil 检验系统硬盘健康度与损坏状态", descriptionEn: "Skip verifying core startup volumes and disk headers" },
  { id: "login_items_audit", name: "开机自启项审计", nameEn: "Login Items Audit", description: "跳过对可能减缓开机速度的无主启动项的扫描", descriptionEn: "Skip auditing login launch components and agents" },
  { id: "quarantine_cleanup", name: "下载安全隔离数据库清理", nameEn: "Quarantine Database Cleanup", description: "跳过网络下载项安全沙盒记录 (Quarantine) 的归档清理", descriptionEn: "Skip cleaning download history isolation logs" },
  { id: "launch_agents_cleanup", name: "启动守护进程代理清理", nameEn: "Launch Agents Cleanup", description: "跳过检测和清理已卸载残留应用的系统自启守护进程", descriptionEn: "Skip checking orphan system startup daemons in Library" },
  { id: "notification_cleanup", name: "通知中心缓存清理", nameEn: "Notifications", description: "跳过对通知中心过期消息数据及缓存的清理", descriptionEn: "Skip clearing accumulated notification alerts and logs" },
  { id: "coreduet_cleanup", name: "系统使用记录数据清理", nameEn: "Usage Data", description: "跳过对 macOS 后台 CoreDuet 用户活动统计数据的清理", descriptionEn: "Skip clearing CoreDuet diagnostic databases of activity" },
];

export function WhitelistPage() {
  const { t, locale } = useT();
  const isZh = locale === "zh";

  // Tabs status
  const [activeTab, setActiveTab] = useState<"clean" | "optimize" | "purge" | "overview">("clean");

  // Clean Whitelist states
  const [activeCleanPatterns, setActiveCleanPatterns] = useState<string[]>([]);
  const [customCleanPaths, setCustomCleanPaths] = useState<string[]>([]);
  const [newPathInput, setNewPathInput] = useState("");

  // Optimize Whitelist states
  const [activeOptimizeTasks, setActiveOptimizeTasks] = useState<string[]>([]);

  // Purge Paths states
  const [purgePaths, setPurgePaths] = useState<string[]>([]);
  const [newPurgePathInput, setNewPurgePathInput] = useState("");

  // Overview 概览扫描目录白名单状态管理
  interface OverviewDir {
    name: string;
    path: string;
    is_insight: boolean;
    is_downloads: boolean;
    exclude_path?: string;
  }
  const [overviewDirs, setOverviewDirs] = useState<OverviewDir[]>([]);
  const [newDirName, setNewDirName] = useState("");
  const [newDirPath, setNewDirPath] = useState("");
  const [newIsInsight, setNewIsInsight] = useState(false);
  const [newIsDownloads, setNewIsDownloads] = useState(false);

  // Status & Error states
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saveStatus, setSaveStatus] = useState<"idle" | "success" | "error">("idle");

  // Fetch all configs on load
  const loadConfigs = async () => {
    setLoading(true);
    setError(null);
    try {
      // 1. Fetch clean whitelist
      const cleanList = await invoke<string[]>("get_whitelist_config", { mode: "clean" });
      
      // Separate predefined and custom paths
      const predefinedPatterns = PREDEFINED_CLEAN_ITEMS.map((item) => item.pattern);
      const custom: string[] = [];
      const activePredefined: string[] = [];

      // Check user configurations
      if (cleanList.length > 0) {
        cleanList.forEach((pat) => {
          if (predefinedPatterns.includes(pat)) {
            activePredefined.push(pat);
          } else {
            custom.push(pat);
          }
        });
        setActiveCleanPatterns(activePredefined);
        setCustomCleanPaths(custom);
      } else {
        // If file doesn't exist, we load CLI defaults
        const defaultPatterns = [
          "~/Library/Caches/ms-playwright*",
          "~/.cache/huggingface*",
          "~/.m2/repository/*",
          "~/.gradle/caches/*",
          "~/.gradle/daemon/*",
          "~/.ollama/models/*",
          "~/Library/Caches/com.nssurge.surge-mac/*",
          "~/Library/Application Support/com.nssurge.surge-mac/*",
          "~/Library/Caches/org.R-project.R/R/renv/*",
          "~/Library/Caches/pypoetry/virtualenvs*",
          "~/Library/Caches/JetBrains*",
          "~/Library/Caches/com.jetbrains.toolbox*",
          "~/Library/Caches/tealdeer/tldr-pages",
          "~/Library/Application Support/JetBrains*",
          "~/Library/Caches/com.apple.finder",
          "~/Library/Mobile Documents*",
          "~/Library/Caches/com.apple.FontRegistry*",
          "~/Library/Caches/com.apple.spotlight*",
          "~/Library/Caches/com.apple.Spotlight*",
          "~/Library/Caches/CloudKit*",
          "FINDER_METADATA",
        ];
        setActiveCleanPatterns(defaultPatterns);
        setCustomCleanPaths([]);
      }

      // 2. Fetch optimize ignore list
      const optimizeList = await invoke<string[]>("get_whitelist_config", { mode: "optimize" });
      setActiveOptimizeTasks(optimizeList);

      // 3. Fetch purge paths
      const purgeList = await invoke<string[]>("get_purge_paths");
      setPurgePaths(purgeList);

      // 4. Fetch overview directories config
      const overviewList = await invoke<OverviewDir[]>("get_overview_dirs");
      setOverviewDirs(overviewList);
    } catch (err) {
      console.error("Failed to load configs:", err);
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadConfigs();
  }, []);

  // Save Clean Whitelist
  const handleSaveCleanWhitelist = async (updatedPredefined: string[], updatedCustom: string[]) => {
    setLoading(true);
    setSaveStatus("idle");
    setError(null);
    try {
      const allPatterns = [...updatedPredefined, ...updatedCustom];
      await invoke("save_whitelist_config", { mode: "clean", patterns: allPatterns });
      setSaveStatus("success");
      setTimeout(() => setSaveStatus("idle"), 2000);
    } catch (err) {
      setSaveStatus("error");
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  // Toggle predefined item in clean whitelist
  const toggleCleanPredefined = (pattern: string) => {
    const next = activeCleanPatterns.includes(pattern)
      ? activeCleanPatterns.filter((p) => p !== pattern)
      : [...activeCleanPatterns, pattern];
    setActiveCleanPatterns(next);
    handleSaveCleanWhitelist(next, customCleanPaths);
  };

  // Add custom path to clean whitelist
  const handleAddCustomCleanPath = () => {
    const val = newPathInput.trim();
    if (!val) return;
    if (customCleanPaths.includes(val)) {
      setNewPathInput("");
      return;
    }
    const next = [...customCleanPaths, val];
    setCustomCleanPaths(next);
    setNewPathInput("");
    handleSaveCleanWhitelist(activeCleanPatterns, next);
  };

  // Remove custom path from clean whitelist
  const handleRemoveCustomCleanPath = (index: number) => {
    const next = customCleanPaths.filter((_, i) => i !== index);
    setCustomCleanPaths(next);
    handleSaveCleanWhitelist(activeCleanPatterns, next);
  };

  // Toggle optimize ignore task
  const toggleOptimizeTask = async (taskId: string) => {
    setLoading(true);
    setError(null);
    try {
      const next = activeOptimizeTasks.includes(taskId)
        ? activeOptimizeTasks.filter((t) => t !== taskId)
        : [...activeOptimizeTasks, taskId];
      setActiveOptimizeTasks(next);
      await invoke("save_whitelist_config", { mode: "optimize", patterns: next });
      setSaveStatus("success");
      setTimeout(() => setSaveStatus("idle"), 2000);
    } catch (err) {
      setSaveStatus("error");
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  // Add project purge directory
  const handleAddPurgePath = async () => {
    const val = newPurgePathInput.trim();
    if (!val) return;
    if (purgePaths.includes(val)) {
      setNewPurgePathInput("");
      return;
    }
    const next = [...purgePaths, val];
    setPurgePaths(next);
    setNewPurgePathInput("");
    setLoading(true);
    try {
      await invoke("save_purge_paths", { paths: next });
      setSaveStatus("success");
      setTimeout(() => setSaveStatus("idle"), 2000);
    } catch (err) {
      setSaveStatus("error");
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  // Remove project purge directory
  const handleRemovePurgePath = async (index: number) => {
    const next = purgePaths.filter((_, i) => i !== index);
    setPurgePaths(next);
    setLoading(true);
    try {
      await invoke("save_purge_paths", { paths: next });
      setSaveStatus("success");
      setTimeout(() => setSaveStatus("idle"), 2000);
    } catch (err) {
      setSaveStatus("error");
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleAddOverviewDir = async () => {
    if (!newDirName || !newDirPath) return;
    const newDir: OverviewDir = {
      name: newDirName,
      path: newDirPath,
      is_insight: newIsInsight,
      is_downloads: newIsDownloads,
    };
    const updated = [...overviewDirs, newDir];
    setOverviewDirs(updated);
    setNewDirName("");
    setNewDirPath("");
    setNewIsInsight(false);
    setNewIsDownloads(false);

    setLoading(true);
    try {
      await invoke("set_overview_dirs", { dirs: updated });
      setSaveStatus("success");
      setTimeout(() => setSaveStatus("idle"), 2000);
    } catch (err) {
      setSaveStatus("error");
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleRemoveOverviewDir = async (index: number) => {
    const updated = overviewDirs.filter((_, i) => i !== index);
    setOverviewDirs(updated);

    setLoading(true);
    try {
      await invoke("set_overview_dirs", { dirs: updated });
      setSaveStatus("success");
      setTimeout(() => setSaveStatus("idle"), 2000);
    } catch (err) {
      setSaveStatus("error");
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="p-8 max-w-4xl space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <ShieldAlert size={22} className="text-mole-400 shrink-0" />
          <div>
            <h1 className="text-xl font-semibold">{t("whitelist.title")}</h1>
            <p className="text-sm text-surface-400 mt-0.5">
              {t("whitelist.subtitle")}
            </p>
          </div>
        </div>
        
        <div className="flex items-center gap-2">
          {loading && (
            <div className="w-4 h-4 border-2 border-mole-400 border-t-transparent rounded-full animate-spin shrink-0" />
          )}
          {saveStatus === "success" && (
            <div className="flex items-center gap-1.5 text-xs text-green-400 bg-green-950/20 border border-green-900/30 px-3 py-1.5 rounded-lg shadow-sm animate-fade-in">
              <Check size={12} />
              <span>{t("whitelist.saveSuccess")}</span>
            </div>
          )}
        </div>
      </div>

      <ErrorBanner message={error} onDismiss={() => setError(null)} />

      {/* Tabs Menu */}
      <div className="flex border-b border-surface-700">
        {(["clean", "optimize", "purge", "overview"] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            className={`px-5 py-3 text-sm font-medium border-b-2 -mb-[2px] transition-all duration-200 ${
              activeTab === tab
                ? "border-mole-500 text-mole-400 font-semibold"
                : "border-transparent text-surface-400 hover:text-surface-200"
            }`}
          >
            {t(`whitelist.tab.${tab}`)}
          </button>
        ))}
      </div>

      {/* Tab Panels */}
      <div className="pt-2">
        {/* Panel 1: Clean Whitelist */}
        {activeTab === "clean" && (
          <div className="space-y-6">
            {/* Custom paths box */}
            <div className="bg-surface-800 border border-surface-700 rounded-xl p-5 space-y-4 shadow-sm">
              <div className="flex items-center gap-2 text-surface-200">
                <FolderPlus size={16} className="text-mole-400" />
                <h2 className="text-sm font-medium">{t("whitelist.customPathsSection")}</h2>
              </div>

              {/* Path entry list */}
              {customCleanPaths.length > 0 ? (
                <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
                  {customCleanPaths.map((path, idx) => (
                    <div
                      key={idx}
                      className="flex items-center justify-between bg-surface-900 border border-surface-750 px-3 py-2 rounded-lg text-xs font-mono text-surface-200 group"
                    >
                      <span className="truncate flex-1 pr-2">{path}</span>
                      <button
                        onClick={() => handleRemoveCustomCleanPath(idx)}
                        className="text-surface-500 hover:text-red-400 p-1 rounded hover:bg-surface-800 transition-colors opacity-0 group-hover:opacity-100 focus:opacity-100"
                      >
                        <Trash2 size={12} />
                      </button>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="text-xs text-surface-500 italic py-2">
                  暂无自定义白名单路径 (No custom whitelisted paths yet)
                </div>
              )}

              {/* Add Input */}
              <div className="space-y-1.5 pt-2">
                <label className="text-xs text-surface-400">
                  {t("whitelist.addCustomPath")}
                </label>
                <div className="flex gap-2">
                  <input
                    type="text"
                    value={newPathInput}
                    onChange={(e) => setNewPathInput(e.target.value)}
                    onKeyDown={(e) => e.key === "Enter" && handleAddCustomCleanPath()}
                    placeholder={t("whitelist.customPathPlaceholder")}
                    className="flex-1 bg-surface-900 border border-surface-650 rounded-lg px-3 py-2 text-sm font-mono text-surface-200 placeholder:text-surface-600 focus:outline-none focus:border-mole-500 focus:ring-1 focus:ring-mole-500/20"
                  />
                  <button
                    onClick={handleAddCustomCleanPath}
                    className="px-4 py-2 bg-mole-600 hover:bg-mole-500 text-white text-sm font-medium rounded-lg transition-colors flex items-center gap-1.5"
                  >
                    <Plus size={14} />
                    {t("whitelist.add")}
                  </button>
                </div>
                <p className="text-[10px] text-surface-500">
                  {t("whitelist.customPathHelp")}
                </p>
              </div>
            </div>

            {/* Predefined checkboxes */}
            <div className="bg-surface-800 border border-surface-700 rounded-xl p-5 space-y-4 shadow-sm">
              <div className="flex items-center gap-2 text-surface-200 border-b border-surface-700 pb-2">
                <ShieldAlert size={16} className="text-mole-400" />
                <h2 className="text-sm font-medium">{t("whitelist.predefinedSection")}</h2>
              </div>

              {/* Cache categories layout */}
              <div className="space-y-4">
                {(["system_cache", "ide_cache", "compiler_cache", "package_manager", "ai_ml_cache", "browser_cache", "network_tools"] as const).map((cat) => {
                  const catItems = PREDEFINED_CLEAN_ITEMS.filter((i) => i.category === cat);
                  if (catItems.length === 0) return null;

                  const categoryLabels: Record<string, string> = {
                    system_cache: isZh ? "系统关键与核心缓存" : "System Caches & Core",
                    ide_cache: isZh ? "集成开发环境与编辑器" : "IDEs & Editors",
                    compiler_cache: isZh ? "编译器与构建工具缓存" : "Compiler Caches",
                    package_manager: isZh ? "依赖包管理器仓库" : "Package Managers",
                    ai_ml_cache: isZh ? "AI大模型与自动化环境" : "AI & ML Model Caches",
                    browser_cache: isZh ? "网页浏览器缓存" : "Web Browsers Caches",
                    network_tools: isZh ? "网络代理与配置数据" : "Proxy & Network Tools",
                  };

                  return (
                    <div key={cat} className="space-y-2">
                      <h3 className="text-xs font-semibold text-mole-400/80 tracking-wide uppercase">
                        {categoryLabels[cat]}
                      </h3>
                      <div className="grid grid-cols-1 md:grid-cols-2 gap-2.5">
                        {catItems.map((item) => {
                          const active = activeCleanPatterns.includes(item.pattern);
                          return (
                            <button
                              key={item.pattern}
                              onClick={() => toggleCleanPredefined(item.pattern)}
                              className={`flex items-start text-left gap-3 p-3 rounded-lg border transition-all duration-200 ${
                                active
                                  ? "bg-mole-950/15 border-mole-800/60 hover:bg-mole-950/25"
                                  : "bg-surface-900 border-surface-750/70 hover:bg-surface-800"
                              }`}
                            >
                              <div
                                className={`w-4 h-4 rounded mt-0.5 border flex items-center justify-center shrink-0 transition-colors ${
                                  active
                                    ? "bg-mole-600 border-mole-500 text-white"
                                    : "border-surface-600 bg-surface-950"
                                }`}
                              >
                                {active && <Check size={10} strokeWidth={4} />}
                              </div>
                              <div className="space-y-0.5">
                                <div className="text-xs font-medium text-surface-200">
                                  {isZh ? item.name : item.nameEn}
                                </div>
                                <div className="text-[10px] font-mono text-surface-500 truncate max-w-[280px]">
                                  {item.pattern}
                                </div>
                              </div>
                            </button>
                          );
                        })}
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        )}

        {/* Panel 2: Optimize Whitelist */}
        {activeTab === "optimize" && (
          <div className="bg-surface-800 border border-surface-700 rounded-xl p-5 space-y-4 shadow-sm">
            <div className="flex items-center gap-2 border-b border-surface-700 pb-2">
              <ShieldAlert size={16} className="text-mole-400" />
              <h2 className="text-sm font-medium">{t("whitelist.optimizeSection")}</h2>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              {PREDEFINED_OPTIMIZE_ITEMS.map((item) => {
                const ignored = activeOptimizeTasks.includes(item.id);
                return (
                  <button
                    key={item.id}
                    onClick={() => toggleOptimizeTask(item.id)}
                    className={`flex items-start text-left gap-3 p-3 rounded-lg border transition-all duration-200 ${
                      ignored
                        ? "bg-amber-950/10 border-amber-900/35 hover:bg-amber-950/15"
                        : "bg-surface-900 border-surface-750/70 hover:bg-surface-800"
                    }`}
                  >
                    <div
                      className={`w-4 h-4 rounded mt-0.5 border flex items-center justify-center shrink-0 transition-colors ${
                        ignored
                          ? "bg-amber-600 border-amber-500 text-white"
                          : "border-surface-600 bg-surface-950"
                      }`}
                    >
                      {ignored && <Check size={10} strokeWidth={4} />}
                    </div>
                    <div>
                      <div className="text-xs font-semibold text-surface-200">
                        {isZh ? item.name : item.nameEn}
                      </div>
                      <div className="text-[10px] text-surface-400 mt-0.5 leading-normal">
                        {isZh ? item.description : item.descriptionEn}
                      </div>
                    </div>
                  </button>
                );
              })}
            </div>
          </div>
        )}

        {/* Panel 3: Purge Scan Paths */}
        {activeTab === "purge" && (
          <div className="bg-surface-800 border border-surface-700 rounded-xl p-5 space-y-4 shadow-sm">
            <div className="flex items-center gap-2 text-surface-200">
              <FolderOpen size={16} className="text-mole-400" />
              <h2 className="text-sm font-medium">{t("whitelist.purgeSection")}</h2>
            </div>
            
            <p className="text-xs text-surface-400 leading-normal">
              {t("whitelist.purgePathsHelp")}
            </p>

            {/* Path List */}
            {purgePaths.length > 0 ? (
              <div className="space-y-1.5 max-h-72 overflow-y-auto pr-1">
                {purgePaths.map((path, idx) => (
                  <div
                    key={idx}
                    className="flex items-center justify-between bg-surface-900 border border-surface-750 px-3 py-2 rounded-lg text-xs font-mono text-surface-200 group"
                  >
                    <span className="truncate flex-1 pr-2">{path}</span>
                    <button
                      onClick={() => handleRemovePurgePath(idx)}
                      className="text-surface-500 hover:text-red-400 p-1 rounded hover:bg-surface-800 transition-colors opacity-0 group-hover:opacity-100 focus:opacity-100"
                    >
                      <Trash2 size={12} />
                    </button>
                  </div>
                ))}
              </div>
            ) : (
              <div className="text-xs text-surface-500 italic py-2">
                暂无自定义项目扫描路径 (No custom purge scan directories configured yet)
              </div>
            )}

            {/* Add Input */}
            <div className="space-y-1.5 pt-2 border-t border-surface-700">
              <div className="flex gap-2">
                <input
                  type="text"
                  value={newPurgePathInput}
                  onChange={(e) => setNewPurgePathInput(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && handleAddPurgePath()}
                  placeholder={t("whitelist.customPathPlaceholder")}
                  className="flex-1 bg-surface-900 border border-surface-650 rounded-lg px-3 py-2 text-sm font-mono text-surface-200 placeholder:text-surface-600 focus:outline-none focus:border-mole-500 focus:ring-1 focus:ring-mole-500/20"
                />
                <button
                  onClick={handleAddPurgePath}
                  className="px-4 py-2 bg-mole-600 hover:bg-mole-500 text-white text-sm font-medium rounded-lg transition-colors flex items-center gap-1.5"
                >
                  <Plus size={14} />
                  {t("whitelist.add")}
                </button>
              </div>
              <p className="text-[10px] text-surface-500">
                支持使用 ~ 代表用户主目录，如 ~/Projects/work
              </p>
            </div>
          </div>
        )}

        {/* Panel 4: Overview Scan Whitelist */}
        {activeTab === "overview" && (
          <div className="bg-surface-800 border border-surface-700 rounded-xl p-5 space-y-4 shadow-sm animate-fade-in">
            <div className="flex items-center gap-2 border-b border-surface-700 pb-2 text-surface-200">
              <FolderOpen size={16} className="text-mole-400" />
              <h2 className="text-sm font-medium">{t("whitelist.overviewSection")}</h2>
            </div>
            
            <p className="text-xs text-surface-400 leading-normal">
              {t("whitelist.overviewHelp")}
            </p>

            {/* Existing items list */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3 max-h-96 overflow-y-auto pr-1">
              {overviewDirs.map((dir, idx) => (
                <div
                  key={idx}
                  className="flex items-center justify-between bg-surface-900 border border-surface-750 rounded-lg p-3 text-xs text-surface-300 relative group"
                >
                  <div className="min-w-0 flex-1 mr-2">
                    <span className="font-semibold text-white block truncate">{dir.name}</span>
                    <span className="font-mono text-surface-500 block truncate mt-0.5">{dir.path}</span>
                    <div className="flex gap-2 mt-1 text-[10px]">
                      {dir.is_insight && <span className="text-purple-400 bg-purple-950/20 px-1.5 rounded">Insight</span>}
                      {dir.is_downloads && <span className="text-amber-400 bg-amber-950/20 px-1.5 rounded">Downloads 90d</span>}
                      {dir.exclude_path && <span className="text-surface-500 font-mono">Exclude: {dir.exclude_path}</span>}
                    </div>
                  </div>
                  <button
                    onClick={() => handleRemoveOverviewDir(idx)}
                    className="p-1.5 text-surface-400 hover:text-red-400 hover:bg-surface-800 rounded transition-colors shrink-0"
                    title="删除 (Delete)"
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              ))}
              {overviewDirs.length === 0 && (
                <div className="text-center py-6 text-xs text-surface-500 col-span-2">暂无配置的概览白名单目录</div>
              )}
            </div>

            {/* Add new item form */}
            <div className="border-t border-surface-750 pt-4 space-y-3">
              <div className="text-xs font-semibold text-surface-300">新增概览目录配置：</div>
              <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                <input
                  type="text"
                  placeholder="名称 (如 Gradle Cache)"
                  value={newDirName}
                  onChange={(e) => setNewDirName(e.target.value)}
                  className="w-full px-3 py-2 text-xs rounded-lg bg-surface-900 border border-surface-700 text-white placeholder:text-surface-650 focus:outline-none focus:border-mole-500"
                />
                <input
                  type="text"
                  placeholder="绝对路径 (如 ~/.gradle/caches)"
                  value={newDirPath}
                  onChange={(e) => setNewDirPath(e.target.value)}
                  className="w-full px-3 py-2 text-xs font-mono rounded-lg bg-surface-900 border border-surface-700 text-white placeholder:text-surface-650 focus:outline-none focus:border-mole-500"
                />
              </div>
              <div className="flex flex-wrap items-center gap-x-5 gap-y-2 text-xs mt-1">
                <label className="flex items-center gap-1.5 cursor-pointer text-surface-400 hover:text-surface-300">
                  <input
                    type="checkbox"
                    checked={newIsInsight}
                    onChange={(e) => setNewIsInsight(e.target.checked)}
                    className="rounded border-surface-700 text-mole-600 focus:ring-mole-500 bg-surface-900 w-3.5 h-3.5"
                  />
                  <span>开发者洞察 (大小为 0 时在主列表自动隐藏)</span>
                </label>
                <label className="flex items-center gap-1.5 cursor-pointer text-surface-400 hover:text-surface-300">
                  <input
                    type="checkbox"
                    checked={newIsDownloads}
                    onChange={(e) => setNewIsDownloads(e.target.checked)}
                    className="rounded border-surface-700 text-mole-600 focus:ring-mole-500 bg-surface-900 w-3.5 h-3.5"
                  />
                  <span>只统计 90 天以上未修改文件</span>
                </label>
              </div>
              <button
                onClick={handleAddOverviewDir}
                disabled={!newDirName || !newDirPath}
                className="flex items-center gap-1.5 px-4 py-2 text-xs font-medium bg-mole-600 hover:bg-mole-500 border border-mole-500 text-white rounded-lg transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
              >
                <Plus size={12} />
                添加项目
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
