export interface AppInfo {
  name: string;
  path: string;
  bundle_id: string;
  size_kb: number;
  is_running: boolean;
  has_brew_cask: boolean;
  is_blocked: boolean;
  selected?: boolean;
}

export interface UninstallScanResult {
  apps: AppInfo[];
  total_apps: number;
  total_size_kb: number;
}

export interface UninstallFileItem {
  path: string;
  size_kb: number;
}

export interface UninstallPreviewApp {
  app_name: string;
  app_path: string;
  user_files: UninstallFileItem[];
  system_files: UninstallFileItem[];
  review_only_files: UninstallFileItem[];
  total_size_kb: number;
}

export interface UninstallPreviewResult {
  targets: string[];
  removal_plan: UninstallPreviewApp[];
  total_size_kb: number;
  requires_sudo: boolean;
}
