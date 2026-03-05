import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { useVirtualizer } from "@tanstack/react-virtual";
import { VscChevronLeft, VscChevronRight } from "react-icons/vsc";

interface DataPreview {
  columns: string[];
  rows: string[][];
  total_rows: number;
}

export interface DataPreviewResult {
  columns: string[];
  rows: string[][];
  total_rows: number;
}

interface Props {
  filePath: string;
  fileType: "csv" | "parquet" | "json" | "jsonl";
  onDataLoaded?: (data: DataPreviewResult) => void;
}

const PAGE_SIZE = 100;
const ROW_HEIGHT = 32;
const DEFAULT_COL_WIDTH = 150;
const MIN_COL_WIDTH = 50;

const COMMAND_MAP: Record<string, string> = {
  csv: "read_csv",
  parquet: "read_parquet",
  json: "read_json",
  jsonl: "read_jsonl",
};

function DataTableView({ filePath, fileType, onDataLoaded }: Props) {
  const [data, setData] = useState<DataPreview | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [page, setPage] = useState(0);
  const [sortColumn, setSortColumn] = useState<number | null>(null);
  const [sortAsc, setSortAsc] = useState(true);
  const [colWidths, setColWidths] = useState<Record<number, number>>({});

  const parentRef = useRef<HTMLDivElement>(null);
  const resizingRef = useRef<{ colIdx: number; startX: number; startWidth: number } | null>(null);

  useEffect(() => {
    setColWidths({});
    loadData();
  }, [filePath, page]);

  const loadData = async () => {
    setLoading(true);
    setError(null);
    try {
      const command = COMMAND_MAP[fileType] || "read_csv";
      const result = await invoke<DataPreview>(command, {
        path: filePath,
        limit: PAGE_SIZE,
        offset: page * PAGE_SIZE,
      });
      setData(result);
      if (onDataLoaded && page === 0) onDataLoaded(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  const handleSort = (colIdx: number) => {
    if (sortColumn === colIdx) {
      setSortAsc(!sortAsc);
    } else {
      setSortColumn(colIdx);
      setSortAsc(true);
    }
  };

  const handleResizeStart = (e: React.MouseEvent, colIdx: number) => {
    e.stopPropagation();
    e.preventDefault();
    const startWidth = colWidths[colIdx] ?? DEFAULT_COL_WIDTH;
    resizingRef.current = { colIdx, startX: e.clientX, startWidth };

    const onMove = (ev: MouseEvent) => {
      if (!resizingRef.current) return;
      const { colIdx: idx, startX, startWidth: sw } = resizingRef.current;
      const newWidth = Math.max(MIN_COL_WIDTH, sw + ev.clientX - startX);
      setColWidths((prev) => ({ ...prev, [idx]: newWidth }));
    };

    const onUp = () => {
      resizingRef.current = null;
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };

    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  };

  const sortedRows = (): string[][] => {
    if (!data || sortColumn === null) return data?.rows || [];
    return [...data.rows].sort((a, b) => {
      const aVal = a[sortColumn] || "";
      const bVal = b[sortColumn] || "";
      const aNum = parseFloat(aVal);
      const bNum = parseFloat(bVal);
      if (!isNaN(aNum) && !isNaN(bNum)) return sortAsc ? aNum - bNum : bNum - aNum;
      return sortAsc ? aVal.localeCompare(bVal) : bVal.localeCompare(aVal);
    });
  };

  const rows = sortedRows();

  const rowVirtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 20,
  });

  const virtualItems = rowVirtualizer.getVirtualItems();
  const paddingTop = virtualItems.length > 0 ? virtualItems[0].start : 0;
  const paddingBottom =
    virtualItems.length > 0
      ? rowVirtualizer.getTotalSize() - virtualItems[virtualItems.length - 1].end
      : 0;

  const totalPages = data ? Math.ceil(data.total_rows / PAGE_SIZE) : 0;

  if (loading && !data) return <div className="data-table-loading">Loading data...</div>;
  if (error) return <div className="data-table-error">Error: {error}</div>;
  if (!data || data.columns.length === 0) return <div className="data-table-empty">No data available</div>;

  return (
    <div className="data-table-container">
      <div className="data-table-info">
        <span className="data-table-rows">
          {data.total_rows > 0
            ? data.total_rows.toLocaleString() + " rows"
            : loading
            ? "counting rows…"
            : "rows"}
        </span>
        <span className="data-table-cols">{data.columns.length} columns</span>
      </div>

      <div ref={parentRef} className="data-table-wrapper">
        <table className="data-table">
          <colgroup>
            <col style={{ width: 48 }} />
            {data.columns.map((_, idx) => (
              <col key={idx} style={{ width: colWidths[idx] ?? DEFAULT_COL_WIDTH }} />
            ))}
          </colgroup>
          <thead>
            <tr>
              <th className="data-table-row-num">#</th>
              {data.columns.map((col, idx) => (
                <th
                  key={idx}
                  onClick={() => handleSort(idx)}
                  className={sortColumn === idx ? "sorted" : ""}
                  style={{ width: colWidths[idx] ?? DEFAULT_COL_WIDTH }}
                >
                  <span className="th-label">
                    {col}
                    {sortColumn === idx && (
                      <span className="sort-indicator">{sortAsc ? " ↑" : " ↓"}</span>
                    )}
                  </span>
                  <div
                    className="col-resize-handle"
                    onMouseDown={(e) => handleResizeStart(e, idx)}
                  />
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {paddingTop > 0 && (
              <tr>
                <td style={{ height: `${paddingTop}px`, padding: 0, border: "none" }} colSpan={data.columns.length + 1} />
              </tr>
            )}
            {virtualItems.map((virtualRow) => {
              const row = rows[virtualRow.index];
              return (
                <tr key={virtualRow.index} style={{ height: `${ROW_HEIGHT}px` }}>
                  <td className="data-table-row-num">{page * PAGE_SIZE + virtualRow.index + 1}</td>
                  {row.map((cell, cellIdx) => (
                    <td key={cellIdx} title={cell}>
                      {cell === "NULL" ? <span className="null-value">NULL</span> : cell}
                    </td>
                  ))}
                </tr>
              );
            })}
            {paddingBottom > 0 && (
              <tr>
                <td style={{ height: `${paddingBottom}px`, padding: 0, border: "none" }} colSpan={data.columns.length + 1} />
              </tr>
            )}
          </tbody>
        </table>
      </div>

      {totalPages > 1 && (
        <div className="data-table-pagination">
          <button
            className="pagination-btn"
            onClick={() => setPage((p) => Math.max(0, p - 1))}
            disabled={page === 0}
          >
            <VscChevronLeft />
          </button>
          <span className="pagination-info">Page {page + 1} of {totalPages}</span>
          <button
            className="pagination-btn"
            onClick={() => setPage((p) => Math.min(totalPages - 1, p + 1))}
            disabled={page >= totalPages - 1}
          >
            <VscChevronRight />
          </button>
        </div>
      )}
    </div>
  );
}

export default DataTableView;
