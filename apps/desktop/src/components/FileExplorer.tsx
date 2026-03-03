import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { open } from "@tauri-apps/api/dialog";
import { homeDir } from "@tauri-apps/api/path";
import {
  VscFolder,
  VscFolderOpened,
  VscFile,
  VscRefresh,
  VscNewFolder,
  VscChevronRight,
  VscChevronDown,
  VscCloudDownload,
  VscClose,
} from "react-icons/vsc";
import { Workspace, getWorkspaceColor } from "../types/workspace";

interface FileEntry {
  name: string;
  path: string;
  is_dir: boolean;
}

interface TreeNode extends FileEntry {
  children?: TreeNode[];
  isExpanded?: boolean;
  isLoading?: boolean;
}

interface WorkspaceTree {
  workspace: Workspace;
  tree: TreeNode[];
  isExpanded: boolean;
}

interface Props {
  workspaces: Workspace[];
  onFileOpen: (path: string, name: string, content: string) => void;
  onImportClick: (workspaceId?: string) => void;
  onAddWorkspace: (workspace: Workspace) => void;
  onRemoveWorkspace: (workspaceId: string) => void;
}

function FileExplorer({
  workspaces,
  onFileOpen,
  onImportClick,
  onAddWorkspace,
  onRemoveWorkspace,
}: Props) {
  const [workspaceTrees, setWorkspaceTrees] = useState<WorkspaceTree[]>([]);
  const [contextMenu, setContextMenu] = useState<{
    x: number;
    y: number;
    workspaceId: string;
  } | null>(null);

  useEffect(() => {
    const loadWorkspaces = async () => {
      const trees: WorkspaceTree[] = [];
      for (const ws of workspaces) {
        const existing = workspaceTrees.find((wt) => wt.workspace.id === ws.id);
        if (existing) {
          trees.push({ ...existing, workspace: ws });
        } else {
          const tree = await loadDirectory(ws.path);
          trees.push({ workspace: ws, tree, isExpanded: true });
        }
      }
      setWorkspaceTrees(trees);
    };
    loadWorkspaces();
  }, [workspaces]);

  const loadDirectory = async (path: string): Promise<TreeNode[]> => {
    try {
      const entries = await invoke<FileEntry[]>("read_dir", { path });
      return entries.map((entry) => ({
        ...entry,
        isExpanded: false,
        children: entry.is_dir ? undefined : undefined,
      }));
    } catch (err) {
      console.error("Failed to read directory:", err);
      return [];
    }
  };

  const handleAddWorkspace = async () => {
    const home = await homeDir();
    const selected = await open({
      directory: true,
      multiple: true,
      defaultPath: home,
    });

    if (!selected) return;

    const paths = Array.isArray(selected) ? selected : [selected];

    for (const path of paths) {
      try {
        const workspace = await invoke<Workspace>("create_workspace", { path });
        const colorIndex = workspaces.length + paths.indexOf(path);
        onAddWorkspace({ ...workspace, color: getWorkspaceColor(colorIndex) });
      } catch (err) {
        console.error("Failed to create workspace:", err);
      }
    }
  };

  const handleRefreshWorkspace = async (workspaceId: string) => {
    const wsTree = workspaceTrees.find((wt) => wt.workspace.id === workspaceId);
    if (!wsTree) return;

    const tree = await loadDirectory(wsTree.workspace.path);
    setWorkspaceTrees((prev) =>
      prev.map((wt) =>
        wt.workspace.id === workspaceId ? { ...wt, tree } : wt
      )
    );
  };

  const handleToggleWorkspace = (workspaceId: string) => {
    setWorkspaceTrees((prev) =>
      prev.map((wt) =>
        wt.workspace.id === workspaceId
          ? { ...wt, isExpanded: !wt.isExpanded }
          : wt
      )
    );
  };

  const handleToggleFolder = async (
    workspaceId: string,
    node: TreeNode,
    path: number[]
  ) => {
    const updateTree = (
      nodes: TreeNode[],
      path: number[],
      depth: number
    ): TreeNode[] => {
      return nodes.map((n, i) => {
        if (i === path[depth]) {
          if (depth === path.length - 1) {
            return { ...n, isExpanded: !n.isExpanded };
          }
          if (n.children) {
            return {
              ...n,
              children: updateTree(n.children, path, depth + 1),
            };
          }
        }
        return n;
      });
    };

    const wsTree = workspaceTrees.find((wt) => wt.workspace.id === workspaceId);
    if (!wsTree) return;

    if (!node.isExpanded && node.is_dir && !node.children) {
      const children = await loadDirectory(node.path);
      const updateWithChildren = (
        nodes: TreeNode[],
        path: number[],
        depth: number
      ): TreeNode[] => {
        return nodes.map((n, i) => {
          if (i === path[depth]) {
            if (depth === path.length - 1) {
              return { ...n, isExpanded: true, children };
            }
            if (n.children) {
              return {
                ...n,
                children: updateWithChildren(n.children, path, depth + 1),
              };
            }
          }
          return n;
        });
      };
      setWorkspaceTrees((prev) =>
        prev.map((wt) =>
          wt.workspace.id === workspaceId
            ? { ...wt, tree: updateWithChildren(wt.tree, path, 0) }
            : wt
        )
      );
    } else {
      setWorkspaceTrees((prev) =>
        prev.map((wt) =>
          wt.workspace.id === workspaceId
            ? { ...wt, tree: updateTree(wt.tree, path, 0) }
            : wt
        )
      );
    }
  };

  const handleFileClick = async (node: TreeNode) => {
    if (node.is_dir) return;

    try {
      const content = await invoke<string>("read_file", { path: node.path });
      onFileOpen(node.path, node.name, content);
    } catch (err) {
      console.error("Failed to read file:", err);
    }
  };

  const handleContextMenu = (e: React.MouseEvent, workspaceId: string) => {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY, workspaceId });
  };

  const closeContextMenu = () => setContextMenu(null);

  const getFileIcon = (name: string) => {
    const ext = name.split(".").pop()?.toLowerCase();
    const iconColor =
      {
        ts: "#3178c6",
        tsx: "#3178c6",
        js: "#f7df1e",
        jsx: "#f7df1e",
        rs: "#dea584",
        json: "#ff9800",
        jsonl: "#ff9800",
        css: "#563d7c",
        html: "#e34c26",
        md: "#083fa1",
        toml: "#9c4221",
        sql: "#e38c00",
        csv: "#4caf50",
        tsv: "#4caf50",
        parquet: "#2196f3",
        arrow: "#9c27b0",
        feather: "#9c27b0",
        xlsx: "#1d6f42",
        xls: "#1d6f42",
        sqlite: "#003b57",
        db: "#003b57",
      }[ext || ""] || "#969696";

    return <VscFile style={{ color: iconColor }} />;
  };

  const renderTree = (
    workspaceId: string,
    nodes: TreeNode[],
    path: number[] = []
  ) => {
    return nodes.map((node, index) => {
      const currentPath = [...path, index];

      return (
        <div key={node.path}>
          <div
            className="tree-item"
            style={{ paddingLeft: `${16 + path.length * 16}px` }}
            onClick={() =>
              node.is_dir
                ? handleToggleFolder(workspaceId, node, currentPath)
                : handleFileClick(node)
            }
          >
            {node.is_dir && (
              <span className="tree-item-icon">
                {node.isExpanded ? <VscChevronDown /> : <VscChevronRight />}
              </span>
            )}
            <span className="tree-item-icon">
              {node.is_dir ? (
                node.isExpanded ? (
                  <VscFolderOpened style={{ color: "#dcb67a" }} />
                ) : (
                  <VscFolder style={{ color: "#dcb67a" }} />
                )
              ) : (
                getFileIcon(node.name)
              )}
            </span>
            <span className="tree-item-name">{node.name}</span>
          </div>
          {node.is_dir && node.isExpanded && node.children && (
            <div className="tree-children">
              {renderTree(workspaceId, node.children, currentPath)}
            </div>
          )}
        </div>
      );
    });
  };

  const renderWorkspace = (wsTree: WorkspaceTree, index: number) => {
    const { workspace, tree, isExpanded } = wsTree;
    const color = workspace.color || getWorkspaceColor(index);

    return (
      <div key={workspace.id} className="workspace-section">
        <div
          className="workspace-header"
          onClick={() => handleToggleWorkspace(workspace.id)}
          onContextMenu={(e) => handleContextMenu(e, workspace.id)}
        >
          <div className="workspace-header-left">
            <span className="tree-item-icon">
              {isExpanded ? <VscChevronDown /> : <VscChevronRight />}
            </span>
            <span
              className="workspace-color-indicator"
              style={{ backgroundColor: color }}
            />
            <span className="workspace-name">{workspace.name}</span>
          </div>
          <div className="workspace-header-actions">
            <button
              className="icon-btn-small"
              onClick={(e) => {
                e.stopPropagation();
                onImportClick(workspace.id);
              }}
              title="Import Data"
            >
              <VscCloudDownload />
            </button>
            <button
              className="icon-btn-small"
              onClick={(e) => {
                e.stopPropagation();
                handleRefreshWorkspace(workspace.id);
              }}
              title="Refresh"
            >
              <VscRefresh />
            </button>
            <button
              className="icon-btn-small"
              onClick={(e) => {
                e.stopPropagation();
                onRemoveWorkspace(workspace.id);
              }}
              title="Remove Workspace"
            >
              <VscClose />
            </button>
          </div>
        </div>
        {isExpanded && (
          <div className="workspace-tree">
            {tree.length === 0 ? (
              <div className="workspace-empty">No files</div>
            ) : (
              renderTree(workspace.id, tree)
            )}
          </div>
        )}
      </div>
    );
  };

  return (
    <div className="file-explorer" onClick={closeContextMenu}>
      <div className="file-explorer-header">
        <span>EXPLORER</span>
        <div className="file-explorer-actions">
          <button
            className="icon-btn"
            onClick={() => onImportClick()}
            title="Import Data"
          >
            <VscCloudDownload />
          </button>
          <button
            className="icon-btn"
            onClick={handleAddWorkspace}
            title="Add Workspace Folder"
          >
            <VscNewFolder />
          </button>
        </div>
      </div>

      <div className="file-tree">
        {workspaceTrees.length === 0 ? (
          <div className="empty-state">
            <p>No workspace opened</p>
            <button className="primary-btn" onClick={handleAddWorkspace}>
              Add Folder to Workspace
            </button>
          </div>
        ) : (
          workspaceTrees.map((wsTree, index) => renderWorkspace(wsTree, index))
        )}
      </div>

      {contextMenu && (
        <div
          className="context-menu"
          style={{ top: contextMenu.y, left: contextMenu.x }}
        >
          <div
            className="context-menu-item"
            onClick={() => {
              onImportClick(contextMenu.workspaceId);
              closeContextMenu();
            }}
          >
            Import Data Files
          </div>
          <div
            className="context-menu-item"
            onClick={() => {
              handleRefreshWorkspace(contextMenu.workspaceId);
              closeContextMenu();
            }}
          >
            Refresh
          </div>
          <div className="context-menu-divider" />
          <div
            className="context-menu-item danger"
            onClick={() => {
              onRemoveWorkspace(contextMenu.workspaceId);
              closeContextMenu();
            }}
          >
            Remove from Workspace
          </div>
        </div>
      )}
    </div>
  );
}

export default FileExplorer;
