#![deny(clippy::all)]
#![warn(clippy::pedantic)]

//! Query analysis pipeline with compile-time state verification.
//!
//! This module provides a typestate-based pipeline for analyzing SQL queries.
//! The pipeline progresses through distinct states: Idle → Parsed → Analyzed → Complete.
//!
//! # Example
//!
//! ```
//! use ambilab::pipeline::{Pipeline, PipelineBuilder};
//!
//! let pipeline = Pipeline::new("SELECT * FROM users");
//! let complete = pipeline
//!     .parse()
//!     .and_then(|p| p.analyze())
//!     .and_then(|p| p.estimate());
//! ```

mod builder;
mod context;
mod error;
mod pipeline;
mod simulation;
mod states;
mod transitions;

pub use builder::PipelineBuilder;
pub use context::{
    AnalyzedContext, ColumnReference, ColumnStats, EstimationContext, ExplosionWarning,
    JoinCardinality, JoinEdge, JoinGraph, JoinInfo, JoinType, ParsedContext, QueryCost, RowCount,
    ScalarValue, TableNode, TableReference, TableStatistics, WarningSeverity,
};
pub use error::{
    AnalysisError, BuildError, EstimationError, ParseError, PipelineError, PipelineResult,
    SimulationError,
};
pub use pipeline::Pipeline;
pub use simulation::{
    Modification, Scenario, SimulationContext, SimulationResult, SimulationWarning,
    SimulationWarningSeverity,
};
pub use states::{
    Analyzed, Analyzing, CanAnalyze, CanEstimate, CanParse, CanReset, CanSimulate, Complete,
    Estimating, Idle, Parsed, Parsing, PipelineState,
};
