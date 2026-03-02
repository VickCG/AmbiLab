import { useState } from "react";
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
} from "react-icons/vsc";

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

interface Props {
  onFileOpen: (path: string, name: string, content: string) => void;
}

function FileExplorer({ onFileOpen }: Props) {
  const [rootPath, setRootPath] = useState<string>("");
  const [tree, setTree] = useState<TreeNode[]>([]);

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

  const handleOpenFolder = async () => {
    const home = await homeDir();
    const selected = await open({
      directory: true,
      multiple: false,
      defaultPath: home,
    });

    if (selected && typeof selected === "string") {
      setRootPath(selected);
      const entries = await loadDirectory(selected);
      setTree(entries);
    }
  };

  const handleRefresh = async () => {
    if (rootPath) {
      const entries = await loadDirectory(rootPath);
      setTree(entries);
    }
  };

  const handleToggleFolder = async (node: TreeNode, path: number[]) => {
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
      setTree(updateWithChildren(tree, path, 0));
    } else {
      setTree(updateTree(tree, path, 0));
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

  const getFileIcon = (name: string) => {
    const ext = name.split(".").pop()?.toLowerCase();
    const iconColor =
      {
        ts: "#3178c6",
        tsx: "#3178c6",
        js: "#f7df1e",
        jsx: "#f7df1e",
        rs: "#dea584",
        json: "#cbcb41",
        css: "#563d7c",
        html: "#e34c26",
        md: "#083fa1",
        toml: "#9c4221",
        sql: "#e38c00",
      }[ext || ""] || "#969696";

    return <VscFile style={{ color: iconColor }} />;
  };

  const renderTree = (nodes: TreeNode[], path: number[] = []) => {
    return nodes.map((node, index) => {
      const currentPath = [...path, index];

      return (
        <div key={node.path}>
          <div
            className="tree-item"
            style={{ paddingLeft: `${8 + path.length * 16}px` }}
            onClick={() =>
              node.is_dir
                ? handleToggleFolder(node, currentPath)
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
              {renderTree(node.children, currentPath)}
            </div>
          )}
        </div>
      );
    });
  };

  return (
    <div className="file-explorer">
      <div className="file-explorer-header">
        <span>Explorer</span>
        <div className="file-explorer-actions">
          <button className="icon-btn" onClick={handleOpenFolder} title="Open Folder">
            <VscNewFolder />
          </button>
          <button className="icon-btn" onClick={handleRefresh} title="Refresh">
            <VscRefresh />
          </button>
        </div>
      </div>

      <div className="file-tree">
        {tree.length === 0 ? (
          <div
            style={{
              padding: "20px",
              textAlign: "center",
              color: "var(--text-muted)",
            }}
          >
            <p style={{ marginBottom: "12px" }}>No folder opened</p>
            <button
              onClick={handleOpenFolder}
              style={{
                background: "var(--accent-color)",
                border: "none",
                padding: "8px 16px",
                borderRadius: "4px",
                color: "white",
                cursor: "pointer",
              }}
            >
              Open Folder
            </button>
          </div>
        ) : (
          renderTree(tree)
        )}
      </div>
    </div>
  );
}

export default FileExplorer;
