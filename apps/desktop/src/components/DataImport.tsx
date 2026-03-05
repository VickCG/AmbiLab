import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { open } from "@tauri-apps/api/dialog";
import { homeDir } from "@tauri-apps/api/path";
import {
  VscClose,
  VscFile,
  VscFolder,
  VscFolderOpened,
  VscTrash,
  VscAdd,
} from "react-icons/vsc";
import {
  ImportedFile,
  ImportResult,
  ImportMode,
  FILE_TYPE_LABELS,
  FILE_TYPE_COLORS,
  formatFileSize,
} from "../types/import";
import { Workspace } from "../types/workspace";
import "../styles/DataImport.css";

interface Props {
  isOpen: boolean;
  onClose: () => void;
  onImport: (files: ImportedFile[]) => void;
  workspaces?: Workspace[];
  activeWorkspaceId?: string;
}

const SUPPORTED_EXTENSIONS = [
  "csv", "tsv", "json", "jsonl", "parquet", "arrow", "feather",
  "xlsx", "xls", "sql", "sqlite", "db",
];

function DataImport({
  isOpen,
  onClose,
  onImport,
  workspaces = [],
  activeWorkspaceId,
}: Props) {
  const [mode, setMode] = useState<ImportMode>("files");
  const [selectedFiles, setSelectedFiles] = useState<ImportedFile[]>([]);
  const [recursive, setRecursive] = useState(true);
  const [isLoading, setIsLoading] = useState(false);
  const [selectedWorkspaceId, setSelectedWorkspaceId] = useState<string | undefined>(
    activeWorkspaceId
  );

  useEffect(() => {
    if (activeWorkspaceId) {
      setSelectedWorkspaceId(activeWorkspaceId);
    }
  }, [activeWorkspaceId]);

  if (!isOpen) return null;

  const activeWorkspace = workspaces.find((ws) => ws.id === selectedWorkspaceId);

  const getDefaultPath = async (): Promise<string> => {
    if (activeWorkspace) {
      return activeWorkspace.path;
    }
    return await homeDir();
  };

  const handleSelectFiles = async () => {
    const defaultPath = await getDefaultPath();
    const selected = await open({
      multiple: true,
      filters: [
        {
          name: "Data Files",
          extensions: SUPPORTED_EXTENSIONS,
        },
        {
          name: "All Files",
          extensions: ["*"],
        },
      ],
      defaultPath,
    });

    if (!selected) return;

    const paths = Array.isArray(selected) ? selected : [selected];
    setIsLoading(true);

    try {
      const result = await invoke<ImportResult>("import_files", { paths });
      setSelectedFiles((prev) => {
        const existingPaths = new Set(prev.map((f) => f.path));
        const newFiles = result.files.filter((f) => !existingPaths.has(f.path));
        return [...prev, ...newFiles];
      });
    } catch (err) {
      console.error("Failed to import files:", err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleSelectFolder = async () => {
    const defaultPath = await getDefaultPath();
    const selected = await open({
      directory: true,
      multiple: false,
      defaultPath,
    });

    if (!selected || typeof selected !== "string") return;

    setIsLoading(true);

    try {
      const result = await invoke<ImportResult>("import_folder", {
        path: selected,
        recursive,
      });
      setSelectedFiles((prev) => {
        const existingPaths = new Set(prev.map((f) => f.path));
        const newFiles = result.files.filter((f) => !existingPaths.has(f.path));
        return [...prev, ...newFiles];
      });
    } catch (err) {
      console.error("Failed to import folder:", err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleSelectFolders = async () => {
    const defaultPath = await getDefaultPath();
    const selected = await open({
      directory: true,
      multiple: true,
      defaultPath,
    });

    if (!selected) return;

    const paths = Array.isArray(selected) ? selected : [selected];
    setIsLoading(true);

    try {
      const result = await invoke<ImportResult>("import_folders", {
        paths,
        recursive,
      });
      setSelectedFiles((prev) => {
        const existingPaths = new Set(prev.map((f) => f.path));
        const newFiles = result.files.filter((f) => !existingPaths.has(f.path));
        return [...prev, ...newFiles];
      });
    } catch (err) {
      console.error("Failed to import folders:", err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleScanWorkspace = async () => {
    if (!activeWorkspace) return;

    setIsLoading(true);

    try {
      const result = await invoke<ImportResult>("import_folder", {
        path: activeWorkspace.path,
        recursive,
      });
      setSelectedFiles((prev) => {
        const existingPaths = new Set(prev.map((f) => f.path));
        const newFiles = result.files.filter((f) => !existingPaths.has(f.path));
        return [...prev, ...newFiles];
      });
    } catch (err) {
      console.error("Failed to scan workspace:", err);
    } finally {
      setIsLoading(false);
    }
  };

  const handleRemoveFile = (path: string) => {
    setSelectedFiles((prev) => prev.filter((f) => f.path !== path));
  };

  const handleClearAll = () => {
    setSelectedFiles([]);
  };

  const handleImport = () => {
    if (selectedFiles.length === 0) return;
    onImport(selectedFiles);
    setSelectedFiles([]);
    onClose();
  };

  const handleClose = () => {
    setSelectedFiles([]);
    onClose();
  };

  return (
    <div className="data-import-overlay" onClick={handleClose}>
      <div className="data-import-modal" onClick={(e) => e.stopPropagation()}>
        <div className="data-import-header">
          <h2>Import Data Files</h2>
          <button className="close-btn" onClick={handleClose}>
            <VscClose />
          </button>
        </div>

        {workspaces.length > 0 && (
          <div className="workspace-selector">
            <label>Workspace:</label>
            <select
              value={selectedWorkspaceId || ""}
              onChange={(e) => setSelectedWorkspaceId(e.target.value || undefined)}
            >
              <option value="">Browse from Home</option>
              {workspaces.map((ws) => (
                <option key={ws.id} value={ws.id}>
                  {ws.name}
                </option>
              ))}
            </select>
            {activeWorkspace && (
              <button
                className="scan-workspace-btn"
                onClick={handleScanWorkspace}
                disabled={isLoading}
              >
                Scan Workspace
              </button>
            )}
          </div>
        )}

        <div className="data-import-tabs">
          <button
            className={`tab ${mode === "files" ? "active" : ""}`}
            onClick={() => setMode("files")}
          >
            <VscFile /> Files
          </button>
          <button
            className={`tab ${mode === "folder" ? "active" : ""}`}
            onClick={() => setMode("folder")}
          >
            <VscFolder /> Folder
          </button>
          <button
            className={`tab ${mode === "folders" ? "active" : ""}`}
            onClick={() => setMode("folders")}
          >
            <VscFolderOpened /> Multiple Folders
          </button>
        </div>

        <div className="data-import-content">
          <div className="import-actions">
            {mode === "files" && (
              <button className="select-btn" onClick={handleSelectFiles} disabled={isLoading}>
                <VscAdd /> Select Files
              </button>
            )}
            {mode === "folder" && (
              <>
                <button className="select-btn" onClick={handleSelectFolder} disabled={isLoading}>
                  <VscAdd /> Select Folder
                </button>
                <label className="recursive-checkbox">
                  <input
                    type="checkbox"
                    checked={recursive}
                    onChange={(e) => setRecursive(e.target.checked)}
                  />
                  Include subfolders
                </label>
              </>
            )}
            {mode === "folders" && (
              <>
                <button className="select-btn" onClick={handleSelectFolders} disabled={isLoading}>
                  <VscAdd /> Select Folders
                </button>
                <label className="recursive-checkbox">
                  <input
                    type="checkbox"
                    checked={recursive}
                    onChange={(e) => setRecursive(e.target.checked)}
                  />
                  Include subfolders
                </label>
              </>
            )}
          </div>

          <div className="supported-formats">
            <span className="label">Supported:</span>
            {["csv", "tsv", "json", "parquet", "arrow", "excel", "sql"].map((type) => (
              <span
                key={type}
                className="format-badge"
                style={{ backgroundColor: FILE_TYPE_COLORS[type as keyof typeof FILE_TYPE_COLORS] }}
              >
                {type.toUpperCase()}
              </span>
            ))}
          </div>

          <div className="selected-files-header">
            <span>Selected Files ({selectedFiles.length})</span>
            {selectedFiles.length > 0 && (
              <button className="clear-btn" onClick={handleClearAll}>
                <VscTrash /> Clear All
              </button>
            )}
          </div>

          <div className="selected-files-list">
            {isLoading ? (
              <div className="loading">Scanning files...</div>
            ) : selectedFiles.length === 0 ? (
              <div className="empty-state">
                No files selected. Click the button above to add data files.
              </div>
            ) : (
              selectedFiles.map((file) => (
                <div key={file.path} className="file-item">
                  <div
                    className="file-type-indicator"
                    style={{ backgroundColor: FILE_TYPE_COLORS[file.file_type] }}
                  />
                  <div className="file-info">
                    <span className="file-name">{file.name}</span>
                    <span className="file-meta">
                      {FILE_TYPE_LABELS[file.file_type]} &middot; {formatFileSize(file.size_bytes)}
                    </span>
                  </div>
                  <button
                    className="remove-btn"
                    onClick={() => handleRemoveFile(file.path)}
                    title="Remove"
                  >
                    <VscClose />
                  </button>
                </div>
              ))
            )}
          </div>
        </div>

        <div className="data-import-footer">
          <button className="cancel-btn" onClick={handleClose}>
            Cancel
          </button>
          <button
            className="import-btn"
            onClick={handleImport}
            disabled={selectedFiles.length === 0 || isLoading}
          >
            Import {selectedFiles.length > 0 ? `(${selectedFiles.length})` : ""}
          </button>
        </div>
      </div>
    </div>
  );
}

export default DataImport;
