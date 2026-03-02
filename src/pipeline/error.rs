#![deny(clippy::all)]
#![warn(clippy::pedantic)]

use std::sync::Arc;
use thiserror::Error;

/// Top-level error type for all pipeline operations.
#[derive(Error, Debug)]
pub enum PipelineError {
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),

    #[error("analysis error: {0}")]
    Analysis(#[from] AnalysisError),

    #[error("estimation error: {0}")]
    Estimation(#[from] EstimationError),

    #[error("simulation error: {0}")]
    Simulation(#[from] SimulationError),

    #[error("missing context: {0}")]
    MissingContext(&'static str),
}

/// Errors that occur during SQL parsing.
#[derive(Error, Debug)]
pub enum ParseError {
    #[error("invalid SQL syntax at position {position}: {message}")]
    InvalidSyntax { position: usize, message: Arc<str> },

    #[error("unsupported SQL feature: {feature}")]
    UnsupportedFeature { feature: Arc<str> },

    #[error("empty query")]
    EmptyQuery,

    #[error("query too large: {size} bytes (max: {max})")]
    QueryTooLarge { size: usize, max: usize },

    #[error("multiple statements not supported")]
    MultipleStatements,
}

/// Errors that occur during query analysis.
#[derive(Error, Debug)]
pub enum AnalysisError {
    #[error("unknown table: {table}")]
    UnknownTable { table: Arc<str> },

    #[error("unknown column: {column} in table {table}")]
    UnknownColumn { table: Arc<str>, column: Arc<str> },

    #[error("ambiguous column reference: {column}")]
    AmbiguousColumn { column: Arc<str> },

    #[error("circular join dependency detected: {tables:?}")]
    CircularDependency { tables: Vec<Arc<str>> },

    #[error("statistics unavailable for table: {table}")]
    MissingStatistics { table: Arc<str> },

    #[error("unsupported join type")]
    UnsupportedJoinType,
}

/// Errors that occur during cardinality estimation.
#[derive(Error, Debug)]
pub enum EstimationError {
    #[error("cardinality overflow: result exceeds u64::MAX")]
    CardinalityOverflow,

    #[error("invalid selectivity: {value} (must be 0.0 to 1.0)")]
    InvalidSelectivity { value: f64 },

    #[error("missing join statistics for {left} -> {right}")]
    MissingJoinStats { left: Arc<str>, right: Arc<str> },

    #[error("negative row estimate")]
    NegativeRowEstimate,
}

/// Errors that occur during what-if simulation.
#[derive(Error, Debug)]
pub enum SimulationError {
    #[error("missing analysis context for simulation")]
    MissingAnalysis,

    #[error("invalid modification: {reason}")]
    InvalidModification { reason: Arc<str> },

    #[error("scenario execution failed: {scenario}")]
    ScenarioFailed { scenario: Arc<str> },

    #[error("parallel execution error: {message}")]
    ParallelExecutionError { message: Arc<str> },

    #[error("table not found in analysis: {table}")]
    TableNotFound { table: Arc<str> },
}

/// Errors that occur during pipeline construction.
#[derive(Error, Debug)]
pub enum BuildError {
    #[error("query is required")]
    MissingQuery,

    #[error("invalid configuration: {reason}")]
    InvalidConfig { reason: Arc<str> },
}

/// Convenience type alias for pipeline results.
pub type PipelineResult<T> = Result<T, PipelineError>;
