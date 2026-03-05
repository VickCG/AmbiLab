export type ProjectStatus = 'active' | 'stable' | 'archived';

export interface DatasetRef {
  name: string;
  /** Relative to project folder, e.g. `datasets/sales.csv`,
   *  or `../../datasets/global.csv` for workspace-shared datasets. */
  path: string;
}

export interface ProjectConfig {
  id: string;
  name: string;
  created_at: string;
  datasets: DatasetRef[];
  status: ProjectStatus;
  version: number;
}

/** Stored as `charts/<name>.chart.json`. References a sibling query file. */
export interface ChartDefinition {
  query: string;
  type: 'bar' | 'line' | 'scatter' | 'pie';
  x: string;
  y: string;
}

/** Persisted in `<workspace>/settings/session.json`. */
export interface SessionConfig {
  last_project: string | null;
  open_files: string[];
  active_dataset: string | null;
}
