import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import {
  VscTable,
  VscSymbolMisc,
  VscWarning,
  VscInfo,
} from "react-icons/vsc";

interface AnalysisResult {
  tables: string[];
  join_count: number;
  estimated_rows: number;
  warnings: string[];
}

interface Props {
  content: string;
}

function SqlAnalysisView({ content }: Props) {
  const [analysis, setAnalysis] = useState<AnalysisResult | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    analyzeQuery();
  }, [content]);

  const analyzeQuery = async () => {
    if (!content.trim()) {
      setAnalysis(null);
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      const result = await invoke<AnalysisResult>("analyze_sql", {
        query: content,
      });
      setAnalysis(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  if (loading) {
    return (
      <div className="sql-analysis-loading">
        Analyzing query...
      </div>
    );
  }

  if (error) {
    return (
      <div className="sql-analysis-error">
        <VscWarning className="error-icon" />
        <span>{error}</span>
      </div>
    );
  }

  if (!analysis) {
    return (
      <div className="sql-analysis-empty">
        <VscInfo className="info-icon" />
        <span>Enter a SQL query to see analysis</span>
      </div>
    );
  }

  return (
    <div className="sql-analysis-container">
      <div className="sql-analysis-section">
        <div className="section-header">
          <VscTable className="section-icon" />
          <span>Tables ({analysis.tables.length})</span>
        </div>
        <div className="section-content">
          {analysis.tables.length > 0 ? (
            <div className="table-list">
              {analysis.tables.map((table, idx) => (
                <span key={idx} className="table-badge">
                  {table}
                </span>
              ))}
            </div>
          ) : (
            <span className="no-data">No tables detected</span>
          )}
        </div>
      </div>

      <div className="sql-analysis-section">
        <div className="section-header">
          <VscSymbolMisc className="section-icon" />
          <span>Statistics</span>
        </div>
        <div className="section-content stats-grid">
          <div className="stat-item">
            <span className="stat-label">Joins</span>
            <span className="stat-value">{analysis.join_count}</span>
          </div>
          <div className="stat-item">
            <span className="stat-label">Est. Rows</span>
            <span className="stat-value">
              {analysis.estimated_rows > 0
                ? analysis.estimated_rows.toLocaleString()
                : "N/A"}
            </span>
          </div>
        </div>
      </div>

      {analysis.warnings.length > 0 && (
        <div className="sql-analysis-section warnings">
          <div className="section-header">
            <VscWarning className="section-icon warning" />
            <span>Warnings ({analysis.warnings.length})</span>
          </div>
          <div className="section-content">
            <ul className="warning-list">
              {analysis.warnings.map((warning, idx) => (
                <li key={idx} className="warning-item">
                  {warning}
                </li>
              ))}
            </ul>
          </div>
        </div>
      )}
    </div>
  );
}

export default SqlAnalysisView;
