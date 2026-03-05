#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use std::sync::Arc;
use std::time::Instant;

use smallvec::SmallVec;
use sqlparser::dialect::GenericDialect;
use sqlparser::parser::Parser;

use crate::pipeline::context::{
    AnalyzedContext, ColumnReference, EstimationContext, ExplosionWarning, JoinCardinality,
    JoinEdge, JoinGraph, JoinInfo, JoinType, ParsedContext, QueryCost, RowCount, TableNode,
    TableReference, TableStatistics, WarningSeverity,
};
use crate::pipeline::error::{AnalysisError, EstimationError, ParseError, PipelineError};
use crate::pipeline::pipeline::Pipeline;
use crate::pipeline::states::{Analyzed, CanAnalyze, CanEstimate, CanParse, Complete, Parsed};

const MAX_QUERY_SIZE: usize = 1_000_000;
const EXPLOSION_THRESHOLD_LOW: f64 = 10.0;
const EXPLOSION_THRESHOLD_MEDIUM: f64 = 100.0;
const EXPLOSION_THRESHOLD_HIGH: f64 = 1000.0;

impl<S: CanParse> Pipeline<S> {
    /// Parses the SQL query into an AST.
    ///
    /// # Errors
    ///
    /// Returns `ParseError` if the SQL syntax is invalid or unsupported.
    pub fn parse(self) -> Result<Pipeline<Parsed>, PipelineError> {
        let start = Instant::now();

        if self.raw_query.is_empty() {
            return Err(ParseError::EmptyQuery.into());
        }

        if self.raw_query.len() > MAX_QUERY_SIZE {
            return Err(ParseError::QueryTooLarge {
                size: self.raw_query.len(),
                max: MAX_QUERY_SIZE,
            }
            .into());
        }

        let dialect = GenericDialect {};
        let statements = Parser::parse_sql(&dialect, &self.raw_query).map_err(|e| {
            ParseError::InvalidSyntax {
                position: 0,
                message: Arc::from(e.to_string()),
            }
        })?;

        if statements.is_empty() {
            return Err(ParseError::EmptyQuery.into());
        }

        if statements.len() > 1 {
            return Err(ParseError::MultipleStatements.into());
        }

        let ast = Arc::new(
            statements
                .into_iter()
                .next()
                .ok_or(ParseError::EmptyQuery)?,
        );
        let tables = extract_tables(&ast);
        let columns = extract_columns(&ast);

        let parsed_context = ParsedContext {
            ast,
            tables,
            columns,
            parse_time_micros: start.elapsed().as_micros() as u64,
        };

        Ok(self.transition_with_parsed(parsed_context))
    }
}

impl<S: CanAnalyze> Pipeline<S> {
    /// Analyzes the parsed query to build join graph and extract metadata.
    ///
    /// # Errors
    ///
    /// Returns `AnalysisError` if analysis fails.
    pub fn analyze(self) -> Result<Pipeline<Analyzed>, PipelineError> {
        let start = Instant::now();

        let parsed = self
            .parsed
            .as_ref()
            .ok_or(PipelineError::MissingContext("parsed"))?;

        let join_graph = build_join_graph(parsed)?;
        let table_stats = build_table_statistics(parsed);
        let join_infos = extract_join_info(parsed)?;

        let analyzed_context = AnalyzedContext {
            join_graph,
            table_stats,
            join_infos,
            analysis_time_micros: start.elapsed().as_micros() as u64,
        };

        Ok(self.transition_with_analyzed(analyzed_context))
    }
}

impl<S: CanEstimate> Pipeline<S> {
    /// Estimates cardinality and detects potential join explosions.
    ///
    /// # Errors
    ///
    /// Returns `EstimationError` if estimation fails.
    pub fn estimate(self) -> Result<Pipeline<Complete>, PipelineError> {
        let start = Instant::now();

        let analyzed = self
            .analyzed
            .as_ref()
            .ok_or(PipelineError::MissingContext("analyzed"))?;

        let estimated_rows = estimate_cardinality(analyzed)?;
        let estimated_cost = estimate_query_cost(analyzed, estimated_rows);
        let explosion_warnings = detect_explosion_risks(analyzed);

        let estimation_context = EstimationContext {
            estimated_rows,
            estimated_cost,
            explosion_warnings,
            estimation_time_micros: start.elapsed().as_micros() as u64,
        };

        Ok(self.transition_with_estimation(estimation_context))
    }
}

impl Pipeline<Complete> {
    /// Returns the total processing time in microseconds.
    #[must_use]
    pub fn total_time_micros(&self) -> u64 {
        let parse_time = self.parsed.as_ref().map_or(0, |p| p.parse_time_micros);
        let analyze_time = self.analyzed.as_ref().map_or(0, |a| a.analysis_time_micros);
        let estimate_time = self
            .estimation
            .as_ref()
            .map_or(0, |e| e.estimation_time_micros);
        parse_time + analyze_time + estimate_time
    }
}

fn extract_tables(ast: &sqlparser::ast::Statement) -> SmallVec<[TableReference; 8]> {
    let mut tables = SmallVec::new();

    if let sqlparser::ast::Statement::Query(query) = ast {
        extract_tables_from_query(query, &mut tables);
    }

    tables
}

fn extract_tables_from_query(
    query: &sqlparser::ast::Query,
    tables: &mut SmallVec<[TableReference; 8]>,
) {
    if let sqlparser::ast::SetExpr::Select(select) = query.body.as_ref() {
        for table_with_joins in &select.from {
            extract_table_from_factor(&table_with_joins.relation, tables);

            for join in &table_with_joins.joins {
                extract_table_from_factor(&join.relation, tables);
            }
        }
    }
}

fn extract_table_from_factor(
    factor: &sqlparser::ast::TableFactor,
    tables: &mut SmallVec<[TableReference; 8]>,
) {
    if let sqlparser::ast::TableFactor::Table { name, alias, .. } = factor {
        let table_name = name.0.last().map_or_else(
            || Arc::from(""),
            |ident| Arc::from(ident.value.as_str()),
        );

        let schema = if name.0.len() > 1 {
            Some(Arc::from(name.0[0].value.as_str()))
        } else {
            None
        };

        let alias_name = alias.as_ref().map(|a| Arc::from(a.name.value.as_str()));

        tables.push(TableReference {
            schema,
            name: table_name,
            alias: alias_name,
        });
    }
}

fn extract_columns(_ast: &sqlparser::ast::Statement) -> SmallVec<[ColumnReference; 32]> {
    SmallVec::new()
}

fn build_join_graph(parsed: &ParsedContext) -> Result<JoinGraph, AnalysisError> {
    let table_count = parsed.tables.len();
    let mut graph = JoinGraph::with_capacity(table_count, table_count.saturating_sub(1));

    for table in &parsed.tables {
        let node = TableNode {
            name: Arc::clone(&table.name),
            row_count: 0,
        };
        graph.add_table(node);
    }

    if let sqlparser::ast::Statement::Query(query) = parsed.ast.as_ref() {
        add_joins_from_query(query, &mut graph)?;
    }

    Ok(graph)
}

fn add_joins_from_query(
    query: &sqlparser::ast::Query,
    graph: &mut JoinGraph,
) -> Result<(), AnalysisError> {
    if let sqlparser::ast::SetExpr::Select(select) = query.body.as_ref() {
        for table_with_joins in &select.from {
            let base_name = get_table_name(&table_with_joins.relation);

            for join in &table_with_joins.joins {
                let join_name = get_table_name(&join.relation);
                let join_type = convert_join_type(&join.join_operator);

                let from_idx = graph
                    .get_table_index(&base_name)
                    .ok_or_else(|| AnalysisError::UnknownTable {
                        table: Arc::from(base_name.as_str()),
                    })?;

                let to_idx = graph
                    .get_table_index(&join_name)
                    .ok_or_else(|| AnalysisError::UnknownTable {
                        table: Arc::from(join_name.as_str()),
                    })?;

                let edge = JoinEdge {
                    join_type,
                    left_columns: SmallVec::new(),
                    right_columns: SmallVec::new(),
                    selectivity: 1.0,
                };

                graph.add_join(from_idx, to_idx, edge);
            }
        }
    }

    Ok(())
}

fn get_table_name(factor: &sqlparser::ast::TableFactor) -> String {
    if let sqlparser::ast::TableFactor::Table { name, alias, .. } = factor {
        if let Some(a) = alias {
            return a.name.value.clone();
        }

        return name
            .0
            .last()
            .map_or_else(String::new, |ident| ident.value.clone());
    }

    String::new()
}

fn convert_join_type(operator: &sqlparser::ast::JoinOperator) -> JoinType {
    match operator {
        sqlparser::ast::JoinOperator::Inner(_) => JoinType::Inner,
        sqlparser::ast::JoinOperator::LeftOuter(_) => JoinType::Left,
        sqlparser::ast::JoinOperator::RightOuter(_) => JoinType::Right,
        sqlparser::ast::JoinOperator::FullOuter(_) => JoinType::Full,
        sqlparser::ast::JoinOperator::CrossJoin => JoinType::Cross,
        _ => JoinType::Inner,
    }
}

fn build_table_statistics(_parsed: &ParsedContext) -> TableStatistics {
    TableStatistics::new()
}

fn extract_join_info(_parsed: &ParsedContext) -> Result<SmallVec<[JoinInfo; 8]>, AnalysisError> {
    Ok(SmallVec::new())
}

fn estimate_cardinality(analyzed: &AnalyzedContext) -> Result<RowCount, EstimationError> {
    let table_count = analyzed.join_graph.table_count();

    if table_count == 0 {
        return Ok(RowCount::new(0));
    }

    let base_estimate: u64 = analyzed
        .table_stats
        .row_counts
        .values()
        .copied()
        .max()
        .unwrap_or(1000);

    Ok(RowCount::new(base_estimate))
}

fn estimate_query_cost(analyzed: &AnalyzedContext, estimated_rows: RowCount) -> QueryCost {
    let table_count = analyzed.join_graph.table_count() as u64;
    let row_estimate = estimated_rows.as_u64();

    QueryCost {
        scan_bytes: row_estimate.saturating_mul(100),
        estimated_memory_bytes: row_estimate.saturating_mul(200),
        parallelism_factor: (table_count as f32).max(1.0),
    }
}

fn detect_explosion_risks(analyzed: &AnalyzedContext) -> SmallVec<[ExplosionWarning; 4]> {
    let mut warnings = SmallVec::new();

    for join_info in &analyzed.join_infos {
        if join_info.cardinality == JoinCardinality::ManyToMany {
            let estimated_factor = 100.0;
            let severity = classify_explosion_severity(estimated_factor);

            warnings.push(ExplosionWarning {
                left_table: Arc::clone(&join_info.left_table),
                right_table: Arc::clone(&join_info.right_table),
                estimated_factor,
                severity,
            });
        }
    }

    warnings
}

fn classify_explosion_severity(factor: f64) -> WarningSeverity {
    if factor >= EXPLOSION_THRESHOLD_HIGH {
        return WarningSeverity::Critical;
    }

    if factor >= EXPLOSION_THRESHOLD_MEDIUM {
        return WarningSeverity::High;
    }

    if factor >= EXPLOSION_THRESHOLD_LOW {
        return WarningSeverity::Medium;
    }

    WarningSeverity::Low
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::states::Idle;

    #[test]
    fn test_parse_simple_select() {
        let pipeline = Pipeline::<Idle>::new("SELECT * FROM users");
        let parsed = pipeline.parse();
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_parse_empty_query() {
        let pipeline = Pipeline::<Idle>::new("");
        let result = pipeline.parse();
        assert!(matches!(
            result,
            Err(PipelineError::Parse(ParseError::EmptyQuery))
        ));
    }

    #[test]
    fn test_full_pipeline() {
        let pipeline = Pipeline::<Idle>::new("SELECT * FROM orders JOIN customers ON orders.customer_id = customers.id");
        let complete = pipeline.parse().and_then(|p| p.analyze()).and_then(|p| p.estimate());
        assert!(complete.is_ok());
    }
}
