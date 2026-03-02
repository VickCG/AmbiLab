import { useState } from "react";
import FileExplorer from "./components/FileExplorer";
import CodeEditor from "./components/CodeEditor";
import AIChat from "./components/AIChat";

interface OpenFile {
  path: string;
  name: string;
  content: string;
  isDirty: boolean;
}

function App() {
  const [openFiles, setOpenFiles] = useState<OpenFile[]>([]);
  const [activeFileIndex, setActiveFileIndex] = useState<number>(-1);
  const [sidebarWidth, setSidebarWidth] = useState(250);
  const [chatWidth, setChatWidth] = useState(350);

  const activeFile = activeFileIndex >= 0 ? openFiles[activeFileIndex] : null;

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

  return (
    <div className="app">
      <div className="sidebar" style={{ width: sidebarWidth }}>
        <FileExplorer onFileOpen={handleFileOpen} />
      </div>

      <div
        className="resize-handle"
        onMouseDown={(e) => {
          const startX = e.clientX;
          const startWidth = sidebarWidth;

          const onMouseMove = (e: MouseEvent) => {
            const newWidth = startWidth + (e.clientX - startX);
            setSidebarWidth(Math.max(150, Math.min(500, newWidth)));
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
    </div>
  );
}

export default App;
