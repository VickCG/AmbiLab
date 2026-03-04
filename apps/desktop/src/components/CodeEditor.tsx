import { useState, useEffect } from "react";
import Editor from "@monaco-editor/react";
import { invoke } from "@tauri-apps/api/tauri";
import { VscClose } from "react-icons/vsc";
import VisualizationPanel from "./VisualizationPanel";

interface OpenFile {
  path: string;
  name: string;
  content: string;
  isDirty: boolean;
}

interface Props {
  files: OpenFile[];
  activeIndex: number;
  onTabClick: (index: number) => void;
  onTabClose: (index: number) => void;
  onContentChange: (content: string) => void;
  onSave: (index: number) => void;
}

type FileType = "csv" | "parquet" | "json" | "jsonl" | "sql" | "text" | "unknown";

function getFileType(filename: string): FileType {
  const ext = filename.split(".").pop()?.toLowerCase();
  switch (ext) {
    case "csv":
    case "tsv":
      return "csv";
    case "parquet":
    case "pq":
      return "parquet";
    case "json":
      return "json";
    case "jsonl":
    case "ndjson":
      return "jsonl";
    case "sql":
      return "sql";
    case "txt":
    case "md":
    case "log":
      return "text";
    default:
      return "unknown";
  }
}

const DEFAULT_PANEL_HEIGHT = 200;
const MIN_PANEL_HEIGHT = 100;
const MAX_PANEL_RATIO = 0.6;

function CodeEditor({
  files,
  activeIndex,
  onTabClick,
  onTabClose,
  onContentChange,
  onSave,
}: Props) {
  const activeFile = activeIndex >= 0 ? files[activeIndex] : null;
  const [panelHeight, setPanelHeight] = useState(DEFAULT_PANEL_HEIGHT);
  const [panelCollapsed, setPanelCollapsed] = useState(false);
  const [showPanel, setShowPanel] = useState(false);

  useEffect(() => {
    if (activeFile) {
      const fileType = getFileType(activeFile.name);
      const dataFileTypes: FileType[] = ["csv", "parquet", "json", "jsonl"];
      setShowPanel(dataFileTypes.includes(fileType));
    } else {
      setShowPanel(false);
    }
  }, [activeFile?.path]);

  const getLanguage = (filename: string): string => {
    const ext = filename.split(".").pop()?.toLowerCase();
    const languages: Record<string, string> = {
      ts: "typescript",
      tsx: "typescript",
      js: "javascript",
      jsx: "javascript",
      rs: "rust",
      json: "json",
      css: "css",
      html: "html",
      md: "markdown",
      toml: "toml",
      sql: "sql",
      py: "python",
      go: "go",
      yaml: "yaml",
      yml: "yaml",
    };
    return languages[ext || ""] || "plaintext";
  };

  const handleSave = async () => {
    if (!activeFile || activeIndex < 0) return;

    try {
      await invoke("write_file", {
        path: activeFile.path,
        content: activeFile.content,
      });
      onSave(activeIndex);
    } catch (err) {
      console.error("Failed to save file:", err);
    }
  };

  const handleEditorMount = (editor: any, monaco: any) => {
    editor.addCommand(monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS, handleSave);

    monaco.editor.defineTheme("ambilab-dark", {
      base: "vs-dark",
      inherit: true,
      rules: [],
      colors: {
        "editor.background": "#1e1e1e",
        "editor.foreground": "#cccccc",
        "editorLineNumber.foreground": "#6e6e6e",
        "editorLineNumber.activeForeground": "#cccccc",
        "editor.selectionBackground": "#264f78",
        "editor.lineHighlightBackground": "#2a2d2e",
      },
    });

    monaco.editor.setTheme("ambilab-dark");
  };

  const handleResizeStart = (e: React.MouseEvent) => {
    e.preventDefault();
    const startY = e.clientY;
    const startHeight = panelHeight;
    const container = (e.target as HTMLElement).parentElement;
    const maxHeight = container ? container.clientHeight * MAX_PANEL_RATIO : 400;

    const onMouseMove = (e: MouseEvent) => {
      const delta = startY - e.clientY;
      const newHeight = Math.max(
        MIN_PANEL_HEIGHT,
        Math.min(maxHeight, startHeight + delta)
      );
      setPanelHeight(newHeight);
    };

    const onMouseUp = () => {
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
    };

    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);
  };

  if (files.length === 0) {
    return (
      <div className="editor-area">
        <div className="welcome-screen">
          <div className="welcome-logo">{"</>"}</div>
          <h1 className="welcome-title">AmbiLab IDE</h1>
          <p className="welcome-subtitle">AI-Powered Local Analytics IDE</p>
          <p className="welcome-drop-hint">
            Drop a dataset here to get started
          </p>
          <p className="welcome-formats">CSV · Parquet · JSON · JSONL</p>
        </div>
      </div>
    );
  }

  const showVisualization = showPanel && activeFile;

  return (
    <>
      <div className="editor-tabs">
        {files.map((file, index) => (
          <div
            key={file.path}
            className={`editor-tab ${index === activeIndex ? "active" : ""}`}
            onClick={() => onTabClick(index)}
          >
            <span className="editor-tab-name">{file.name}</span>
            {file.isDirty && <span className="editor-tab-dirty">●</span>}
            <button
              className="editor-tab-close"
              onClick={(e) => {
                e.stopPropagation();
                onTabClose(index);
              }}
            >
              <VscClose />
            </button>
          </div>
        ))}
      </div>

      <div className="editor-split-container">
        <div
          className="editor-content"
          style={{
            flex: showVisualization && !panelCollapsed ? undefined : 1,
            height: showVisualization && !panelCollapsed
              ? `calc(100% - ${panelHeight}px - 4px)`
              : "100%",
          }}
        >
          {activeFile && (
            <Editor
              height="100%"
              language={getLanguage(activeFile.name)}
              value={activeFile.content}
              onChange={(value) => onContentChange(value || "")}
              onMount={handleEditorMount}
              options={{
                fontSize: 14,
                fontFamily: "'Fira Code', 'Consolas', 'Monaco', monospace",
                fontLigatures: true,
                minimap: { enabled: true },
                scrollBeyondLastLine: false,
                renderLineHighlight: "all",
                cursorBlinking: "smooth",
                smoothScrolling: true,
                padding: { top: 10 },
                automaticLayout: true,
              }}
            />
          )}
        </div>

        {showVisualization && (
          <>
            {!panelCollapsed && (
              <div
                className="horizontal-resize-handle"
                onMouseDown={handleResizeStart}
              />
            )}
            <VisualizationPanel
              file={activeFile}
              height={panelHeight}
              collapsed={panelCollapsed}
              onToggleCollapse={() => setPanelCollapsed(!panelCollapsed)}
              onClose={() => setShowPanel(false)}
            />
          </>
        )}
      </div>
    </>
  );
}

export default CodeEditor;
