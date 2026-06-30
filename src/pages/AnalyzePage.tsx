import React, { useCallback, useEffect, useRef, useState } from "react";
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

  const scan = useCallback(async (path?: string) => {
    // Abort any previous in-flight scan so its completion cannot
    // overwrite the new scan's data.
    scanAbortRef.current = (scanAbortRef.current ?? 0) + 1;
    const myAbortId = scanAbortRef.current;

    const key: string | null = path ?? null;

    // Start scan in store
    scanStore.startScan(key);
    scanStore.setEntryCount(key, 0);

    // Accumulate NDJSON streaming events into an AnalyzeResult
    const entries: AnalyzeEntry[] = [];
    const largeFiles: AnalyzeLargeFile[] = [];
    let summaryPath = path ?? "/";
    let isOverview = !path;
    let totalSize = 0;
    let totalFiles: number | undefined;

    // Listen for streaming NDJSON events from the backend
    const unlisten = await listen<AnalyzeStreamEvent>(
      "mole-analyze_scan-event",
      (event) => {
        if (myAbortId !== scanAbortRef.current) return;
        const payload = event.payload;
        switch (payload.type) {
          case "progress":
            break;
          case "entry":
            entries.push({
              name: payload.name,
              path: payload.path,
              size: payload.size,
              is_dir: payload.is_dir,
              insight: payload.insight,
              cleanable: payload.cleanable,
              last_access: payload.last_access,
            });
            scanStore.setEntryCount(key, entries.length);
            // Update result incrementally so entries appear during scanning
            scanStore.updateResult(
              key,
              {
                path: summaryPath,
                overview: isOverview,
                total_size: totalSize,
                total_files: totalFiles,
              },
              [...entries],
              largeFiles.length > 0 ? [...largeFiles] : undefined
            );
            break;
          case "large_file":
            largeFiles.push({
              name: payload.name,
              path: payload.path,
              size: payload.size,
            });
            break;
          case "summary":
            summaryPath = payload.path;
            isOverview = payload.overview;
            totalSize = payload.total_size;
            totalFiles = payload.total_files;
            break;
        }
      }
    );

    try {
      await invoke<string>("analyze_scan", { path: path ?? null });

      // If a newer scan was started while we were waiting, bail out.
      if (myAbortId !== scanAbortRef.current) return;

      // Final result from accumulated events
      const finalResult: AnalyzeResult = {
        path: summaryPath,
        overview: isOverview,
        entries,
        large_files: largeFiles.length > 0 ? largeFiles : undefined,
        total_size: totalSize,
        total_files: totalFiles,
      };

      scanStore.completeScan(key, finalResult);
    } catch (err) {
      if (myAbortId !== scanAbortRef.current) return;
      scanStore.setError(key, err instanceof Error ? err.message : String(err));
    } finally {
      unlisten();
    }
  }, [scanStore]);

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
    scanStore.pushPath(entry.path);
    scan(entry.path);
  };

  // Handle clicking on a breadcrumb path segment
  const handleBreadcrumbClick = (targetPath: string | null) => {
    if (targetPath === null) {
      // Go back to overview (root)
      scanStore.clearPaths();
      scan(undefined);
    } else {
      // Find the index of the target path in the stack
      const currentStack = useScanStore.getState().pathStack;
      const targetIndex = currentStack.indexOf(targetPath);
      
      if (targetIndex !== -1) {
        // Pop paths until we reach the target
        // We need to pop (currentLength - targetIndex - 1) times
        const popsNeeded = currentStack.length - targetIndex - 1;
        
        for (let i = 0; i < popsNeeded; i++) {
          scanStore.popPath();
        }
        
        // Scan the target path if not already loaded
        if (!scanStore.getResult(targetPath)) {
          scan(targetPath);
        }
      } else {
        // Path not in stack, need to navigate there directly
        // Clear stack and set new path
        scanStore.setPathStack([targetPath]);
        scan(targetPath);
      }
    }
  };

  const handleRefresh = useCallback(() => {
    // Always trigger a fresh scan (corresponds to 'r' key in mole CLI)
    scanStore.clearPath(currentPath);
    scan(currentPath || undefined);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [currentPath]);

  const handleStopScan = useCallback(() => {
    // Increment abort ref to cancel current scan
    scanAbortRef.current = (scanAbortRef.current ?? 0) + 1;
    // Clear loading state
    scanStore.setLoading(currentPath, false);
    console.log('[Analyze] Scan stopped by user');
  }, [currentPath, scanStore]);

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
    
    if (selectedPaths.size === allPaths.length && allPaths.length > 0) {
      setSelectedPaths(new Set());
    } else {
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
      await invoke("analyze_delete", {
        paths: Array.from(selectedPaths),
      });
      
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

  const sortedEntries = result
    ? [...result.entries].sort((a, b) => b.size - a.size)
    : [];

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
                let accumulatedPath = '';
                
                return pathSegments.map((segment, index) => {
                  accumulatedPath += '/' + segment;
                  const isLast = index === pathSegments.length - 1;
                  
                  return (
                    <React.Fragment key={accumulatedPath}>
                      <ChevronRight size={12} className="text-surface-500 shrink-0" />
                      {!isLast ? (
                        <button
                          onClick={() => handleBreadcrumbClick(accumulatedPath)}
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
                {selectedPaths.size === (result.entries.length + (result.large_files?.length ?? 0)) ? t("analyze.deselectAll") : t("analyze.selectAll")}
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
                onSelectToggle={() => handleSelectToggle(entry.path)}
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
                      onSelectToggle={() => handleSelectToggle(file.path)}
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

function EntryRow({
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
}

function LargeFileRow({
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
}
