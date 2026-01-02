//! 查询计划器模块
//!
//! 本模块实现查询优化和执行计划生成:
//! - 将 MQL 语句转换为执行计划树
//! - 查询优化:过滤器下推、连续过滤器合并、LIMIT 下推
//! - 成本估算:估算执行计划的代价
//! - 索引选择(待实现)
//!
//! 执行计划节点类型:
//! - Scan: 全表扫描
//! - IndexScan: 索引扫描
//! - Filter: 过滤
//! - Project: 投影
//! - Sort: 排序
//! - Limit/Skip: 分页
//! - HashAggregate: 哈希聚合
//! - NestedLoopJoin: 嵌套循环连接

use crate::ast::*;
use crate::{QueryError, QueryResult};

/// 查询执行计划
///
/// 包含执行计划树和估算的执行代价。
#[derive(Debug, Clone)]
pub struct QueryPlan {
    /// 执行计划的根节点
    pub root: PlanNode,
    /// 估算的执行代价
    pub estimated_cost: f64,
}

/// 执行计划节点
///
/// 表示查询执行计划树的节点,每个节点代表一种关系代数操作。
#[derive(Debug, Clone)]
pub enum PlanNode {
    /// 全表扫描
    Scan {
        /// 集合名称
        collection: String,
        /// 可选的下推过滤器
        filter: Option<Expression>,
    },
    /// 索引扫描
    IndexScan {
        /// 集合名称
        collection: String,
        /// 索引名称
        index_name: String,
        /// 可选的过滤器
        filter: Option<Expression>,
    },
    /// 过滤器
    Filter {
        /// 输入节点
        input: Box<PlanNode>,
        /// 过滤谓词
        predicate: Expression,
    },
    /// 投影(选择列)
    Project {
        /// 输入节点
        input: Box<PlanNode>,
        /// 投影字段列表
        fields: Vec<String>,
    },
    /// 排序
    Sort {
        /// 输入节点
        input: Box<PlanNode>,
        /// 排序字段列表
        fields: Vec<SortField>,
    },
    /// 限制返回数量
    Limit {
        /// 输入节点
        input: Box<PlanNode>,
        /// 限制数量
        count: u64,
    },
    /// 跳过记录数
    Skip {
        /// 输入节点
        input: Box<PlanNode>,
        /// 跳过数量
        count: u64,
    },
    /// 哈希聚合
    HashAggregate {
        /// 输入节点
        input: Box<PlanNode>,
        /// 分组字段
        group_by: Vec<String>,
        /// 聚合函数列表
        aggregates: Vec<Accumulator>,
    },
    /// 嵌套循环连接
    NestedLoopJoin {
        /// 左输入节点
        left: Box<PlanNode>,
        /// 右输入节点
        right: Box<PlanNode>,
        /// 连接条件
        condition: Option<Expression>,
    },
    /// 空节点
    Empty,
}

impl PlanNode {
    /// # Brief
    /// 创建全表扫描节点
    ///
    /// # Arguments
    /// * `collection` - 集合名称
    pub fn scan(collection: impl Into<String>) -> Self {
        PlanNode::Scan {
            collection: collection.into(),
            filter: None,
        }
    }

    /// # Brief
    /// 添加过滤器节点
    ///
    /// 在当前节点上添加一个 Filter 节点。
    ///
    /// # Arguments
    /// * `predicate` - 过滤谓词
    pub fn with_filter(self, predicate: Expression) -> Self {
        PlanNode::Filter {
            input: Box::new(self),
            predicate,
        }
    }

    /// # Brief
    /// 添加投影节点
    ///
    /// # Arguments
    /// * `fields` - 投影字段列表
    pub fn with_project(self, fields: Vec<String>) -> Self {
        PlanNode::Project {
            input: Box::new(self),
            fields,
        }
    }

    /// # Brief
    /// 添加排序节点
    ///
    /// # Arguments
    /// * `fields` - 排序字段列表
    pub fn with_sort(self, fields: Vec<SortField>) -> Self {
        PlanNode::Sort {
            input: Box::new(self),
            fields,
        }
    }

    /// # Brief
    /// 添加 LIMIT 节点
    ///
    /// # Arguments
    /// * `count` - 限制数量
    pub fn with_limit(self, count: u64) -> Self {
        PlanNode::Limit {
            input: Box::new(self),
            count,
        }
    }

    /// # Brief
    /// 添加 SKIP 节点
    ///
    /// # Arguments
    /// * `count` - 跳过数量
    pub fn with_skip(self, count: u64) -> Self {
        PlanNode::Skip {
            input: Box::new(self),
            count,
        }
    }
}

/// 查询计划器
///
/// 负责将 MQL 语句转换为优化的执行计划。
pub struct QueryPlanner {
    /// 是否启用索引优化
    use_index_optimization: bool,
    /// 是否启用过滤器下推优化
    push_down_filters: bool,
}

impl Default for QueryPlanner {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryPlanner {
    /// # Brief
    /// 创建查询计划器
    ///
    /// 默认启用所有优化选项。
    pub fn new() -> Self {
        Self {
            use_index_optimization: true,
            push_down_filters: true,
        }
    }

    /// # Brief
    /// 为语句生成执行计划
    ///
    /// # Arguments
    /// * `stmt` - MQL 语句
    ///
    /// # Returns
    /// 执行计划(包含成本估算)
    pub fn plan(&self, stmt: &Statement) -> QueryResult<QueryPlan> {
        match stmt {
            Statement::Find(find) => self.plan_find(find),
            Statement::Aggregate(agg) => self.plan_aggregate(agg),
            _ => Err(QueryError::Internal("Statement not supported for planning".to_string())),
        }
    }

    /// # Brief
    /// 为 FIND 语句生成执行计划
    ///
    /// 执行计划构建顺序:
    /// 1. Scan 节点(带下推过滤器)
    /// 2. Sort 节点
    /// 3. Skip 节点
    /// 4. Limit 节点
    /// 5. Project 节点
    ///
    /// # Arguments
    /// * `find` - FIND 语句
    ///
    /// # Returns
    /// 执行计划
    fn plan_find(&self, find: &FindStatement) -> QueryResult<QueryPlan> {
        let mut node = PlanNode::Scan {
            collection: find.collection.clone(),
            filter: None,
        };

        // 过滤器下推优化:将过滤条件下推到 Scan 节点
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

        // 添加排序节点
        if let Some(sort) = &find.sort {
            node = node.with_sort(sort.clone());
        }

        // 添加分页节点
        if let Some(skip) = find.skip {
            node = node.with_skip(skip);
        }

        if let Some(limit) = find.limit {
            node = node.with_limit(limit);
        }

        // 添加投影节点(最后执行)
        if let Some(projection) = &find.projection {
            node = node.with_project(projection.clone());
        }

        let cost = self.estimate_cost(&node);

        Ok(QueryPlan {
            root: node,
            estimated_cost: cost,
        })
    }

    /// # Brief
    /// 为 AGGREGATE 语句生成执行计划
    ///
    /// 按照管道阶段顺序构建执行计划树,支持过滤器下推。
    ///
    /// # Arguments
    /// * `agg` - AGGREGATE 语句
    ///
    /// # Returns
    /// 执行计划
    fn plan_aggregate(&self, agg: &AggregateStatement) -> QueryResult<QueryPlan> {
        let mut node = PlanNode::Scan {
            collection: agg.collection.clone(),
            filter: None,
        };

        // 按顺序应用聚合管道阶段
        for stage in &agg.pipeline {
            node = match stage {
                // MATCH 阶段:如果是第一个阶段,下推到 Scan
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
                // GROUP BY 阶段:创建 HashAggregate 节点
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

    /// # Brief
    /// 估算执行计划的代价
    ///
    /// 代价模型:
    /// - Scan: 1000.0 (带过滤器 * 0.5)
    /// - IndexScan: 10.0
    /// - Filter: 输入代价 * 1.1
    /// - Project: 输入代价 * 1.01
    /// - Sort: 输入代价 + 输入代价 * ln(输入代价) (快速排序复杂度)
    /// - Limit: min(输入代价, limit)
    /// - Skip: 输入代价 + skip * 0.1
    /// - HashAggregate: 输入代价 * 1.5
    /// - NestedLoopJoin: 左代价 * 右代价
    ///
    /// # Arguments
    /// * `node` - 执行计划节点
    ///
    /// # Returns
    /// 估算代价
    fn estimate_cost(&self, node: &PlanNode) -> f64 {
        match node {
            PlanNode::Scan { filter, .. } => {
                let base_cost = 1000.0;
                // 带过滤器的扫描估算成本减半
                if filter.is_some() {
                    base_cost * 0.5
                } else {
                    base_cost
                }
            }
            PlanNode::IndexScan { .. } => 10.0,
            PlanNode::Filter { input, .. } => self.estimate_cost(input) * 1.1,
            PlanNode::Project { input, .. } => self.estimate_cost(input) * 1.01,
            // 排序代价:O(n log n)
            PlanNode::Sort { input, .. } => {
                let input_cost = self.estimate_cost(input);
                input_cost + input_cost.ln().max(1.0) * input_cost
            }
            // LIMIT 可以显著减少输出
            PlanNode::Limit { input, count } => {
                self.estimate_cost(input).min(*count as f64)
            }
            PlanNode::Skip { input, count } => {
                self.estimate_cost(input) + *count as f64 * 0.1
            }
            PlanNode::HashAggregate { input, .. } => {
                self.estimate_cost(input) * 1.5
            }
            // 嵌套循环连接:O(m * n)
            PlanNode::NestedLoopJoin { left, right, .. } => {
                self.estimate_cost(left) * self.estimate_cost(right)
            }
            PlanNode::Empty => 0.0,
        }
    }

    /// # Brief
    /// 优化执行计划
    ///
    /// 应用优化规则:
    /// - LIMIT 下推
    /// - 连续 Filter 合并
    ///
    /// # Arguments
    /// * `plan` - 原始执行计划
    ///
    /// # Returns
    /// 优化后的执行计划
    pub fn optimize(&self, plan: QueryPlan) -> QueryPlan {
        let optimized = self.apply_optimizations(plan.root);
        let cost = self.estimate_cost(&optimized);
        QueryPlan {
            root: optimized,
            estimated_cost: cost,
        }
    }

    /// # Brief
    /// 应用所有优化规则
    ///
    /// # Arguments
    /// * `node` - 执行计划节点
    ///
    /// # Returns
    /// 优化后的节点
    fn apply_optimizations(&self, node: PlanNode) -> PlanNode {
        let node = self.push_down_limit(node);
        let node = self.merge_consecutive_filters(node);
        node
    }

    /// # Brief
    /// LIMIT 下推优化
    ///
    /// 将 LIMIT 尽可能下推到计划树的底部,减少中间结果集大小。
    /// 特殊处理:LIMIT 不能下推到 Sort 之下(需要完整数据集排序)。
    ///
    /// # Arguments
    /// * `node` - 执行计划节点
    ///
    /// # Returns
    /// 优化后的节点
    fn push_down_limit(&self, node: PlanNode) -> PlanNode {
        match node {
            PlanNode::Limit {
                input,
                count: limit_count,
            } => {
                match *input {
                    // 不能将 LIMIT 下推到 Sort 之下
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
            // 递归处理其他节点
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

    /// # Brief
    /// 合并连续的 Filter 节点
    ///
    /// 将多个 Filter 节点合并为一个 Filter,使用 AND 连接。
    /// 优化: Filter(Filter(input, pred1), pred2) => Filter(input, pred1 AND pred2)
    ///
    /// # Arguments
    /// * `node` - 执行计划节点
    ///
    /// # Returns
    /// 优化后的节点
    fn merge_consecutive_filters(&self, node: PlanNode) -> PlanNode {
        match node {
            PlanNode::Filter {
                input,
                predicate: outer_pred,
            } => {
                let optimized_input = self.merge_consecutive_filters(*input);
                match optimized_input {
                    // 发现内层也是 Filter,合并
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
            // 递归处理其他节点
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
    /// # Brief
    /// 格式化执行计划树为缩进字符串
    ///
    /// 用于可视化执行计划。
    ///
    /// # Arguments
    /// * `f` - 格式化器
    /// * `indent` - 缩进级别
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
