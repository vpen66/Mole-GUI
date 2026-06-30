export interface PurgeArtifact {
  name: string;
  path: string;
  size_kb: number;
  size_human: string;
}

export interface PurgeProject {
  name: string;
  path: string;
  total_size_kb: number;
  artifacts: PurgeArtifact[];
  selected?: boolean;
}

export interface PurgeResult {
  projects: PurgeProject[];
  total_size_kb: number;
  total_projects: number;
}
