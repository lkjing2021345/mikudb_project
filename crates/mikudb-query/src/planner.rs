use crate::ast::*;
use crate::{QueryError, QueryResult};

#[derive(Debug, Clone)]
pub struct QueryPlan {
    pub root: PlanNode,
    pub estimated_cost: f64,
}

#[derive(Debug, Clone)]
pub enum PlanNode {
    Scan {
        collection: String,
        filter: Option<Expression>,
    },
    IndexScan {
        collection: String,
        index_name: String,
        filter: Option<Expression>,
    },
    Filter {
        input: Box<PlanNode>,
        predicate: Expression,
    },
    Project {
        input: Box<PlanNode>,
        fields: Vec<String>,
    },
    Sort {
        input: Box<PlanNode>,
        fields: Vec<SortField>,
    },
    Limit {
        input: Box<PlanNode>,
        count: u64,
    },
    Skip {
        input: Box<PlanNode>,
        count: u64,
    },
    HashAggregate {
        input: Box<PlanNode>,
        group_by: Vec<String>,
        aggregates: Vec<Accumulator>,
    },
    NestedLoopJoin {
        left: Box<PlanNode>,
        right: Box<PlanNode>,
        condition: Option<Expression>,
    },
    Empty,
}

impl PlanNode {
    pub fn scan(collection: impl Into<String>) -> Self {
        PlanNode::Scan {
            collection: collection.into(),
            filter: None,
        }
    }

    pub fn with_filter(self, predicate: Expression) -> Self {
        PlanNode::Filter {
            input: Box::new(self),
            predicate,
        }
    }

    pub fn with_project(self, fields: Vec<String>) -> Self {
        PlanNode::Project {
            input: Box::new(self),
            fields,
        }
    }

    pub fn with_sort(self, fields: Vec<SortField>) -> Self {
        PlanNode::Sort {
            input: Box::new(self),
            fields,
        }
    }

    pub fn with_limit(self, count: u64) -> Self {
        PlanNode::Limit {
            input: Box::new(self),
            count,
        }
    }

    pub fn with_skip(self, count: u64) -> Self {
        PlanNode::Skip {
            input: Box::new(self),
            count,
        }
    }
}

pub struct QueryPlanner {
    use_index_optimization: bool,
    push_down_filters: bool,
}

impl Default for QueryPlanner {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryPlanner {
    pub fn new() -> Self {
        Self {
            use_index_optimization: true,
            push_down_filters: true,
        }
    }

    pub fn plan(&self, stmt: &Statement) -> QueryResult<QueryPlan> {
        match stmt {
            Statement::Find(find) => self.plan_find(find),
            Statement::Aggregate(agg) => self.plan_aggregate(agg),
            _ => Err(QueryError::Internal("Statement not supported for planning".to_string())),
        }
    }

    fn plan_find(&self, find: &FindStatement) -> QueryResult<QueryPlan> {
        let mut node = PlanNode::Scan {
            collection: find.collection.clone(),
            filter: None,
        };

        if let Some(filter) = &find.filter {
            if self.push_down_filters {
                node = PlanNode::Scan {
                    collection: find.collection.clone(),
                    filter: Some(filter.clone()),
                };
            } else {
                node = node.with_filter(filter.clone());
            }
        }

        if let Some(sort) = &find.sort {
            node = node.with_sort(sort.clone());
        }

        if let Some(skip) = find.skip {
            node = node.with_skip(skip);
        }

        if let Some(limit) = find.limit {
            node = node.with_limit(limit);
        }

        if let Some(projection) = &find.projection {
            node = node.with_project(projection.clone());
        }

        let cost = self.estimate_cost(&node);

        Ok(QueryPlan {
            root: node,
            estimated_cost: cost,
        })
    }

    fn plan_aggregate(&self, agg: &AggregateStatement) -> QueryResult<QueryPlan> {
        let mut node = PlanNode::Scan {
            collection: agg.collection.clone(),
            filter: None,
        };

        for stage in &agg.pipeline {
            node = match stage {
                AggregateStage::Match(expr) => {
                    if self.push_down_filters && matches!(node, PlanNode::Scan { .. }) {
                        if let PlanNode::Scan { collection, .. } = node {
                            PlanNode::Scan {
                                collection,
                                filter: Some(expr.clone()),
                            }
                        } else {
                            node.with_filter(expr.clone())
                        }
                    } else {
                        node.with_filter(expr.clone())
                    }
                }
                AggregateStage::Sort(fields) => node.with_sort(fields.clone()),
                AggregateStage::Limit(n) => node.with_limit(*n),
                AggregateStage::Skip(n) => node.with_skip(*n),
                AggregateStage::Project(fields) => {
                    let field_names: Vec<String> = fields.iter().map(|f| f.name.clone()).collect();
                    node.with_project(field_names)
                }
                AggregateStage::Group { by, accumulators } => PlanNode::HashAggregate {
                    input: Box::new(node),
                    group_by: by.clone(),
                    aggregates: accumulators.clone(),
                },
                _ => node,
            };
        }

        let cost = self.estimate_cost(&node);

        Ok(QueryPlan {
            root: node,
            estimated_cost: cost,
        })
    }

    fn estimate_cost(&self, node: &PlanNode) -> f64 {
        match node {
            PlanNode::Scan { filter, .. } => {
                let base_cost = 1000.0;
                if filter.is_some() {
                    base_cost * 0.5
                } else {
                    base_cost
                }
            }
            PlanNode::IndexScan { .. } => 10.0,
            PlanNode::Filter { input, .. } => self.estimate_cost(input) * 1.1,
            PlanNode::Project { input, .. } => self.estimate_cost(input) * 1.01,
            PlanNode::Sort { input, .. } => {
                let input_cost = self.estimate_cost(input);
                input_cost + input_cost.ln().max(1.0) * input_cost
            }
            PlanNode::Limit { input, count } => {
                self.estimate_cost(input).min(*count as f64)
            }
            PlanNode::Skip { input, count } => {
                self.estimate_cost(input) + *count as f64 * 0.1
            }
            PlanNode::HashAggregate { input, .. } => {
                self.estimate_cost(input) * 1.5
            }
            PlanNode::NestedLoopJoin { left, right, .. } => {
                self.estimate_cost(left) * self.estimate_cost(right)
            }
            PlanNode::Empty => 0.0,
        }
    }

    pub fn optimize(&self, plan: QueryPlan) -> QueryPlan {
        let optimized = self.apply_optimizations(plan.root);
        let cost = self.estimate_cost(&optimized);
        QueryPlan {
            root: optimized,
            estimated_cost: cost,
        }
    }

    fn apply_optimizations(&self, node: PlanNode) -> PlanNode {
        let node = self.push_down_limit(node);
        let node = self.merge_consecutive_filters(node);
        node
    }

    fn push_down_limit(&self, node: PlanNode) -> PlanNode {
        match node {
            PlanNode::Limit {
                input,
                count: limit_count,
            } => {
                match *input {
                    PlanNode::Sort { input: sort_input, fields } => {
                        PlanNode::Limit {
                            input: Box::new(PlanNode::Sort {
                                input: sort_input,
                                fields,
                            }),
                            count: limit_count,
                        }
                    }
                    other => PlanNode::Limit {
                        input: Box::new(self.push_down_limit(other)),
                        count: limit_count,
                    },
                }
            }
            PlanNode::Filter { input, predicate } => PlanNode::Filter {
                input: Box::new(self.push_down_limit(*input)),
                predicate,
            },
            PlanNode::Project { input, fields } => PlanNode::Project {
                input: Box::new(self.push_down_limit(*input)),
                fields,
            },
            PlanNode::Sort { input, fields } => PlanNode::Sort {
                input: Box::new(self.push_down_limit(*input)),
                fields,
            },
            other => other,
        }
    }

    fn merge_consecutive_filters(&self, node: PlanNode) -> PlanNode {
        match node {
            PlanNode::Filter {
                input,
                predicate: outer_pred,
            } => {
                let optimized_input = self.merge_consecutive_filters(*input);
                match optimized_input {
                    PlanNode::Filter {
                        input: inner_input,
                        predicate: inner_pred,
                    } => {
                        let merged = Expression::and(inner_pred, outer_pred);
                        PlanNode::Filter {
                            input: inner_input,
                            predicate: merged,
                        }
                    }
                    other => PlanNode::Filter {
                        input: Box::new(other),
                        predicate: outer_pred,
                    },
                }
            }
            PlanNode::Project { input, fields } => PlanNode::Project {
                input: Box::new(self.merge_consecutive_filters(*input)),
                fields,
            },
            PlanNode::Sort { input, fields } => PlanNode::Sort {
                input: Box::new(self.merge_consecutive_filters(*input)),
                fields,
            },
            PlanNode::Limit { input, count } => PlanNode::Limit {
                input: Box::new(self.merge_consecutive_filters(*input)),
                count,
            },
            PlanNode::Skip { input, count } => PlanNode::Skip {
                input: Box::new(self.merge_consecutive_filters(*input)),
                count,
            },
            other => other,
        }
    }
}

impl std::fmt::Display for PlanNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.format(f, 0)
    }
}

impl PlanNode {
    fn format(&self, f: &mut std::fmt::Formatter<'_>, indent: usize) -> std::fmt::Result {
        let prefix = "  ".repeat(indent);
        match self {
            PlanNode::Scan { collection, filter } => {
                write!(f, "{}Scan({})", prefix, collection)?;
                if let Some(flt) = filter {
                    write!(f, " [filter]")?;
                }
                Ok(())
            }
            PlanNode::IndexScan { collection, index_name, .. } => {
                write!(f, "{}IndexScan({}, {})", prefix, collection, index_name)
            }
            PlanNode::Filter { input, .. } => {
                writeln!(f, "{}Filter", prefix)?;
                input.format(f, indent + 1)
            }
            PlanNode::Project { input, fields } => {
                writeln!(f, "{}Project({:?})", prefix, fields)?;
                input.format(f, indent + 1)
            }
            PlanNode::Sort { input, fields } => {
                writeln!(f, "{}Sort({:?})", prefix, fields.iter().map(|s| &s.field).collect::<Vec<_>>())?;
                input.format(f, indent + 1)
            }
            PlanNode::Limit { input, count } => {
                writeln!(f, "{}Limit({})", prefix, count)?;
                input.format(f, indent + 1)
            }
            PlanNode::Skip { input, count } => {
                writeln!(f, "{}Skip({})", prefix, count)?;
                input.format(f, indent + 1)
            }
            PlanNode::HashAggregate { input, group_by, .. } => {
                writeln!(f, "{}HashAggregate(by: {:?})", prefix, group_by)?;
                input.format(f, indent + 1)
            }
            PlanNode::NestedLoopJoin { left, right, .. } => {
                writeln!(f, "{}NestedLoopJoin", prefix)?;
                left.format(f, indent + 1)?;
                right.format(f, indent + 1)
            }
            PlanNode::Empty => write!(f, "{}Empty", prefix),
        }
    }
}
