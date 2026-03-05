import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import FileExplorer from "./components/FileExplorer";
import CodeEditor from "./components/CodeEditor";
import AIChat from "./components/AIChat";
import DataImport from "./components/DataImport";
import { ImportedFile } from "./types/import";
import { Workspace, WorkspaceConfig } from "./types/workspace";

interface OpenFile {
  path: string;
  name: string;
  content: string;
  isDirty: boolean;
}

function App() {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [openFiles, setOpenFiles] = useState<OpenFile[]>([]);
  const [activeFileIndex, setActiveFileIndex] = useState<number>(-1);
  const [sidebarWidth, setSidebarWidth] = useState(280);
  const [chatWidth, setChatWidth] = useState(350);
  const [showImportModal, setShowImportModal] = useState(false);
  const [importWorkspaceId, setImportWorkspaceId] = useState<string | undefined>();
  const [isDragging, setIsDragging] = useState(false);
  const dragCounterRef = useRef(0);

  const activeFile = activeFileIndex >= 0 ? openFiles[activeFileIndex] : null;
  const saveTimeoutRef = useRef<number | null>(null);

  useEffect(() => {
    const createConfigsForNewWorkspaces = async () => {
      for (const workspace of workspaces) {
        try {
          await invoke("create_workspace_with_config", {
            path: workspace.path,
            name: workspace.name,
            color: workspace.color,
          });
        } catch (err) {
          console.warn(`Could not create config for ${workspace.name}:`, err);
        }
      }
    };

    if (workspaces.length > 0) {
      createConfigsForNewWorkspaces();
    }
  }, [workspaces]);

  useEffect(() => {
    if (saveTimeoutRef.current !== null) {
      clearTimeout(saveTimeoutRef.current);
    }

    saveTimeoutRef.current = window.setTimeout(() => {
      const saveWorkspaceConfigs = async () => {
        for (const workspace of workspaces) {
          try {
            const config: WorkspaceConfig = {
              metadata: {
                id: workspace.id,
                name: workspace.name,
                path: workspace.path,
                color: workspace.color,
                created_at: new Date().toISOString(),
                last_opened: new Date().toISOString(),
                version: 1,
              },
              editor_state: {
                open_files: openFiles.map((file) => ({
                  path: file.path,
                  relative_path: file.path.replace(workspace.path + "/", ""),
                  content_hash: "",
                  cursor_position: { line: 0, column: 0 },
                  scroll_position: { line: 0, column: 0 },
                  is_dirty: file.isDirty,
                  last_modified: new Date().toISOString(),
                })),
                active_file_index: activeFileIndex,
                closed_tabs: [],
              },
              tree_state: {
                expanded_folders: [],
                pinned_items: [],
                collapsed_workspaces: [],
              },
              workspace_settings: {
                default_query_template: "sql",
                auto_save_interval_ms: 2000,
              },
              ui_state: {
                sidebar_width: sidebarWidth,
                chat_width: chatWidth,
                panel_height: 200,
              },
            };

            await invoke("update_workspace_config", {
              path: workspace.path,
              config,
            });
          } catch (err) {
            console.error(`Failed to save config for ${workspace.name}:`, err);
          }
        }
      };

      saveWorkspaceConfigs();
    }, 1000);

    return () => {
      if (saveTimeoutRef.current !== null) {
        clearTimeout(saveTimeoutRef.current);
      }
    };
  }, [workspaces, openFiles, activeFileIndex, sidebarWidth, chatWidth]);

  const handleAddWorkspace = (workspace: Workspace) => {
    setWorkspaces((prev) => {
      if (prev.some((ws) => ws.path === workspace.path)) {
        return prev;
      }
      return [...prev, workspace];
    });
  };

  const handleRemoveWorkspace = (workspaceId: string) => {
    setWorkspaces((prev) => prev.filter((ws) => ws.id !== workspaceId));
  };

  const handleFileOpen = (path: string, name: string, content: string) => {
    const existingIndex = openFiles.findIndex((f) => f.path === path);
    if (existingIndex >= 0) {
      setActiveFileIndex(existingIndex);
      return;
    }

    const newFile: OpenFile = { path, name, content, isDirty: false };
    setOpenFiles([...openFiles, newFile]);
    setActiveFileIndex(openFiles.length);
  };

  const handleFileClose = (index: number) => {
    const newFiles = openFiles.filter((_, i) => i !== index);
    setOpenFiles(newFiles);

    if (activeFileIndex === index) {
      setActiveFileIndex(newFiles.length > 0 ? Math.max(0, index - 1) : -1);
    } else if (activeFileIndex > index) {
      setActiveFileIndex(activeFileIndex - 1);
    }
  };

  const handleContentChange = (content: string) => {
    if (activeFileIndex < 0) return;

    const newFiles = [...openFiles];
    newFiles[activeFileIndex] = {
      ...newFiles[activeFileIndex],
      content,
      isDirty: true,
    };
    setOpenFiles(newFiles);
  };

  const handleFileSave = (index: number) => {
    const newFiles = [...openFiles];
    newFiles[index] = { ...newFiles[index], isDirty: false };
    setOpenFiles(newFiles);
  };

  const handleImportClick = (workspaceId?: string) => {
    setImportWorkspaceId(workspaceId);
    setShowImportModal(true);
  };

  const handleImportFiles = async (importedFiles: ImportedFile[]) => {
    for (const file of importedFiles) {
      try {
        const content = await invoke<string>("read_file", { path: file.path });
        handleFileOpen(file.path, file.name, content);
      } catch (err) {
        console.error(`Failed to read imported file: ${file.path}`, err);
      }
    }
  };

  const DATA_EXTS = new Set(["csv", "tsv", "parquet", "pq", "json", "jsonl", "ndjson"]);

  const handleDragEnter = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    dragCounterRef.current++;
    if (e.dataTransfer.items.length > 0) setIsDragging(true);
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    dragCounterRef.current--;
    if (dragCounterRef.current === 0) setIsDragging(false);
  }, []);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
  }, []);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    dragCounterRef.current = 0;
    setIsDragging(false);
    Array.from(e.dataTransfer.files).forEach((file) => {
      const ext = file.name.split(".").pop()?.toLowerCase() ?? "";
      if (!DATA_EXTS.has(ext)) return;
      const path = (file as any).path as string | undefined;
      if (!path) return;
      handleFileOpen(path, file.name, "");
    });
  }, [handleFileOpen]);

  return (
    <div
      className="app"
      onDragEnter={handleDragEnter}
      onDragLeave={handleDragLeave}
      onDragOver={handleDragOver}
      onDrop={handleDrop}
    >
      {isDragging && (
        <div className="drop-overlay">
          <div className="drop-overlay-content">
            <div className="drop-overlay-icon">⬇</div>
            <p className="drop-overlay-title">Drop dataset to open</p>
            <p className="drop-overlay-formats">CSV · Parquet · JSON · JSONL</p>
          </div>
        </div>
      )}
      <div className="sidebar" style={{ width: sidebarWidth }}>
        <FileExplorer
          workspaces={workspaces}
          onFileOpen={handleFileOpen}
          onImportClick={handleImportClick}
          onAddWorkspace={handleAddWorkspace}
          onRemoveWorkspace={handleRemoveWorkspace}
        />
      </div>

      <div
        className="resize-handle"
        onMouseDown={(e) => {
          const startX = e.clientX;
          const startWidth = sidebarWidth;

          const onMouseMove = (e: MouseEvent) => {
            const newWidth = startWidth + (e.clientX - startX);
            setSidebarWidth(Math.max(200, Math.min(500, newWidth)));
          };

          const onMouseUp = () => {
            document.removeEventListener("mousemove", onMouseMove);
            document.removeEventListener("mouseup", onMouseUp);
          };

          document.addEventListener("mousemove", onMouseMove);
          document.addEventListener("mouseup", onMouseUp);
        }}
      />

      <div className="editor-area">
        <CodeEditor
          files={openFiles}
          activeIndex={activeFileIndex}
          onTabClick={setActiveFileIndex}
          onTabClose={handleFileClose}
          onContentChange={handleContentChange}
          onSave={handleFileSave}
        />
      </div>

      <div
        className="resize-handle"
        onMouseDown={(e) => {
          const startX = e.clientX;
          const startWidth = chatWidth;

          const onMouseMove = (e: MouseEvent) => {
            const newWidth = startWidth - (e.clientX - startX);
            setChatWidth(Math.max(250, Math.min(600, newWidth)));
          };

          const onMouseUp = () => {
            document.removeEventListener("mousemove", onMouseMove);
            document.removeEventListener("mouseup", onMouseUp);
          };

          document.addEventListener("mousemove", onMouseMove);
          document.addEventListener("mouseup", onMouseUp);
        }}
      />

      <div className="chat-panel" style={{ width: chatWidth }}>
        <AIChat currentFile={activeFile} />
      </div>

      <DataImport
        isOpen={showImportModal}
        onClose={() => {
          setShowImportModal(false);
          setImportWorkspaceId(undefined);
        }}
        onImport={handleImportFiles}
        workspaces={workspaces}
        activeWorkspaceId={importWorkspaceId}
      />
    </div>
  );
}

export default App;
