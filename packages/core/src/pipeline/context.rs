#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use std::sync::Arc;

use petgraph::graph::{DiGraph, NodeIndex};
use rustc_hash::FxHashMap;
use smallvec::SmallVec;
use sqlparser::ast::Statement;

/// Context produced after successfully parsing SQL.
#[derive(Debug, Clone)]
pub struct ParsedContext {
    pub ast: Arc<Statement>,
    pub tables: SmallVec<[TableReference; 8]>,
    pub columns: SmallVec<[ColumnReference; 32]>,
    pub parse_time_micros: u64,
}

/// Context produced after analyzing query structure.
#[derive(Debug, Clone)]
pub struct AnalyzedContext {
    pub join_graph: JoinGraph,
    pub table_stats: TableStatistics,
    pub join_infos: SmallVec<[JoinInfo; 8]>,
    pub analysis_time_micros: u64,
}

/// Context produced after cardinality estimation.
#[derive(Debug, Clone)]
pub struct EstimationContext {
    pub estimated_rows: RowCount,
    pub estimated_cost: QueryCost,
    pub explosion_warnings: SmallVec<[ExplosionWarning; 4]>,
    pub estimation_time_micros: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RowCount(pub u64);

impl RowCount {
    #[inline]
    #[must_use]
    pub const fn new(count: u64) -> Self {
        Self(count)
    }

    #[inline]
    #[must_use]
    pub fn checked_multiply(self, other: Self) -> Option<Self> {
        self.0.checked_mul(other.0).map(Self)
    }

    #[inline]
    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct QueryCost {
    pub scan_bytes: u64,
    pub estimated_memory_bytes: u64,
    pub parallelism_factor: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TableReference {
    pub schema: Option<Arc<str>>,
    pub name: Arc<str>,
    pub alias: Option<Arc<str>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ColumnReference {
    pub table: Option<Arc<str>>,
    pub name: Arc<str>,
}

#[derive(Debug, Clone)]
pub struct JoinInfo {
    pub left_table: Arc<str>,
    pub right_table: Arc<str>,
    pub join_type: JoinType,
    pub cardinality: JoinCardinality,
    pub left_columns: SmallVec<[Arc<str>; 2]>,
    pub right_columns: SmallVec<[Arc<str>; 2]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JoinCardinality {
    OneToOne,
    OneToMany,
    ManyToOne,
    ManyToMany,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct ExplosionWarning {
    pub left_table: Arc<str>,
    pub right_table: Arc<str>,
    pub estimated_factor: f64,
    pub severity: WarningSeverity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WarningSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone)]
pub struct TableNode {
    pub name: Arc<str>,
    pub row_count: u64,
}

#[derive(Debug, Clone)]
pub struct JoinEdge {
    pub join_type: JoinType,
    pub left_columns: SmallVec<[Arc<str>; 2]>,
    pub right_columns: SmallVec<[Arc<str>; 2]>,
    pub selectivity: f64,
}

#[derive(Debug, Clone, Default)]
pub struct JoinGraph {
    pub(crate) graph: DiGraph<TableNode, JoinEdge>,
    pub(crate) name_to_index: FxHashMap<Arc<str>, NodeIndex>,
}

impl JoinGraph {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_capacity(nodes: usize, edges: usize) -> Self {
        Self {
            graph: DiGraph::with_capacity(nodes, edges),
            name_to_index: FxHashMap::with_capacity_and_hasher(nodes, Default::default()),
        }
    }

    pub fn add_table(&mut self, node: TableNode) -> NodeIndex {
        let name = Arc::clone(&node.name);
        let idx = self.graph.add_node(node);
        self.name_to_index.insert(name, idx);
        idx
    }

    pub fn add_join(&mut self, from: NodeIndex, to: NodeIndex, edge: JoinEdge) {
        self.graph.add_edge(from, to, edge);
    }

    #[must_use]
    pub fn get_table_index(&self, name: &str) -> Option<NodeIndex> {
        self.name_to_index.get(name).copied()
    }

    #[must_use]
    pub fn table_count(&self) -> usize {
        self.graph.node_count()
    }

    #[must_use]
    pub fn join_count(&self) -> usize {
        self.graph.edge_count()
    }
}

#[derive(Debug, Clone, Default)]
pub struct TableStatistics {
    pub row_counts: FxHashMap<Arc<str>, u64>,
    pub column_stats: FxHashMap<Arc<str>, ColumnStats>,
}

impl TableStatistics {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_row_count(&mut self, table: Arc<str>, count: u64) {
        self.row_counts.insert(table, count);
    }

    #[must_use]
    pub fn get_row_count(&self, table: &str) -> Option<u64> {
        self.row_counts.get(table).copied()
    }
}

#[derive(Debug, Clone, Default)]
pub struct ColumnStats {
    pub distinct_count: u64,
    pub null_count: u64,
    pub min: Option<ScalarValue>,
    pub max: Option<ScalarValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScalarValue {
    Int64(i64),
    Float64(f64),
    String(Arc<str>),
    Boolean(bool),
    Null,
}
