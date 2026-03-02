import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import { VscChevronLeft, VscChevronRight } from "react-icons/vsc";

interface DataPreview {
  columns: string[];
  rows: string[][];
  total_rows: number;
}

interface Props {
  filePath: string;
  fileType: "csv" | "parquet";
}

const PAGE_SIZE = 100;

function DataTableView({ filePath, fileType }: Props) {
  const [data, setData] = useState<DataPreview | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [page, setPage] = useState(0);
  const [sortColumn, setSortColumn] = useState<number | null>(null);
  const [sortAsc, setSortAsc] = useState(true);

  useEffect(() => {
    loadData();
  }, [filePath, page]);

  const loadData = async () => {
    setLoading(true);
    setError(null);

    try {
      const command = fileType === "csv" ? "read_csv" : "read_parquet";
      const result = await invoke<DataPreview>(command, {
        path: filePath,
        limit: PAGE_SIZE,
        offset: page * PAGE_SIZE,
      });
      setData(result);
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

  const sortedRows = () => {
    if (!data || sortColumn === null) return data?.rows || [];

    return [...data.rows].sort((a, b) => {
      const aVal = a[sortColumn] || "";
      const bVal = b[sortColumn] || "";

      const aNum = parseFloat(aVal);
      const bNum = parseFloat(bVal);

      if (!isNaN(aNum) && !isNaN(bNum)) {
        return sortAsc ? aNum - bNum : bNum - aNum;
      }

      return sortAsc
        ? aVal.localeCompare(bVal)
        : bVal.localeCompare(aVal);
    });
  };

  const totalPages = data ? Math.ceil(data.total_rows / PAGE_SIZE) : 0;

  if (loading && !data) {
    return (
      <div className="data-table-loading">
        Loading data...
      </div>
    );
  }

  if (error) {
    return (
      <div className="data-table-error">
        Error: {error}
      </div>
    );
  }

  if (!data || data.columns.length === 0) {
    return (
      <div className="data-table-empty">
        No data available
      </div>
    );
  }

  return (
    <div className="data-table-container">
      <div className="data-table-info">
        <span className="data-table-rows">
          {data.total_rows.toLocaleString()} rows
        </span>
        <span className="data-table-cols">
          {data.columns.length} columns
        </span>
      </div>

      <div className="data-table-wrapper">
        <table className="data-table">
          <thead>
            <tr>
              <th className="data-table-row-num">#</th>
              {data.columns.map((col, idx) => (
                <th
                  key={idx}
                  onClick={() => handleSort(idx)}
                  className={sortColumn === idx ? "sorted" : ""}
                >
                  {col}
                  {sortColumn === idx && (
                    <span className="sort-indicator">
                      {sortAsc ? " ↑" : " ↓"}
                    </span>
                  )}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {sortedRows().map((row, rowIdx) => (
              <tr key={rowIdx}>
                <td className="data-table-row-num">
                  {page * PAGE_SIZE + rowIdx + 1}
                </td>
                {row.map((cell, cellIdx) => (
                  <td key={cellIdx} title={cell}>
                    {cell === "NULL" ? (
                      <span className="null-value">NULL</span>
                    ) : (
                      cell
                    )}
                  </td>
                ))}
              </tr>
            ))}
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
          <span className="pagination-info">
            Page {page + 1} of {totalPages}
          </span>
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
