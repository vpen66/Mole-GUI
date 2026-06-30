import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useT } from "@/i18n";
import { HardDrive, Folder, File, ChevronRight, ArrowLeft, Eye, Trash2, RotateCcw, CheckCircle, Play, Square } from "lucide-react";
import { formatBytes } from "@/types/analyze";
import type { AnalyzeResult, AnalyzeEntry, AnalyzeLargeFile, AnalyzeStreamEvent } from "@/types/analyze";
import { ErrorBanner } from "@/components/shared/ErrorBanner";
import { DeleteConfirmDialog } from "@/components/shared/DeleteConfirmDialog";
import { useScanStore } from "@/hooks/useScanStore";

export function AnalyzePage() {
  const { t } = useT();
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set());
  const [deleting, setDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);

  // Use global scan store for persistent state across tab switches
  const scanStore = useScanStore();

  // Use pathStack from global store (persists across tab switches)
  const pathStack = useScanStore((s) => s.pathStack);
  const initialScanDone = useRef(false);
  const scanAbortRef = useRef(0);

  const currentPath = pathStack.length > 0 ? pathStack[pathStack.length - 1] ?? null : null;
  
  // Get current state from store
  const result = scanStore.getResult(currentPath);
  const loading = scanStore.isLoading(currentPath);
  const error = scanStore.getError(currentPath);
  const entryCount = scanStore.getEntryCount(currentPath);

  // Refs for throttled batch updates – accumulate events in refs,
  // then flush to the Zustand store on a timer so we don't trigger
  // a React re-render for every single NDJSON line.
  const entriesRef = useRef<AnalyzeEntry[]>([]);
  const largeFilesRef = useRef<AnalyzeLargeFile[]>([]);
  const summaryRef = useRef({ path: "/", overview: true, totalSize: 0, totalFiles: undefined as number | undefined });
  const scanKeyRef = useRef<string | null>(null);
  const flushTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const FLUSH_INTERVAL = 500; // ms – flush at most ~2 times/sec to reduce re-renders

  const flushToStore = useCallback(() => {
    const key = scanKeyRef.current;
    scanStore.batchUpdate(
      key,
      [...entriesRef.current],
      [...largeFilesRef.current],
      { ...summaryRef.current },
    );
  }, [scanStore]);

  const startFlushTimer = useCallback(() => {
    if (flushTimerRef.current) clearInterval(flushTimerRef.current);
    flushTimerRef.current = setInterval(flushToStore, FLUSH_INTERVAL);
  }, [flushToStore]);

  const stopFlushTimer = useCallback(() => {
    if (flushTimerRef.current) {
      clearInterval(flushTimerRef.current);
      flushTimerRef.current = null;
    }
  }, []);

  const scan = useCallback(async (path?: string) => {
    // Abort any previous in-flight scan so its completion cannot
    // overwrite the new scan's data.
    scanAbortRef.current = (scanAbortRef.current ?? 0) + 1;
    const myAbortId = scanAbortRef.current;

    const key: string | null = path ?? null;
    console.log(`[Analyze] scan() called for path="${path ?? '/'}" abortId=${myAbortId}`);

    // Update the key ref so flushToStore knows where to write
    scanKeyRef.current = key;

    // Stop any previous flush timer
    stopFlushTimer();

    // Reset accumulation refs
    entriesRef.current = [];
    largeFilesRef.current = [];
    summaryRef.current = { path: path ?? "/", overview: !path, totalSize: 0, totalFiles: undefined };

    // Start scan in store
    scanStore.startScan(key);
    scanStore.setEntryCount(key, 0);

    const isOverview = !path;

    // Listen for streaming NDJSON events from the backend (batch array)
    const unlisten = await listen<AnalyzeStreamEvent[]>(
      "mole-analyze_scan-event",
      (event) => {
        if (myAbortId !== scanAbortRef.current) return;
        const payloads = event.payload;

        for (const payload of payloads) {
          switch (payload.type) {
            case "progress":
              // progress events are lightweight – no store update needed
              break;
            case "entry": {
              const entryPath = payload.path;
              // For specific directory scans, verify the entry is under that directory
              let shouldAdd = true;
              if (!isOverview && path && path !== "/") {
                shouldAdd = entryPath.startsWith(path + "/") || entryPath === path || entryPath.startsWith(path);
              }
              if (shouldAdd) {
                entriesRef.current.push({
                  name: payload.name,
                  path: payload.path,
                  size: payload.size,
                  is_dir: payload.is_dir,
                  insight: payload.insight,
                  cleanable: payload.cleanable,
                  last_access: payload.last_access,
                });
              }
              break;
            }
            case "large_file": {
              const largeFilePath = payload.path;
              let shouldAdd = true;
              if (!isOverview && path && path !== "/") {
                shouldAdd = largeFilePath.startsWith(path + "/") || largeFilePath === path || largeFilePath.startsWith(path);
              }
              if (shouldAdd) {
                largeFilesRef.current.push({
                  name: payload.name,
                  path: payload.path,
                  size: payload.size,
                });
              }
              break;
            }
            case "summary":
              summaryRef.current = {
                path: payload.path,
                overview: payload.overview,
                totalSize: payload.total_size,
                totalFiles: payload.total_files,
              };
              break;
          }
        }
      }
    );

    // Start periodic flush so UI updates incrementally
    startFlushTimer();

    try {
      await invoke<string>("analyze_scan", { path: path ?? null });

      // If a newer scan was started while we were waiting, bail out.
      if (myAbortId !== scanAbortRef.current) return;

      // Final flush to make sure everything is in the store
      stopFlushTimer();
      flushToStore();

      // Final result from accumulated events
      const finalResult: AnalyzeResult = {
        path: summaryRef.current.path,
        overview: summaryRef.current.overview,
        entries: entriesRef.current,
        large_files: largeFilesRef.current.length > 0 ? largeFilesRef.current : undefined,
        total_size: summaryRef.current.totalSize,
        total_files: summaryRef.current.totalFiles,
      };

      scanStore.completeScan(key, finalResult);
    } catch (err) {
      if (myAbortId !== scanAbortRef.current) return;
      stopFlushTimer();
      scanStore.setError(key, err instanceof Error ? err.message : String(err));
    } finally {
      unlisten();
    }
  }, [scanStore, flushToStore, startFlushTimer, stopFlushTimer]);

  // Trigger initial scan on mount if no result exists for current path
  useEffect(() => {
    if (!initialScanDone.current) {
      initialScanDone.current = true;
      const key = currentPath ?? null;
      const store = useScanStore.getState();
      if (!store.getResult(key) && !store.isLoading(key)) {
        scan(currentPath || undefined);
      }
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);



  const handleDrillDown = (entry: AnalyzeEntry) => {
    if (!entry.is_dir) return;
    console.log(`[Analyze] handleDrillDown -> ${entry.path}, cached=${!!scanStore.getResult(entry.path)}`);
    // Clear selection when navigating
    setSelectedPaths(new Set());
    scanStore.pushPath(entry.path);
    // Use cached result if available; only scan if not yet loaded
    if (!scanStore.getResult(entry.path)) {
      scan(entry.path);
    }
  };

  // Handle clicking on a breadcrumb path segment
  const handleBreadcrumbClick = (targetPath: string | null) => {
    // Clear selection when navigating
    setSelectedPaths(new Set());
    if (targetPath === null) {
      // Go back to overview (root)
      scanStore.clearPaths();
      // Use cached result if available
      if (!scanStore.getResult(null)) {
        scan(undefined);
      }
    } else {
      // Find the index of the target path in the stack
      const currentStack = useScanStore.getState().pathStack;
      const targetIndex = currentStack.indexOf(targetPath);
      
      if (targetIndex !== -1) {
        // Pop paths until we reach the target
        const popsNeeded = currentStack.length - targetIndex - 1;
        for (let i = 0; i < popsNeeded; i++) {
          scanStore.popPath();
        }
        
        // Scan the target path only if not already cached
        if (!scanStore.getResult(targetPath)) {
          scan(targetPath);
        }
      } else {
        // Path not in stack, need to navigate there directly
        scanStore.setPathStack([targetPath]);
        // Use cached result if available
        if (!scanStore.getResult(targetPath)) {
          scan(targetPath);
        }
      }
    }
  };

  const handleRefresh = useCallback(() => {
    // Always trigger a fresh scan (corresponds to 'r' key in mole CLI)
    scanStore.clearPath(currentPath);
    scan(currentPath || undefined);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentPath]);

  const handleStopScan = useCallback(async () => {
    // Increment abort ref to cancel current scan on frontend
    scanAbortRef.current = (scanAbortRef.current ?? 0) + 1;
    // Stop the flush timer
    stopFlushTimer();
    // Tell the Rust backend to kill the mole process
    try {
      await invoke("cancel_analyze_scan");
    } catch {
      // ignore
    }
    // Clear loading state
    scanStore.setLoading(currentPath, false);
    console.log('[Analyze] Scan stopped by user');
  }, [currentPath, scanStore, stopFlushTimer]);

  const handleSelectToggle = (path: string) => {
    setSelectedPaths(prev => {
      const newSet = new Set(prev);
      if (newSet.has(path)) {
        newSet.delete(path);
      } else {
        newSet.add(path);
      }
      return newSet;
    });
  };

  const handleSelectAll = () => {
    if (!result) return;
    // Select all items: entries + large files
    const allPaths = [
      ...result.entries.map(e => e.path),
      ...(result.large_files?.map(f => f.path) ?? []),
    ];
    
    if (allPaths.length === 0) return;

    // Check if ALL current items are already selected
    const allSelected = allPaths.every(p => selectedPaths.has(p));
    if (allSelected) {
      // Deselect all current items (keep any extras from other dirs)
      setSelectedPaths(prev => {
        const next = new Set(prev);
        for (const p of allPaths) next.delete(p);
        return next;
      });
    } else {
      // Select all current items
      setSelectedPaths(new Set(allPaths));
    }
  };

  const handleDelete = async () => {
    if (selectedPaths.size === 0) return;
    
    // Show confirmation dialog
    setShowDeleteConfirm(true);
  };

  const handleDeleteConfirm = async () => {
    setDeleting(true);
    setDeleteError(null);
    
    try {
      const result = await invoke("analyze_delete", {
        paths: Array.from(selectedPaths),
      });
      
      // Check if the operation was successful
      if (result && typeof result === 'object' && 'success' in result) {
        const cleanResult = result as { success: boolean; error?: string };
        
        if (!cleanResult.success) {
          throw new Error(cleanResult.error || "Failed to delete files");
        }
      }
      
      // Clear selection after successful delete
      setSelectedPaths(new Set());
      
      // Close dialog
      setShowDeleteConfirm(false);
      
      // Refresh the current view
      setTimeout(() => {
        scan(currentPath || undefined);
      }, 500);
    } catch (err) {
      setDeleteError(err instanceof Error ? err.message : String(err));
      setShowDeleteConfirm(false);
    } finally {
      setDeleting(false);
    }
  };

  const handleDeleteCancel = () => {
    setShowDeleteConfirm(false);
  };

  // Stable callback for select toggle – avoids creating new functions in render
  const onSelectToggleRef = useRef<Map<string, () => void>>(new Map());
  const getSelectToggle = useCallback((path: string) => {
    if (!onSelectToggleRef.current.has(path)) {
      onSelectToggleRef.current.set(path, () => handleSelectToggle(path));
    }
    return onSelectToggleRef.current.get(path)!;
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const sortedEntries = useMemo(
    () => result ? [...result.entries].sort((a, b) => b.size - a.size) : [],
    [result?.entries]
  );

  const maxSize = sortedEntries[0]?.size ?? 1;

  return (
    <div className="p-8 max-w-4xl space-y-6">
      <div>
        <h1 className="text-xl font-semibold flex items-center gap-2">
          <HardDrive size={20} className="text-cyan-400" />
          {t("analyze.title")}
        </h1>
        <p className="text-sm text-surface-400 mt-1">
          {currentPath ? (
            <span className="flex items-center gap-1 flex-wrap">
              {/* Overview link */}
              <button
                onClick={() => handleBreadcrumbClick(null)}
                className="flex items-center gap-1 text-cyan-400 hover:text-cyan-300 transition-colors"
              >
                <ArrowLeft size={13} />
                {t("analyze.overview")}
              </button>
              
              {/* Breadcrumb path segments */}
              {(() => {
                // Parse the current path into segments
                const pathSegments = currentPath.split('/').filter(Boolean);
                
                return pathSegments.map((segment, index) => {
                  // Build absolute path: join all segments up to and including current one
                  const accumulatedPath = '/' + pathSegments.slice(0, index + 1).join('/');
                  const isLast = index === pathSegments.length - 1;
                  
                  return (
                    <React.Fragment key={accumulatedPath}>
                      <ChevronRight size={12} className="text-surface-500 shrink-0" />
                      {!isLast ? (
                        <button
                          onClick={() => {
                            handleBreadcrumbClick(accumulatedPath);
                          }}
                          className={`font-mono text-xs truncate max-w-[150px] text-cyan-400 hover:text-cyan-300 transition-colors cursor-pointer`}
                          title={accumulatedPath}
                        >
                          {segment}
                        </button>
                      ) : (
                        <span
                          className={`font-mono text-xs truncate max-w-[150px] text-surface-300`}
                          title={accumulatedPath}
                        >
                          {segment}
                        </span>
                      )}
                    </React.Fragment>
                  );
                });
              })()}
            </span>
          ) : (
            t("analyze.systemOverview")
          )}
        </p>
      </div>

      <ErrorBanner message={error} onDismiss={() => scanStore.setError(currentPath, null)} />
      {deleteError && (
        <ErrorBanner message={deleteError} onDismiss={() => setDeleteError(null)} />
      )}

      {/* Action toolbar */}
      <div className="flex items-center gap-2">
        {/* Stop Scan button - only show when scanning */}
        {loading && (
          <button
            onClick={handleStopScan}
            className="flex items-center gap-2 px-3 py-1.5 text-xs font-medium bg-red-600/20 border border-red-600/50 text-red-400 rounded-lg hover:bg-red-600/30 transition-colors"
          >
            <Square size={14} />
            {t("analyze.stopScan")}
          </button>
        )}
        
        {result && (
          <>
            {result.entries.length > 0 && (
              <button
                onClick={handleSelectAll}
                disabled={deleting}
                className="flex items-center gap-2 px-3 py-1.5 text-xs font-medium bg-surface-800 border border-surface-700 rounded-lg hover:bg-surface-750 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                <CheckCircle size={14} />
                {(() => {
                  const totalItems = result.entries.length + (result.large_files?.length ?? 0);
                  if (totalItems === 0) return null;
                  const allSelected = result.entries.every(e => selectedPaths.has(e.path)) &&
                    (result.large_files ?? []).every(f => selectedPaths.has(f.path));
                  return allSelected ? t("analyze.deselectAll") : t("analyze.selectAll");
                })()}
              </button>
            )}
            
            <button
              onClick={handleRefresh}
              disabled={deleting}
              className="flex items-center gap-2 px-3 py-1.5 text-xs font-medium bg-surface-800 border border-surface-700 rounded-lg hover:bg-surface-750 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <RotateCcw size={14} className={loading ? "animate-spin" : ""} />
              {t("common.refresh")}
            </button>
            
            {selectedPaths.size > 0 && (
              <button
                onClick={handleDelete}
                disabled={deleting}
                className="flex items-center gap-2 px-3 py-1.5 text-xs font-medium bg-red-600/20 border border-red-600/50 text-red-400 rounded-lg hover:bg-red-600/30 transition-colors disabled:opacity-50 disabled:cursor-not-allowed ml-auto"
              >
                <Trash2 size={14} />
                {t("analyze.delete")} ({selectedPaths.size})
              </button>
            )}
          </>
        )}
        
        {/* Start Scan button - only when no result and not loading */}
        {!result && !loading && !error && (
          <button
            onClick={() => scan(currentPath || undefined)}
            disabled={deleting}
            className="flex items-center gap-2 px-3 py-1.5 text-xs font-medium bg-cyan-600 hover:bg-cyan-700 rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <Play size={14} />
            {t("analyze.startScan")}
          </button>
        )}
      </div>

      {/* Delete Confirmation Dialog */}
      <DeleteConfirmDialog
        isOpen={showDeleteConfirm}
        itemCount={selectedPaths.size}
        onConfirm={handleDeleteConfirm}
        onCancel={handleDeleteCancel}
        title={t("analyze.deleteTitle")}
        confirmText={t("analyze.moveToTrash")}
      />

      {/* Initial state - no scan started yet */}
      {!result && !loading && !error && (
        <div className="py-12 flex flex-col items-center justify-center gap-4">
          <HardDrive size={48} className="text-surface-500" />
          <div className="text-sm text-surface-400">
            {currentPath ? t("analyze.scanningDir") : t("analyze.systemOverview")}
          </div>
        </div>
      )}

      {/* Scanning indicator - shows during background scans (even when results are streaming) */}
      {loading && result && result.entries.length === 0 && (
        <div className="py-12 flex flex-col items-center justify-center gap-4">
          {/* Pulsing scan animation */}
          <div className="relative w-16 h-16">
            <div className="absolute inset-0 rounded-full border-2 border-cyan-400/20" />
            <div className="absolute inset-0 rounded-full border-2 border-transparent border-t-cyan-400 animate-spin" />
            <div className="absolute inset-2 rounded-full border-2 border-transparent border-t-cyan-300 animate-spin" style={{ animationDuration: '1.5s', animationDirection: 'reverse' }} />
            <div className="absolute inset-4 rounded-full border-2 border-transparent border-t-cyan-200 animate-spin" style={{ animationDuration: '2s' }} />
            <HardDrive size={16} className="absolute inset-0 m-auto text-cyan-400" />
          </div>
          <div className="text-sm text-surface-400">
            {currentPath ? t("analyze.scanningDir") : t("analyze.scanningOverview")}
          </div>
        </div>
      )}

      {result && (
        <>
          {/* Scanning indicator - shows during background scans */}
          {loading && (
            <div className="flex items-center gap-2 text-xs text-cyan-400 mb-3">
              <div className="w-3 h-3 border-2 border-cyan-400 border-t-transparent rounded-full animate-spin" />
              <span>{t("analyze.scanningInBackground")} {entryCount > 0 && t("analyze.entriesFound", { count: entryCount })}</span>
            </div>
          )}

          {/* Summary bar (only when scan complete) */}
          {!loading && (
            <div className="bg-surface-800 border border-surface-700 rounded-xl p-4 flex items-center justify-between">
              <div className="text-sm text-surface-300">
                <span className="font-medium text-white">
                  {formatBytes(result.total_size)}
                </span>
                <span className="text-surface-400 ml-1">
                  {result.overview ? t("analyze.totalUsed") : t("analyze.inThisDir")}
                </span>
              </div>
              {result.total_files !== undefined && (
                <div className="text-xs text-surface-400">
                  {result.total_files.toLocaleString()} {t("common.files")}
                </div>
              )}
            </div>
          )}

          {/* Entries list */}
          <div className="space-y-1">
            {sortedEntries.map((entry) => (
              <EntryRow
                key={entry.path}
                entry={entry}
                maxSize={maxSize}
                onDrillDown={handleDrillDown}
                isSelected={selectedPaths.has(entry.path)}
                onSelectToggle={getSelectToggle(entry.path)}
              />
            ))}
          </div>

          {/* Large files section */}
          {result.large_files && result.large_files.length > 0 && (
            <div className="mt-6">
              <h2 className="text-sm font-medium text-surface-300 mb-2 flex items-center gap-2">
                <File size={14} className="text-amber-400" />
                {t("analyze.largeFiles")}
              </h2>
              <div className="space-y-1">
                {result.large_files
                  .sort((a, b) => b.size - a.size)
                  .map((file) => (
                    <LargeFileRow
                      key={file.path}
                      file={file}
                      isSelected={selectedPaths.has(file.path)}
                      onSelectToggle={getSelectToggle(file.path)}
                    />
                  ))}
              </div>
            </div>
          )}
        </>
      )}
    </div>
  );
}

const EntryRow = React.memo(function EntryRow({
  entry,
  maxSize,
  onDrillDown,
  isSelected = false,
  onSelectToggle,
}: {
  entry: AnalyzeEntry;
  maxSize: number;
  onDrillDown: (e: AnalyzeEntry) => void;
  isSelected?: boolean;
  onSelectToggle?: () => void;
}) {
  const { t } = useT();
  const pct = maxSize > 0 ? Math.max(2, (entry.size / maxSize) * 100) : 2;
  const isDir = entry.is_dir;

  const handleRowClick = () => {
    if (isDir) {
      onDrillDown(entry);
    }
  };

  return (
    <div
      onClick={handleRowClick}
      className={`w-full flex items-center gap-3 px-3 py-2 border rounded-lg hover:bg-surface-750 transition-colors group text-left ${
        isDir ? 'cursor-pointer' : 'cursor-default'
      } ${
        isSelected
          ? 'bg-cyan-900/30 border-cyan-500/50'
          : 'bg-surface-800 border-surface-700'
      }`}
    >
      {/* Selection checkbox */}
      <div 
        onClick={(e: React.MouseEvent) => {
          e.stopPropagation();
          onSelectToggle?.();
        }}
        className={`flex-shrink-0 w-4 h-4 rounded border flex items-center justify-center cursor-pointer transition-colors ${
          isSelected
            ? 'bg-cyan-500 border-cyan-500'
            : 'border-surface-600 hover:border-surface-500'
        }`}
      >
        {isSelected && <CheckCircle size={12} className="text-white" />}
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2 mb-1">
          {isDir ? (
            entry.insight ? (
              <Eye size={13} className="text-purple-400 shrink-0" />
            ) : (
              <Folder size={13} className="text-cyan-400 shrink-0" />
            )
          ) : (
            <File size={13} className="text-surface-400 shrink-0" />
          )}
          <span className="text-sm text-surface-200 truncate">{entry.name}</span>
          {entry.cleanable && (
            <span className="text-[10px] text-green-400 uppercase font-medium shrink-0">
              {t("analyze.cleanable")}
            </span>
          )}
        </div>
        <div className="h-1.5 bg-surface-700 rounded-full overflow-hidden">
          <div
            className={`h-full rounded-full transition-all ${
              entry.insight
                ? "bg-purple-500"
                : entry.cleanable
                ? "bg-green-500"
                : "bg-cyan-500"
            }`}
            style={{ width: `${pct}%` }}
          />
        </div>
      </div>
      <div className="flex items-center gap-2 shrink-0">
        <span className="text-xs font-medium text-surface-300">
          {formatBytes(entry.size)}
        </span>
        {isDir && (
          <ChevronRight
            size={13}
            className="text-surface-500 group-hover:text-surface-300 transition-colors"
          />
        )}
      </div>
    </div>
  );
});

const LargeFileRow = React.memo(function LargeFileRow({
  file,
  isSelected = false,
  onSelectToggle,
}: {
  file: AnalyzeLargeFile;
  isSelected?: boolean;
  onSelectToggle?: () => void;
}) {
  return (
    <div
      className={`flex items-center gap-3 px-3 py-2 border rounded-lg text-xs transition-colors ${
        isSelected
          ? 'bg-cyan-900/30 border-cyan-500/50'
          : 'bg-surface-800 border-surface-700'
      }`}
    >
      {/* Selection checkbox */}
      <div 
        onClick={(e: React.MouseEvent) => {
          e.stopPropagation();
          onSelectToggle?.();
        }}
        className={`flex-shrink-0 w-4 h-4 rounded border flex items-center justify-center cursor-pointer transition-colors ${
          isSelected
            ? 'bg-cyan-500 border-cyan-500'
            : 'border-surface-600 hover:border-surface-500'
        }`}
      >
        {isSelected && <CheckCircle size={12} className="text-white" />}
      </div>
      <File size={13} className="text-amber-400 shrink-0" />
      <span className="text-surface-300 truncate flex-1 font-mono">
        {file.name}
      </span>
      <span className="text-amber-400 ml-3 shrink-0">
        {formatBytes(file.size)}
      </span>
    </div>
  );
});
