import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import type { CommandStatus, MoleEvent, ProgressEvent } from "@/types/common";

interface UseMoleCommandOptions {
  command: string;
  onEvent?: (event: MoleEvent) => void;
}

interface UseMoleCommandResult<T> {
  data: T | null;
  status: CommandStatus;
  progress: ProgressEvent[];
  error: string | null;
  execute: (args?: Record<string, unknown>) => Promise<T | null>;
  cancel: () => void;
  reset: () => void;
}

export function useMoleCommand<T = unknown>({
  command,
  onEvent,
}: UseMoleCommandOptions): UseMoleCommandResult<T> {
  const [data, setData] = useState<T | null>(null);
  const [status, setStatus] = useState<CommandStatus>("idle");
  const [progress, setProgress] = useState<ProgressEvent[]>([]);
  const [error, setError] = useState<string | null>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  useEffect(() => {
    const eventName = `mole-${command}-event`;
    let mounted = true;

    const setup = async () => {
      const unlisten = await listen<MoleEvent>(eventName, (event) => {
        if (!mounted) return;
        const moleEvent = event.payload;
        onEvent?.(moleEvent);

        if (moleEvent.type === "progress") {
          setProgress((prev) => [...prev, moleEvent]);
        } else if (moleEvent.type === "error") {
          setError(moleEvent.message);
          setStatus("error");
        }
      });
      unlistenRef.current = unlisten;
    };

    setup();
    return () => {
      mounted = false;
      unlistenRef.current?.();
    };
  }, [command, onEvent]);

  const execute = useCallback(
    async (args?: Record<string, unknown>) => {
      setStatus("scanning");
      setProgress([]);
      setError(null);
      setData(null);

      try {
        const result = await invoke<T>(command, args);
        setData(result);
        setStatus("preview");
        return result;
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        setError(msg);
        setStatus("error");
        return null;
      }
    },
    [command]
  );

  const cancel = useCallback(async () => {
    try {
      await invoke(`cancel_${command}`);
    } catch {
      // ignore
    }
    setStatus("idle");
  }, [command]);

  const reset = useCallback(() => {
    setData(null);
    setStatus("idle");
    setProgress([]);
    setError(null);
  }, []);

  return { data, status, progress, error, execute, cancel, reset };
}
