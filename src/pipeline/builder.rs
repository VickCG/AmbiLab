#![deny(clippy::all)]
#![warn(clippy::pedantic)]

use std::sync::Arc;

use crate::pipeline::context::TableStatistics;
use crate::pipeline::error::BuildError;
use crate::pipeline::pipeline::Pipeline;
use crate::pipeline::states::Idle;

const DEFAULT_TIMEOUT_MICROS: u64 = 200_000;
const DEFAULT_PARALLEL_THRESHOLD: usize = 10_000;

/// Builder for constructing pipelines with configuration options.
#[derive(Debug, Clone)]
#[must_use = "PipelineBuilder does nothing until build() is called"]
pub struct PipelineBuilder {
    query: Option<Arc<str>>,
    statistics: Option<TableStatistics>,
    timeout_micros: u64,
    parallel_threshold: usize,
}

impl PipelineBuilder {
    /// Creates a new pipeline builder with default settings.
    #[inline]
    pub fn new() -> Self {
        Self {
            query: None,
            statistics: None,
            timeout_micros: DEFAULT_TIMEOUT_MICROS,
            parallel_threshold: DEFAULT_PARALLEL_THRESHOLD,
        }
    }

    /// Sets the SQL query to analyze.
    #[inline]
    pub fn query(mut self, query: impl Into<Arc<str>>) -> Self {
        self.query = Some(query.into());
        self
    }

    /// Sets the table statistics for cardinality estimation.
    #[inline]
    pub fn statistics(mut self, stats: TableStatistics) -> Self {
        self.statistics = Some(stats);
        self
    }

    /// Sets the timeout in microseconds for operations.
    #[inline]
    pub fn timeout_micros(mut self, micros: u64) -> Self {
        self.timeout_micros = micros;
        self
    }

    /// Sets the threshold for parallel processing.
    #[inline]
    pub fn parallel_threshold(mut self, threshold: usize) -> Self {
        self.parallel_threshold = threshold;
        self
    }

    /// Builds the pipeline.
    ///
    /// # Errors
    ///
    /// Returns `BuildError::MissingQuery` if no query was provided.
    pub fn build(self) -> Result<Pipeline<Idle>, BuildError> {
        let query = self.query.ok_or(BuildError::MissingQuery)?;
        Ok(Pipeline::new(query))
    }

    /// Returns the configured timeout in microseconds.
    #[must_use]
    pub const fn get_timeout_micros(&self) -> u64 {
        self.timeout_micros
    }

    /// Returns the configured parallel threshold.
    #[must_use]
    pub const fn get_parallel_threshold(&self) -> usize {
        self.parallel_threshold
    }
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_default() {
        let builder = PipelineBuilder::new();
        assert_eq!(builder.get_timeout_micros(), DEFAULT_TIMEOUT_MICROS);
        assert_eq!(builder.get_parallel_threshold(), DEFAULT_PARALLEL_THRESHOLD);
    }

    #[test]
    fn test_builder_with_query() {
        let result = PipelineBuilder::new()
            .query("SELECT * FROM users")
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_builder_missing_query() {
        let result = PipelineBuilder::new().build();
        assert!(matches!(result, Err(BuildError::MissingQuery)));
    }

    #[test]
    fn test_builder_custom_timeout() {
        let builder = PipelineBuilder::new().timeout_micros(500_000);
        assert_eq!(builder.get_timeout_micros(), 500_000);
    }
}
