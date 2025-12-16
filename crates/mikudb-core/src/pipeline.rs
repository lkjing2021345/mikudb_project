//! 聚合管道构建器模块
//!
//! 提供流式 API 构建聚合管道查询。
//!
//! # 示例
//!
//! ```rust,ignore
//! use mikudb_core::Pipeline;
//!
//! let pipeline = Pipeline::new()
//!     .match_expr(|m| m.field("status").eq("active"))
//!     .group(|g| g.by("category").count("total").sum("amount", "total_amount"))
//!     .sort(|s| s.desc("total"))
//!     .limit(10);
//!
//! let results = collection.aggregate(pipeline).await?;
//! ```

use crate::boml::BomlValue;
use crate::query::{
    AggregateStage, AggregateFunction, Accumulator, Expression, SortField, SortOrder,
    ProjectField, BinaryOp, UnaryOp,
};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct Pipeline {
    stages: Vec<AggregateStage>,
}

impl Pipeline {
    pub fn new() -> Self {
        Self { stages: vec![] }
    }

    pub fn stages(&self) -> &[AggregateStage] {
        &self.stages
    }

    pub fn into_stages(self) -> Vec<AggregateStage> {
        self.stages
    }

    pub fn match_filter(mut self, expr: Expression) -> Self {
        self.stages.push(AggregateStage::Match(expr));
        self
    }

    pub fn match_expr<F>(self, f: F) -> Self
    where
        F: FnOnce(MatchBuilder) -> MatchBuilder,
    {
        let builder = f(MatchBuilder::new());
        self.match_filter(builder.build())
    }

    pub fn project(mut self, fields: Vec<ProjectField>) -> Self {
        self.stages.push(AggregateStage::Project(fields));
        self
    }

    pub fn project_fields<F>(self, f: F) -> Self
    where
        F: FnOnce(ProjectBuilder) -> ProjectBuilder,
    {
        let builder = f(ProjectBuilder::new());
        self.project(builder.build())
    }

    pub fn group(mut self, by: Vec<String>, accumulators: Vec<Accumulator>) -> Self {
        self.stages.push(AggregateStage::Group { by, accumulators });
        self
    }

    pub fn group_by<F>(self, f: F) -> Self
    where
        F: FnOnce(GroupBuilder) -> GroupBuilder,
    {
        let builder = f(GroupBuilder::new());
        let (by, accumulators) = builder.build();
        self.group(by, accumulators)
    }

    pub fn sort(mut self, fields: Vec<SortField>) -> Self {
        self.stages.push(AggregateStage::Sort(fields));
        self
    }

    pub fn sort_by<F>(self, f: F) -> Self
    where
        F: FnOnce(SortBuilder) -> SortBuilder,
    {
        let builder = f(SortBuilder::new());
        self.sort(builder.build())
    }

    pub fn limit(mut self, n: u64) -> Self {
        self.stages.push(AggregateStage::Limit(n));
        self
    }

    pub fn skip(mut self, n: u64) -> Self {
        self.stages.push(AggregateStage::Skip(n));
        self
    }

    pub fn count(mut self, field_name: impl Into<String>) -> Self {
        self.stages.push(AggregateStage::Count(field_name.into()));
        self
    }

    pub fn unwind(mut self, path: impl Into<String>) -> Self {
        self.stages.push(AggregateStage::Unwind {
            path: path.into(),
            preserve_null: false,
        });
        self
    }

    pub fn unwind_preserve_null(mut self, path: impl Into<String>) -> Self {
        self.stages.push(AggregateStage::Unwind {
            path: path.into(),
            preserve_null: true,
        });
        self
    }

    pub fn lookup<F>(self, f: F) -> Self
    where
        F: FnOnce(LookupBuilder) -> LookupBuilder,
    {
        let builder = f(LookupBuilder::new());
        self.add_lookup(builder.build())
    }

    fn add_lookup(mut self, lookup: AggregateStage) -> Self {
        self.stages.push(lookup);
        self
    }

    pub fn then(mut self, stage: AggregateStage) -> Self {
        self.stages.push(stage);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.stages.is_empty()
    }

    pub fn len(&self) -> usize {
        self.stages.len()
    }
}

#[derive(Debug, Clone)]
pub struct MatchBuilder {
    conditions: Vec<Expression>,
}

impl MatchBuilder {
    pub fn new() -> Self {
        Self { conditions: vec![] }
    }

    pub fn field(self, name: &str) -> FieldMatcher {
        FieldMatcher {
            builder: self,
            field_name: name.to_string(),
        }
    }

    pub fn and<F>(mut self, f: F) -> Self
    where
        F: FnOnce(MatchBuilder) -> MatchBuilder,
    {
        let inner = f(MatchBuilder::new());
        let expr = inner.build();
        self.conditions.push(expr);
        self
    }

    pub fn or(self, left: Expression, right: Expression) -> Self {
        self.expr(Expression::or(left, right))
    }

    pub fn not(self, expr: Expression) -> Self {
        self.expr(Expression::not(expr))
    }

    pub fn expr(mut self, expr: Expression) -> Self {
        self.conditions.push(expr);
        self
    }

    pub fn build(self) -> Expression {
        if self.conditions.is_empty() {
            Expression::Literal(BomlValue::Boolean(true))
        } else if self.conditions.len() == 1 {
            self.conditions.into_iter().next().unwrap()
        } else {
            self.conditions
                .into_iter()
                .reduce(|acc, expr| Expression::and(acc, expr))
                .unwrap()
        }
    }
}

impl Default for MatchBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct FieldMatcher {
    builder: MatchBuilder,
    field_name: String,
}

impl FieldMatcher {
    pub fn eq(mut self, value: impl Into<BomlValue>) -> MatchBuilder {
        self.builder.conditions.push(Expression::eq(
            Expression::Field(self.field_name),
            Expression::Literal(value.into()),
        ));
        self.builder
    }

    pub fn ne(mut self, value: impl Into<BomlValue>) -> MatchBuilder {
        self.builder.conditions.push(Expression::ne(
            Expression::Field(self.field_name),
            Expression::Literal(value.into()),
        ));
        self.builder
    }

    pub fn gt(mut self, value: impl Into<BomlValue>) -> MatchBuilder {
        self.builder.conditions.push(Expression::gt(
            Expression::Field(self.field_name),
            Expression::Literal(value.into()),
        ));
        self.builder
    }

    pub fn gte(mut self, value: impl Into<BomlValue>) -> MatchBuilder {
        self.builder.conditions.push(Expression::ge(
            Expression::Field(self.field_name),
            Expression::Literal(value.into()),
        ));
        self.builder
    }

    pub fn lt(mut self, value: impl Into<BomlValue>) -> MatchBuilder {
        self.builder.conditions.push(Expression::lt(
            Expression::Field(self.field_name),
            Expression::Literal(value.into()),
        ));
        self.builder
    }

    pub fn lte(mut self, value: impl Into<BomlValue>) -> MatchBuilder {
        self.builder.conditions.push(Expression::le(
            Expression::Field(self.field_name),
            Expression::Literal(value.into()),
        ));
        self.builder
    }

    pub fn in_values(mut self, values: Vec<BomlValue>) -> MatchBuilder {
        let exprs: Vec<Expression> = values.into_iter().map(Expression::Literal).collect();
        self.builder.conditions.push(Expression::In {
            expr: Box::new(Expression::Field(self.field_name)),
            list: exprs,
        });
        self.builder
    }

    pub fn exists(mut self, exists: bool) -> MatchBuilder {
        self.builder.conditions.push(Expression::Exists {
            field: self.field_name,
            negated: !exists,
        });
        self.builder
    }

    pub fn like(mut self, pattern: impl Into<String>) -> MatchBuilder {
        self.builder.conditions.push(Expression::Like {
            expr: Box::new(Expression::Field(self.field_name)),
            pattern: pattern.into(),
        });
        self.builder
    }

    pub fn is_null(mut self) -> MatchBuilder {
        self.builder.conditions.push(Expression::IsNull {
            expr: Box::new(Expression::Field(self.field_name)),
            negated: false,
        });
        self.builder
    }

    pub fn is_not_null(mut self) -> MatchBuilder {
        self.builder.conditions.push(Expression::IsNull {
            expr: Box::new(Expression::Field(self.field_name)),
            negated: true,
        });
        self.builder
    }

    pub fn between(mut self, low: impl Into<BomlValue>, high: impl Into<BomlValue>) -> MatchBuilder {
        self.builder.conditions.push(Expression::Between {
            expr: Box::new(Expression::Field(self.field_name)),
            low: Box::new(Expression::Literal(low.into())),
            high: Box::new(Expression::Literal(high.into())),
        });
        self.builder
    }
}

#[derive(Debug, Clone, Default)]
pub struct GroupBuilder {
    by_fields: Vec<String>,
    accumulators: Vec<Accumulator>,
}

impl GroupBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn by(mut self, field: impl Into<String>) -> Self {
        self.by_fields.push(field.into());
        self
    }

    pub fn by_fields(mut self, fields: Vec<String>) -> Self {
        self.by_fields.extend(fields);
        self
    }

    pub fn count(mut self, name: impl Into<String>) -> Self {
        self.accumulators.push(Accumulator {
            name: name.into(),
            function: AggregateFunction::Count,
            field: None,
        });
        self
    }

    pub fn sum(mut self, field: impl Into<String>, name: impl Into<String>) -> Self {
        self.accumulators.push(Accumulator {
            name: name.into(),
            function: AggregateFunction::Sum,
            field: Some(field.into()),
        });
        self
    }

    pub fn avg(mut self, field: impl Into<String>, name: impl Into<String>) -> Self {
        self.accumulators.push(Accumulator {
            name: name.into(),
            function: AggregateFunction::Avg,
            field: Some(field.into()),
        });
        self
    }

    pub fn min(mut self, field: impl Into<String>, name: impl Into<String>) -> Self {
        self.accumulators.push(Accumulator {
            name: name.into(),
            function: AggregateFunction::Min,
            field: Some(field.into()),
        });
        self
    }

    pub fn max(mut self, field: impl Into<String>, name: impl Into<String>) -> Self {
        self.accumulators.push(Accumulator {
            name: name.into(),
            function: AggregateFunction::Max,
            field: Some(field.into()),
        });
        self
    }

    pub fn first(mut self, field: impl Into<String>, name: impl Into<String>) -> Self {
        self.accumulators.push(Accumulator {
            name: name.into(),
            function: AggregateFunction::First,
            field: Some(field.into()),
        });
        self
    }

    pub fn last(mut self, field: impl Into<String>, name: impl Into<String>) -> Self {
        self.accumulators.push(Accumulator {
            name: name.into(),
            function: AggregateFunction::Last,
            field: Some(field.into()),
        });
        self
    }

    pub fn push(mut self, field: impl Into<String>, name: impl Into<String>) -> Self {
        self.accumulators.push(Accumulator {
            name: name.into(),
            function: AggregateFunction::Push,
            field: Some(field.into()),
        });
        self
    }

    pub fn add_to_set(mut self, field: impl Into<String>, name: impl Into<String>) -> Self {
        self.accumulators.push(Accumulator {
            name: name.into(),
            function: AggregateFunction::AddToSet,
            field: Some(field.into()),
        });
        self
    }

    pub fn build(self) -> (Vec<String>, Vec<Accumulator>) {
        (self.by_fields, self.accumulators)
    }
}

#[derive(Debug, Clone, Default)]
pub struct SortBuilder {
    fields: Vec<SortField>,
}

impl SortBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn asc(mut self, field: impl Into<String>) -> Self {
        self.fields.push(SortField {
            field: field.into(),
            order: SortOrder::Ascending,
        });
        self
    }

    pub fn desc(mut self, field: impl Into<String>) -> Self {
        self.fields.push(SortField {
            field: field.into(),
            order: SortOrder::Descending,
        });
        self
    }

    pub fn field(mut self, field: impl Into<String>, order: SortOrder) -> Self {
        self.fields.push(SortField {
            field: field.into(),
            order,
        });
        self
    }

    pub fn build(self) -> Vec<SortField> {
        self.fields
    }
}

#[derive(Debug, Clone, Default)]
pub struct ProjectBuilder {
    fields: Vec<ProjectField>,
}

impl ProjectBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn include(mut self, name: impl Into<String>) -> Self {
        self.fields.push(ProjectField {
            name: name.into(),
            expression: None,
            include: true,
        });
        self
    }

    pub fn exclude(mut self, name: impl Into<String>) -> Self {
        self.fields.push(ProjectField {
            name: name.into(),
            expression: None,
            include: false,
        });
        self
    }

    pub fn computed(mut self, name: impl Into<String>, expr: Expression) -> Self {
        self.fields.push(ProjectField {
            name: name.into(),
            expression: Some(expr),
            include: true,
        });
        self
    }

    pub fn rename(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.fields.push(ProjectField {
            name: to.into(),
            expression: Some(Expression::Field(from.into())),
            include: true,
        });
        self
    }

    pub fn build(self) -> Vec<ProjectField> {
        self.fields
    }
}

#[derive(Debug, Clone)]
pub struct LookupBuilder {
    from: String,
    local_field: String,
    foreign_field: String,
    as_field: String,
}

impl LookupBuilder {
    pub fn new() -> Self {
        Self {
            from: String::new(),
            local_field: String::new(),
            foreign_field: String::new(),
            as_field: String::new(),
        }
    }

    pub fn from(mut self, collection: impl Into<String>) -> Self {
        self.from = collection.into();
        self
    }

    pub fn local_field(mut self, field: impl Into<String>) -> Self {
        self.local_field = field.into();
        self
    }

    pub fn foreign_field(mut self, field: impl Into<String>) -> Self {
        self.foreign_field = field.into();
        self
    }

    pub fn as_field(mut self, name: impl Into<String>) -> Self {
        self.as_field = name.into();
        self
    }

    pub fn build(self) -> AggregateStage {
        AggregateStage::Lookup {
            from: self.from,
            local_field: self.local_field,
            foreign_field: self.foreign_field,
            as_field: self.as_field,
        }
    }
}

impl Default for LookupBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub fn field(name: impl Into<String>) -> Expression {
    Expression::Field(name.into())
}

pub fn literal(value: impl Into<BomlValue>) -> Expression {
    Expression::Literal(value.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_basic() {
        let pipeline = Pipeline::new()
            .limit(10)
            .skip(5);

        assert_eq!(pipeline.stages().len(), 2);
    }

    #[test]
    fn test_pipeline_match() {
        let pipeline = Pipeline::new()
            .match_expr(|m| m.field("status").eq("active"));

        assert_eq!(pipeline.stages().len(), 1);
    }

    #[test]
    fn test_pipeline_group() {
        let pipeline = Pipeline::new()
            .group_by(|g| {
                g.by("category")
                    .count("total")
                    .sum("amount", "total_amount")
            });

        assert_eq!(pipeline.stages().len(), 1);
    }

    #[test]
    fn test_pipeline_sort() {
        let pipeline = Pipeline::new()
            .sort_by(|s| s.desc("created_at").asc("name"));

        assert_eq!(pipeline.stages().len(), 1);
    }

    #[test]
    fn test_pipeline_project() {
        let pipeline = Pipeline::new()
            .project_fields(|p| {
                p.include("name")
                    .include("email")
                    .exclude("password")
            });

        assert_eq!(pipeline.stages().len(), 1);
    }

    #[test]
    fn test_complex_pipeline() {
        let pipeline = Pipeline::new()
            .match_expr(|m| m.field("status").eq("active"))
            .group_by(|g| g.by("category").count("count").avg("price", "avg_price"))
            .sort_by(|s| s.desc("count"))
            .limit(10);

        assert_eq!(pipeline.stages().len(), 4);
    }

    #[test]
    fn test_match_builder_multiple_conditions() {
        let pipeline = Pipeline::new()
            .match_expr(|m| {
                m.field("age").gte(18)
                    .field("status").eq("active")
            });

        assert_eq!(pipeline.stages().len(), 1);
    }

    #[test]
    fn test_lookup_builder() {
        let pipeline = Pipeline::new()
            .lookup(|l| {
                l.from("orders")
                    .local_field("_id")
                    .foreign_field("user_id")
                    .as_field("user_orders")
            });

        assert_eq!(pipeline.stages().len(), 1);
    }
}
