#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use std::sync::Arc;

use rayon::prelude::*;

use crate::pipeline::context::{AnalyzedContext, RowCount};
use crate::pipeline::error::SimulationError;
use crate::pipeline::pipeline::Pipeline;
use crate::pipeline::states::CanSimulate;

const MIN_PARALLEL_SCENARIOS: usize = 4;

/// Context for running what-if simulation scenarios.
#[derive(Debug, Clone)]
pub struct SimulationContext {
    base_analysis: Arc<AnalyzedContext>,
    scenarios: Vec<Scenario>,
}

/// A single what-if scenario with modifications to apply.
#[derive(Debug, Clone)]
pub struct Scenario {
    pub name: Arc<str>,
    pub modifications: Vec<Modification>,
}

/// Types of modifications that can be applied in a simulation.
#[derive(Debug, Clone)]
pub enum Modification {
    RowCountMultiplier { table: Arc<str>, factor: f64 },
    FilterSelectivity { table: Arc<str>, selectivity: f64 },
    JoinCardinality { left: Arc<str>, right: Arc<str>, factor: f64 },
}

/// Result of running a single simulation scenario.
#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub scenario_name: Arc<str>,
    pub estimated_rows: RowCount,
    pub delta_from_base: f64,
    pub warnings: Vec<SimulationWarning>,
}

/// Warning generated during simulation.
#[derive(Debug, Clone)]
pub struct SimulationWarning {
    pub message: Arc<str>,
    pub severity: SimulationWarningSeverity,
}

/// Severity levels for simulation warnings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationWarningSeverity {
    Info,
    Warning,
    Critical,
}

impl<S: CanSimulate> Pipeline<S> {
    /// Branches the current analysis into a simulation context.
    ///
    /// This allows running what-if scenarios without consuming the pipeline.
    ///
    /// # Errors
    ///
    /// Returns `SimulationError::MissingAnalysis` if no analysis context exists.
    pub fn branch_simulation(&self) -> Result<SimulationContext, SimulationError> {
        let analyzed = self
            .analyzed
            .as_ref()
            .ok_or(SimulationError::MissingAnalysis)?;

        Ok(SimulationContext {
            base_analysis: Arc::new(analyzed.clone()),
            scenarios: Vec::new(),
        })
    }
}

impl SimulationContext {
    /// Creates a new simulation context from an analyzed context.
    #[must_use]
    pub fn new(base: AnalyzedContext) -> Self {
        Self {
            base_analysis: Arc::new(base),
            scenarios: Vec::new(),
        }
    }

    /// Adds a scenario to the simulation.
    pub fn add_scenario(&mut self, scenario: Scenario) {
        self.scenarios.push(scenario);
    }

    /// Adds multiple scenarios at once.
    pub fn add_scenarios(&mut self, scenarios: impl IntoIterator<Item = Scenario>) {
        self.scenarios.extend(scenarios);
    }

    /// Returns the number of scenarios to run.
    #[must_use]
    pub fn scenario_count(&self) -> usize {
        self.scenarios.len()
    }

    /// Runs all scenarios in parallel using Rayon.
    ///
    /// Automatically falls back to sequential execution for small scenario counts.
    #[must_use]
    pub fn run_parallel(self) -> Vec<Result<SimulationResult, SimulationError>> {
        if self.scenarios.len() < MIN_PARALLEL_SCENARIOS {
            return self.run_sequential();
        }

        let base = Arc::clone(&self.base_analysis);
        self.scenarios
            .into_par_iter()
            .map(|scenario| run_single_scenario(&base, scenario))
            .collect()
    }

    /// Runs all scenarios sequentially.
    #[must_use]
    pub fn run_sequential(self) -> Vec<Result<SimulationResult, SimulationError>> {
        let base = &self.base_analysis;
        self.scenarios
            .into_iter()
            .map(|scenario| run_single_scenario(base, scenario))
            .collect()
    }

    /// Returns a reference to the base analysis.
    #[must_use]
    pub fn base_analysis(&self) -> &AnalyzedContext {
        &self.base_analysis
    }
}

impl Scenario {
    /// Creates a new scenario with the given name.
    #[must_use]
    pub fn new(name: impl Into<Arc<str>>) -> Self {
        Self {
            name: name.into(),
            modifications: Vec::new(),
        }
    }

    /// Adds a modification to this scenario.
    #[must_use]
    pub fn with_modification(mut self, modification: Modification) -> Self {
        self.modifications.push(modification);
        self
    }

    /// Adds a row count multiplier modification.
    #[must_use]
    pub fn multiply_rows(self, table: impl Into<Arc<str>>, factor: f64) -> Self {
        self.with_modification(Modification::RowCountMultiplier {
            table: table.into(),
            factor,
        })
    }

    /// Adds a filter selectivity modification.
    #[must_use]
    pub fn set_selectivity(self, table: impl Into<Arc<str>>, selectivity: f64) -> Self {
        self.with_modification(Modification::FilterSelectivity {
            table: table.into(),
            selectivity,
        })
    }

    /// Adds a join cardinality factor modification.
    #[must_use]
    pub fn adjust_join(
        self,
        left: impl Into<Arc<str>>,
        right: impl Into<Arc<str>>,
        factor: f64,
    ) -> Self {
        self.with_modification(Modification::JoinCardinality {
            left: left.into(),
            right: right.into(),
            factor,
        })
    }
}

fn run_single_scenario(
    base: &AnalyzedContext,
    scenario: Scenario,
) -> Result<SimulationResult, SimulationError> {
    let modified = apply_modifications(base, &scenario.modifications)?;
    let base_rows = compute_base_rows(base);
    let estimated_rows = compute_modified_rows(&modified);
    let delta = calculate_delta(base_rows, estimated_rows);
    let warnings = generate_warnings(&modified, delta);

    Ok(SimulationResult {
        scenario_name: scenario.name,
        estimated_rows,
        delta_from_base: delta,
        warnings,
    })
}

fn apply_modifications(
    base: &AnalyzedContext,
    modifications: &[Modification],
) -> Result<AnalyzedContext, SimulationError> {
    let mut modified = base.clone();

    for modification in modifications {
        apply_single_modification(&mut modified, modification)?;
    }

    Ok(modified)
}

fn apply_single_modification(
    context: &mut AnalyzedContext,
    modification: &Modification,
) -> Result<(), SimulationError> {
    match modification {
        Modification::RowCountMultiplier { table, factor } => {
            apply_row_multiplier(context, table, *factor)
        }
        Modification::FilterSelectivity { table, selectivity } => {
            apply_selectivity(context, table, *selectivity)
        }
        Modification::JoinCardinality { left, right, factor } => {
            apply_join_factor(context, left, right, *factor)
        }
    }
}

fn apply_row_multiplier(
    context: &mut AnalyzedContext,
    table: &str,
    factor: f64,
) -> Result<(), SimulationError> {
    let current = context.table_stats.get_row_count(table).ok_or_else(|| {
        SimulationError::TableNotFound {
            table: Arc::from(table),
        }
    })?;

    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let new_count = (current as f64 * factor).round() as u64;
    context
        .table_stats
        .set_row_count(Arc::from(table), new_count);

    Ok(())
}

fn apply_selectivity(
    context: &mut AnalyzedContext,
    table: &str,
    selectivity: f64,
) -> Result<(), SimulationError> {
    let current = context.table_stats.get_row_count(table).ok_or_else(|| {
        SimulationError::TableNotFound {
            table: Arc::from(table),
        }
    })?;

    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    let new_count = (current as f64 * selectivity).round() as u64;
    context
        .table_stats
        .set_row_count(Arc::from(table), new_count);

    Ok(())
}

fn apply_join_factor(
    _context: &mut AnalyzedContext,
    _left: &str,
    _right: &str,
    _factor: f64,
) -> Result<(), SimulationError> {
    Ok(())
}

fn compute_base_rows(context: &AnalyzedContext) -> RowCount {
    let max_rows = context
        .table_stats
        .row_counts
        .values()
        .copied()
        .max()
        .unwrap_or(1000);

    RowCount::new(max_rows)
}

fn compute_modified_rows(context: &AnalyzedContext) -> RowCount {
    compute_base_rows(context)
}

fn calculate_delta(base: RowCount, estimated: RowCount) -> f64 {
    if base.as_u64() == 0 {
        return 0.0;
    }

    (estimated.as_u64() as f64 - base.as_u64() as f64) / base.as_u64() as f64
}

fn generate_warnings(context: &AnalyzedContext, delta: f64) -> Vec<SimulationWarning> {
    let mut warnings = Vec::new();

    if delta > 10.0 {
        warnings.push(SimulationWarning {
            message: Arc::from("Row count increased by more than 10x"),
            severity: SimulationWarningSeverity::Critical,
        });
    } else if delta > 2.0 {
        warnings.push(SimulationWarning {
            message: Arc::from("Row count increased by more than 2x"),
            severity: SimulationWarningSeverity::Warning,
        });
    }

    if !context.join_infos.is_empty() && delta.abs() > 0.5 {
        warnings.push(SimulationWarning {
            message: Arc::from("Significant change in join output"),
            severity: SimulationWarningSeverity::Info,
        });
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::context::{JoinGraph, TableStatistics};

    fn create_test_analyzed_context() -> AnalyzedContext {
        let mut stats = TableStatistics::new();
        stats.set_row_count(Arc::from("orders"), 10000);
        stats.set_row_count(Arc::from("customers"), 1000);

        AnalyzedContext {
            join_graph: JoinGraph::new(),
            table_stats: stats,
            join_infos: smallvec::smallvec![],
            analysis_time_micros: 0,
        }
    }

    #[test]
    fn test_create_simulation_context() {
        let analyzed = create_test_analyzed_context();
        let sim = SimulationContext::new(analyzed);
        assert_eq!(sim.scenario_count(), 0);
    }

    #[test]
    fn test_add_scenario() {
        let analyzed = create_test_analyzed_context();
        let mut sim = SimulationContext::new(analyzed);

        sim.add_scenario(Scenario::new("double_orders").multiply_rows("orders", 2.0));

        assert_eq!(sim.scenario_count(), 1);
    }

    #[test]
    fn test_run_sequential() {
        let analyzed = create_test_analyzed_context();
        let mut sim = SimulationContext::new(analyzed);

        sim.add_scenario(Scenario::new("double_orders").multiply_rows("orders", 2.0));

        let results = sim.run_sequential();
        assert_eq!(results.len(), 1);
        assert!(results[0].is_ok());
    }
}
