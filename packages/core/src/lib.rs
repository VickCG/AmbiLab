#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![warn(missing_docs)]
#![deny(rust_2018_idioms)]

//! AmbiLab - AI-Powered Local Analytics IDE
//!
//! A high-performance, local-first analytical engine for SQL query analysis
//! with pre-execution intelligence and sub-200ms feedback.
//!
//! # Core Features
//!
//! - **Query Analysis Pipeline**: Parse, analyze, and estimate SQL queries
//! - **Join Explosion Detection**: Identify risky many-to-many joins
//! - **Cardinality Estimation**: Predict query output sizes
//! - **What-If Simulation**: Run parallel scenarios with modified assumptions
//!
//! # Example
//!
//! ```
//! use ambilab::pipeline::{Pipeline, PipelineBuilder};
//!
//! // Create and run the analysis pipeline
//! let pipeline = Pipeline::new("SELECT * FROM orders JOIN customers ON orders.customer_id = customers.id");
//!
//! let result = pipeline
//!     .parse()
//!     .and_then(|p| p.analyze())
//!     .and_then(|p| p.estimate());
//!
//! match result {
//!     Ok(complete) => {
//!         if let Some(estimation) = complete.estimation() {
//!             println!("Estimated rows: {}", estimation.estimated_rows.as_u64());
//!         }
//!     }
//!     Err(e) => eprintln!("Analysis failed: {}", e),
//! }
//! ```
//!
//! # What-If Simulation
//!
//! ```
//! use ambilab::pipeline::{Pipeline, Scenario};
//!
//! let pipeline = Pipeline::new("SELECT * FROM orders");
//! let analyzed = pipeline.parse().unwrap().analyze().unwrap();
//!
//! // Branch into simulation without consuming the pipeline
//! let mut sim = analyzed.branch_simulation().unwrap();
//!
//! sim.add_scenario(
//!     Scenario::new("double_orders")
//!         .multiply_rows("orders", 2.0)
//! );
//!
//! let results = sim.run_parallel();
//! ```

pub mod pipeline;

pub use pipeline::{
    AnalyzedContext, BuildError, EstimationContext, ParsedContext, Pipeline, PipelineBuilder,
    PipelineError, PipelineResult, RowCount, Scenario, SimulationContext,
};
