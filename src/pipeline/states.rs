#![deny(clippy::all)]
#![warn(clippy::pedantic)]

/// Marker type for pipeline in idle state, waiting for query input.
pub struct Idle;

/// Marker type for pipeline actively parsing SQL.
pub struct Parsing;

/// Marker type for pipeline with successfully parsed AST.
pub struct Parsed;

/// Marker type for pipeline actively building join graph.
pub struct Analyzing;

/// Marker type for pipeline with completed analysis.
pub struct Analyzed;

/// Marker type for pipeline actively estimating cardinality.
pub struct Estimating;

/// Marker type for pipeline with all analysis complete.
pub struct Complete;

/// Sealed trait for all valid pipeline states.
pub trait PipelineState: private::Sealed + Send + Sync {}

impl PipelineState for Idle {}
impl PipelineState for Parsing {}
impl PipelineState for Parsed {}
impl PipelineState for Analyzing {}
impl PipelineState for Analyzed {}
impl PipelineState for Estimating {}
impl PipelineState for Complete {}

mod private {
    pub trait Sealed {}
    impl Sealed for super::Idle {}
    impl Sealed for super::Parsing {}
    impl Sealed for super::Parsed {}
    impl Sealed for super::Analyzing {}
    impl Sealed for super::Analyzed {}
    impl Sealed for super::Estimating {}
    impl Sealed for super::Complete {}
}

/// Trait bound for states that can transition to Parsed via `parse()`.
pub trait CanParse: PipelineState {}
impl CanParse for Idle {}

/// Trait bound for states that can transition to Analyzed via `analyze()`.
pub trait CanAnalyze: PipelineState {}
impl CanAnalyze for Parsed {}

/// Trait bound for states that can transition to Complete via `estimate()`.
pub trait CanEstimate: PipelineState {}
impl CanEstimate for Analyzed {}

/// Trait bound for states that can branch into simulation context.
pub trait CanSimulate: PipelineState {}
impl CanSimulate for Analyzed {}

/// Trait bound for states that can reset to Idle.
pub trait CanReset: PipelineState {}
impl CanReset for Idle {}
impl CanReset for Parsed {}
impl CanReset for Analyzed {}
impl CanReset for Complete {}
