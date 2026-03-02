import { VscClose, VscChevronDown, VscChevronUp } from "react-icons/vsc";
import DataTableView from "./DataTableView";
import SqlAnalysisView from "./SqlAnalysisView";

interface OpenFile {
  path: string;
  name: string;
  content: string;
  isDirty: boolean;
}

interface Props {
  file: OpenFile | null;
  height: number;
  collapsed: boolean;
  onToggleCollapse: () => void;
  onClose: () => void;
}

type FileType = "csv" | "parquet" | "sql" | "unknown";

function getFileType(filename: string): FileType {
  const ext = filename.split(".").pop()?.toLowerCase();

  switch (ext) {
    case "csv":
      return "csv";
    case "parquet":
    case "pq":
      return "parquet";
    case "sql":
      return "sql";
    default:
      return "unknown";
  }
}

function getViewTitle(fileType: FileType): string {
  switch (fileType) {
    case "csv":
      return "CSV Data Preview";
    case "parquet":
      return "Parquet Data Preview";
    case "sql":
      return "SQL Analysis";
    default:
      return "Preview";
  }
}

function VisualizationPanel({
  file,
  height,
  collapsed,
  onToggleCollapse,
  onClose,
}: Props) {
  if (!file) return null;

  const fileType = getFileType(file.name);

  if (fileType === "unknown") return null;

  const renderContent = () => {
    if (collapsed) return null;

    switch (fileType) {
      case "csv":
        return <DataTableView filePath={file.path} fileType="csv" />;
      case "parquet":
        return <DataTableView filePath={file.path} fileType="parquet" />;
      case "sql":
        return <SqlAnalysisView content={file.content} />;
      default:
        return null;
    }
  };

  return (
    <div
      className="visualization-panel"
      style={{ height: collapsed ? "auto" : height }}
    >
      <div className="visualization-header">
        <button
          className="visualization-toggle"
          onClick={onToggleCollapse}
          title={collapsed ? "Expand" : "Collapse"}
        >
          {collapsed ? <VscChevronUp /> : <VscChevronDown />}
        </button>
        <span className="visualization-title">{getViewTitle(fileType)}</span>
        <span className="visualization-filename">{file.name}</span>
        <button
          className="visualization-close"
          onClick={onClose}
          title="Close panel"
        >
          <VscClose />
        </button>
      </div>
      <div className="visualization-content">{renderContent()}</div>
    </div>
  );
}

export default VisualizationPanel;
