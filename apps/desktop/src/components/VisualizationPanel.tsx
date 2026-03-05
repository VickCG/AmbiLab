import { useState } from "react";
import {
  VscClose,
  VscChevronDown,
  VscChevronUp,
  VscTable,
  VscGraph,
} from "react-icons/vsc";
import DataTableView, { DataPreviewResult } from "./DataTableView";
import SqlAnalysisView from "./SqlAnalysisView";
import ChartView from "./ChartView";

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

type FileType = "csv" | "parquet" | "json" | "jsonl" | "sql" | "unknown";
type ViewMode = "table" | "chart";

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
    default:
      return "unknown";
  }
}

function getViewTitle(fileType: FileType): string {
  switch (fileType) {
    case "csv":
      return "CSV/TSV Data Preview";
    case "parquet":
      return "Parquet Data Preview";
    case "json":
      return "JSON Data Preview";
    case "jsonl":
      return "JSONL Data Preview";
    case "sql":
      return "SQL Analysis";
    default:
      return "Preview";
  }
}

function isDataFile(fileType: FileType): boolean {
  return ["csv", "parquet", "json", "jsonl"].includes(fileType);
}

function VisualizationPanel({
  file,
  height,
  collapsed,
  onToggleCollapse,
  onClose,
}: Props) {
  const [viewMode, setViewMode] = useState<ViewMode>("table");
  const [chartData, setChartData] = useState<DataPreviewResult | null>(null);

  if (!file) return null;

  const fileType = getFileType(file.name);

  if (fileType === "unknown") return null;

  const handleDataLoaded = (data: DataPreviewResult) => {
    setChartData(data);
  };

  const renderContent = () => {
    if (collapsed) return null;

    if (fileType === "sql") {
      return <SqlAnalysisView content={file.content} />;
    }

    if (isDataFile(fileType)) {
      if (viewMode === "chart" && chartData) {
        return <ChartView data={chartData} />;
      }
      return (
        <DataTableView
          filePath={file.path}
          fileType={fileType as "csv" | "parquet" | "json" | "jsonl"}
          onDataLoaded={handleDataLoaded}
        />
      );
    }

    return null;
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

        {isDataFile(fileType) && !collapsed && (
          <div className="visualization-mode-toggle">
            <button
              className={`mode-btn ${viewMode === "table" ? "active" : ""}`}
              onClick={() => setViewMode("table")}
              title="Table View"
            >
              <VscTable />
            </button>
            <button
              className={`mode-btn ${viewMode === "chart" ? "active" : ""}`}
              onClick={() => setViewMode("chart")}
              title="Chart View"
              disabled={!chartData}
            >
              <VscGraph />
            </button>
          </div>
        )}

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
