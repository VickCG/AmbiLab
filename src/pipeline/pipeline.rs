#![deny(clippy::all)]
#![warn(clippy::pedantic)]

use std::marker::PhantomData;
use std::sync::Arc;

use crate::pipeline::context::{AnalyzedContext, EstimationContext, ParsedContext};
use crate::pipeline::states::{Idle, PipelineState};

/// Core pipeline struct with compile-time state tracking.
///
/// The state parameter `S` is a phantom type that tracks the current
/// pipeline state at compile time, enabling the typestate pattern.
pub struct Pipeline<S: PipelineState> {
    pub(crate) raw_query: Arc<str>,
    pub(crate) parsed: Option<ParsedContext>,
    pub(crate) analyzed: Option<AnalyzedContext>,
    pub(crate) estimation: Option<EstimationContext>,
    pub(crate) _state: PhantomData<S>,
}

impl Pipeline<Idle> {
    /// Creates a new pipeline in the Idle state.
    #[inline]
    #[must_use]
    pub fn new(query: impl Into<Arc<str>>) -> Self {
        Self {
            raw_query: query.into(),
            parsed: None,
            analyzed: None,
            estimation: None,
            _state: PhantomData,
        }
    }
}

impl<S: PipelineState> Pipeline<S> {
    /// Returns the raw SQL query string.
    #[inline]
    #[must_use]
    pub fn query(&self) -> &str {
        &self.raw_query
    }

    /// Returns the query as an Arc for cheap cloning.
    #[inline]
    #[must_use]
    pub fn query_arc(&self) -> Arc<str> {
        Arc::clone(&self.raw_query)
    }

    /// Returns a reference to the parsed context if available.
    #[inline]
    #[must_use]
    pub fn parsed(&self) -> Option<&ParsedContext> {
        self.parsed.as_ref()
    }

    /// Returns a reference to the analyzed context if available.
    #[inline]
    #[must_use]
    pub fn analyzed(&self) -> Option<&AnalyzedContext> {
        self.analyzed.as_ref()
    }

    /// Returns a reference to the estimation context if available.
    #[inline]
    #[must_use]
    pub fn estimation(&self) -> Option<&EstimationContext> {
        self.estimation.as_ref()
    }

    /// Internal method to transition between states.
    #[inline]
    pub(crate) fn transition<T: PipelineState>(self) -> Pipeline<T> {
        Pipeline {
            raw_query: self.raw_query,
            parsed: self.parsed,
            analyzed: self.analyzed,
            estimation: self.estimation,
            _state: PhantomData,
        }
    }

    /// Internal method to transition with new parsed context.
    #[inline]
    pub(crate) fn transition_with_parsed<T: PipelineState>(
        self,
        parsed: ParsedContext,
    ) -> Pipeline<T> {
        Pipeline {
            raw_query: self.raw_query,
            parsed: Some(parsed),
            analyzed: None,
            estimation: None,
            _state: PhantomData,
        }
    }

    /// Internal method to transition with new analyzed context.
    #[inline]
    pub(crate) fn transition_with_analyzed<T: PipelineState>(
        self,
        analyzed: AnalyzedContext,
    ) -> Pipeline<T> {
        Pipeline {
            raw_query: self.raw_query,
            parsed: self.parsed,
            analyzed: Some(analyzed),
            estimation: None,
            _state: PhantomData,
        }
    }

    /// Internal method to transition with new estimation context.
    #[inline]
    pub(crate) fn transition_with_estimation<T: PipelineState>(
        self,
        estimation: EstimationContext,
    ) -> Pipeline<T> {
        Pipeline {
            raw_query: self.raw_query,
            parsed: self.parsed,
            analyzed: self.analyzed,
            estimation: Some(estimation),
            _state: PhantomData,
        }
    }
}

impl<S: PipelineState> std::fmt::Debug for Pipeline<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pipeline")
            .field("query", &self.raw_query)
            .field("has_parsed", &self.parsed.is_some())
            .field("has_analyzed", &self.analyzed.is_some())
            .field("has_estimation", &self.estimation.is_some())
            .field("state", &std::any::type_name::<S>())
            .finish()
    }
}

unsafe impl<S: PipelineState> Send for Pipeline<S> {}
unsafe impl<S: PipelineState> Sync for Pipeline<S> {}
