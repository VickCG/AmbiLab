export interface Workspace {
  id: string;
  name: string;
  path: string;
  color?: string;
}

export interface WorkspaceInfo {
  workspace: Workspace;
  file_count: number;
  folder_count: number;
}

export const WORKSPACE_COLORS = [
  "#4caf50", // Green
  "#2196f3", // Blue
  "#ff9800", // Orange
  "#9c27b0", // Purple
  "#f44336", // Red
  "#00bcd4", // Cyan
  "#e91e63", // Pink
  "#ffeb3b", // Yellow
];

export function getWorkspaceColor(index: number): string {
  return WORKSPACE_COLORS[index % WORKSPACE_COLORS.length];
}
