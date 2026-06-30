export interface OptimizeItem {
  action: string;
  name: string;
  description: string;
  safe: boolean;
  requires_sudo: boolean;
  enabled: boolean;
  status?: "pending" | "applied" | "skipped" | "failed";
}

export interface SystemHealth {
  memory_used_gb: number;
  memory_total_gb: number;
  disk_used_gb: number;
  disk_total_gb: number;
  uptime_days: number;
}

export interface OptimizeResult {
  system_health: SystemHealth | null;
  optimizations: OptimizeItem[];
  total_items: number;
  applied_count: number;
}
