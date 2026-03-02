import Editor from "@monaco-editor/react";
import { invoke } from "@tauri-apps/api/tauri";
import { VscClose } from "react-icons/vsc";

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

function CodeEditor({
  files,
  activeIndex,
  onTabClick,
  onTabClose,
  onContentChange,
  onSave,
}: Props) {
  const activeFile = activeIndex >= 0 ? files[activeIndex] : null;

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

  if (files.length === 0) {
    return (
      <div className="editor-area">
        <div className="welcome-screen">
          <div className="welcome-logo">{"</>"}</div>
          <h1 className="welcome-title">AmbiLab IDE</h1>
          <p className="welcome-subtitle">
            AI-Powered Local Analytics IDE
          </p>
          <p style={{ color: "var(--text-muted)", fontSize: "12px" }}>
            Open a folder to get started
          </p>
        </div>
      </div>
    );
  }

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

      <div className="editor-content">
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
    </>
  );
}

export default CodeEditor;
