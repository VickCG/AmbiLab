import { useState, useMemo } from "react";
import {
  BarChart,
  Bar,
  LineChart,
  Line,
  PieChart,
  Pie,
  ScatterChart,
  Scatter,
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  Cell,
} from "recharts";

interface DataPreview {
  columns: string[];
  rows: string[][];
  total_rows: number;
}

interface Props {
  data: DataPreview;
}

type ChartType = "bar" | "line" | "pie" | "scatter" | "area";

const CHART_COLORS = [
  "#8884d8",
  "#82ca9d",
  "#ffc658",
  "#ff7300",
  "#00C49F",
  "#FFBB28",
  "#FF8042",
  "#0088FE",
];

function isNumeric(value: string): boolean {
  if (value === "NULL" || value === "") return false;
  return !isNaN(parseFloat(value)) && isFinite(Number(value));
}

function detectNumericColumns(data: DataPreview): number[] {
  if (data.rows.length === 0) return [];

  const numericIndices: number[] = [];
  for (let i = 0; i < data.columns.length; i++) {
    const sampleValues = data.rows.slice(0, 10).map((row) => row[i]);
    const numericCount = sampleValues.filter(isNumeric).length;
    if (numericCount >= sampleValues.length * 0.7) {
      numericIndices.push(i);
    }
  }
  return numericIndices;
}

function ChartView({ data }: Props) {
  const [chartType, setChartType] = useState<ChartType>("bar");
  const [xColumn, setXColumn] = useState(0);
  const [yColumn, setYColumn] = useState(1);

  const numericColumns = useMemo(() => detectNumericColumns(data), [data]);

  const chartData = useMemo(() => {
    return data.rows.slice(0, 100).map((row, idx) => {
      const item: Record<string, string | number> = {
        name: row[xColumn] || `Row ${idx + 1}`,
        [data.columns[xColumn]]: row[xColumn],
      };

      if (numericColumns.includes(yColumn)) {
        item[data.columns[yColumn]] = parseFloat(row[yColumn]) || 0;
      } else {
        item[data.columns[yColumn]] = row[yColumn];
      }

      return item;
    });
  }, [data, xColumn, yColumn, numericColumns]);

  const pieData = useMemo(() => {
    const counts: Record<string, number> = {};
    data.rows.slice(0, 100).forEach((row) => {
      const key = row[xColumn] || "Unknown";
      counts[key] = (counts[key] || 0) + 1;
    });
    return Object.entries(counts).map(([name, value]) => ({ name, value }));
  }, [data, xColumn]);

  if (data.columns.length === 0) {
    return <div className="chart-empty">No data available for chart</div>;
  }

  const renderChart = () => {
    const yKey = data.columns[yColumn];
    const xKey = data.columns[xColumn];

    switch (chartType) {
      case "bar":
        return (
          <ResponsiveContainer width="100%" height={300}>
            <BarChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#3c3c3c" />
              <XAxis dataKey="name" tick={{ fill: "#ccc", fontSize: 11 }} />
              <YAxis tick={{ fill: "#ccc", fontSize: 11 }} />
              <Tooltip
                contentStyle={{ backgroundColor: "#252526", border: "1px solid #3c3c3c" }}
              />
              <Legend />
              <Bar dataKey={yKey} fill="#8884d8" />
            </BarChart>
          </ResponsiveContainer>
        );

      case "line":
        return (
          <ResponsiveContainer width="100%" height={300}>
            <LineChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#3c3c3c" />
              <XAxis dataKey="name" tick={{ fill: "#ccc", fontSize: 11 }} />
              <YAxis tick={{ fill: "#ccc", fontSize: 11 }} />
              <Tooltip
                contentStyle={{ backgroundColor: "#252526", border: "1px solid #3c3c3c" }}
              />
              <Legend />
              <Line type="monotone" dataKey={yKey} stroke="#8884d8" dot={false} />
            </LineChart>
          </ResponsiveContainer>
        );

      case "area":
        return (
          <ResponsiveContainer width="100%" height={300}>
            <AreaChart data={chartData}>
              <CartesianGrid strokeDasharray="3 3" stroke="#3c3c3c" />
              <XAxis dataKey="name" tick={{ fill: "#ccc", fontSize: 11 }} />
              <YAxis tick={{ fill: "#ccc", fontSize: 11 }} />
              <Tooltip
                contentStyle={{ backgroundColor: "#252526", border: "1px solid #3c3c3c" }}
              />
              <Legend />
              <Area type="monotone" dataKey={yKey} fill="#8884d8" stroke="#8884d8" />
            </AreaChart>
          </ResponsiveContainer>
        );

      case "scatter":
        return (
          <ResponsiveContainer width="100%" height={300}>
            <ScatterChart>
              <CartesianGrid strokeDasharray="3 3" stroke="#3c3c3c" />
              <XAxis
                dataKey={xKey}
                name={xKey}
                tick={{ fill: "#ccc", fontSize: 11 }}
                type="number"
              />
              <YAxis
                dataKey={yKey}
                name={yKey}
                tick={{ fill: "#ccc", fontSize: 11 }}
                type="number"
              />
              <Tooltip
                contentStyle={{ backgroundColor: "#252526", border: "1px solid #3c3c3c" }}
              />
              <Legend />
              <Scatter name={yKey} data={chartData} fill="#8884d8" />
            </ScatterChart>
          </ResponsiveContainer>
        );

      case "pie":
        return (
          <ResponsiveContainer width="100%" height={300}>
            <PieChart>
              <Pie
                data={pieData}
                dataKey="value"
                nameKey="name"
                cx="50%"
                cy="50%"
                outerRadius={100}
                label
              >
                {pieData.map((_, index) => (
                  <Cell key={`cell-${index}`} fill={CHART_COLORS[index % CHART_COLORS.length]} />
                ))}
              </Pie>
              <Tooltip
                contentStyle={{ backgroundColor: "#252526", border: "1px solid #3c3c3c" }}
              />
              <Legend />
            </PieChart>
          </ResponsiveContainer>
        );

      default:
        return null;
    }
  };

  return (
    <div className="chart-view">
      <div className="chart-controls">
        <div className="chart-control-group">
          <label>Chart Type:</label>
          <select value={chartType} onChange={(e) => setChartType(e.target.value as ChartType)}>
            <option value="bar">Bar Chart</option>
            <option value="line">Line Chart</option>
            <option value="area">Area Chart</option>
            <option value="scatter">Scatter Plot</option>
            <option value="pie">Pie Chart</option>
          </select>
        </div>

        <div className="chart-control-group">
          <label>X Axis:</label>
          <select value={xColumn} onChange={(e) => setXColumn(parseInt(e.target.value))}>
            {data.columns.map((col, idx) => (
              <option key={idx} value={idx}>
                {col}
              </option>
            ))}
          </select>
        </div>

        {chartType !== "pie" && (
          <div className="chart-control-group">
            <label>Y Axis:</label>
            <select value={yColumn} onChange={(e) => setYColumn(parseInt(e.target.value))}>
              {data.columns.map((col, idx) => (
                <option key={idx} value={idx}>
                  {col} {numericColumns.includes(idx) ? "(numeric)" : ""}
                </option>
              ))}
            </select>
          </div>
        )}
      </div>

      <div className="chart-container">{renderChart()}</div>
    </div>
  );
}

export default ChartView;
