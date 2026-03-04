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

export interface WorkspaceConfig {
  metadata: WorkspaceMetadata;
  editor_state: EditorState;
  tree_state: TreeState;
  workspace_settings: WorkspaceSettings;
  ui_state: UiState;
}

export interface WorkspaceMetadata {
  id: string;
  name: string;
  path: string;
  color?: string;
  created_at: string;
  last_opened: string;
  version: number;
}

export interface EditorState {
  open_files: OpenFileState[];
  active_file_index: number;
  closed_tabs: string[];
}

export interface OpenFileState {
  path: string;
  relative_path: string;
  content_hash: string;
  cursor_position: CursorPosition;
  scroll_position: ScrollPosition;
  is_dirty: boolean;
  last_modified: string;
}

export interface CursorPosition {
  line: number;
  column: number;
}

export interface ScrollPosition {
  line: number;
  column: number;
}

export interface TreeState {
  expanded_folders: string[];
  pinned_items: string[];
  collapsed_workspaces: string[];
}

export interface WorkspaceSettings {
  default_query_template: string;
  auto_save_interval_ms: number;
}

export interface UiState {
  sidebar_width: number;
  chat_width: number;
  panel_height: number;
}

/** Top-level workspace manifest stored at `<workspace>/workspace.json`. */
export interface WorkspaceRoot {
  workspace_version: number;
  created_at: string;
  projects: string[];
}
