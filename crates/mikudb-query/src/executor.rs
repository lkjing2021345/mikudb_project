//! 查询执行器模块
//!
//! 负责执行解析后的 MQL 语句，包括 CRUD 操作、聚合查询等。

use crate::ast::*;
use crate::filter;
use crate::planner::{PlanNode, QueryPlan, QueryPlanner};
use crate::{QueryError, QueryResult};
use mikudb_boml::{BomlValue, Document};
use mikudb_storage::{Collection, StorageEngine};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, trace};

/// 查询执行器
///
/// 负责执行已解析的 MQL 语句
pub struct QueryExecutor {
    storage: Arc<StorageEngine>,
    planner: QueryPlanner,
}

impl QueryExecutor {
    /// 创建新的查询执行器
    ///
    /// # Brief
    /// 创建一个与存储引擎绑定的查询执行器
    ///
    /// # Arguments
    /// * `storage` - 存储引擎的 Arc 引用
    ///
    /// # Returns
    /// 新的 QueryExecutor 实例
    pub fn new(storage: Arc<StorageEngine>) -> Self {
        Self {
            storage,
            planner: QueryPlanner::new(),
        }
    }

    /// 执行语句
    ///
    /// # Brief
    /// 执行已解析的 MQL 语句并返回结果
    ///
    /// # Arguments
    /// * `stmt` - 要执行的 Statement
    ///
    /// # Returns
    /// 执行结果 QueryResponse，或错误
    pub fn execute(&self, stmt: &Statement) -> QueryResult<QueryResponse> {
        match stmt {
            Statement::Use(use_stmt) => {
                Ok(QueryResponse::Ok {
                    message: format!("Switched to database: {}", use_stmt.database),
                })
            }

            Statement::ShowDatabases => {
                Ok(QueryResponse::Databases(vec!["default".to_string()]))
            }

            Statement::ShowCollections => {
                let collections = self.storage.list_collections()?;
                Ok(QueryResponse::Collections(collections))
            }

            Statement::ShowIndexes(collection) => {
                Ok(QueryResponse::Indexes(vec![]))
            }

            Statement::ShowStatus => {
                let size = self.storage.get_approximate_size();
                let stats = self.storage.get_statistics();
                Ok(QueryResponse::Status {
                    size,
                    stats: stats.unwrap_or_default(),
                })
            }

            Statement::CreateDatabase(name) => {
                Ok(QueryResponse::Ok {
                    message: format!("Created database: {}", name),
                })
            }

            Statement::DropDatabase(name) => {
                Ok(QueryResponse::Ok {
                    message: format!("Dropped database: {}", name),
                })
            }

            Statement::CreateCollection(name) => {
                self.storage.create_collection(name)?;
                Ok(QueryResponse::Ok {
                    message: format!("Created collection: {}", name),
                })
            }

            Statement::DropCollection(name) => {
                self.storage.drop_collection(name)?;
                Ok(QueryResponse::Ok {
                    message: format!("Dropped collection: {}", name),
                })
            }

            Statement::CreateIndex(create_idx) => {
                Ok(QueryResponse::Ok {
                    message: format!("Created index: {}", create_idx.name),
                })
            }

            Statement::DropIndex(drop_idx) => {
                Ok(QueryResponse::Ok {
                    message: format!("Dropped index: {}", drop_idx.name),
                })
            }

            Statement::Insert(insert) => self.execute_insert(insert),
            Statement::Find(find) => self.execute_find(find),
            Statement::Update(update) => self.execute_update(update),
            Statement::Delete(delete) => self.execute_delete(delete),
            Statement::Aggregate(agg) => self.execute_aggregate(agg),

            Statement::BeginTransaction => {
                Ok(QueryResponse::Ok {
                    message: "Transaction started".to_string(),
                })
            }

            Statement::Commit => {
                Ok(QueryResponse::Ok {
                    message: "Transaction committed".to_string(),
                })
            }

            Statement::Rollback => {
                Ok(QueryResponse::Ok {
                    message: "Transaction rolled back".to_string(),
                })
            }

            _ => Err(QueryError::Internal("Not implemented".to_string())),
        }
    }

    fn execute_insert(&self, insert: &InsertStatement) -> QueryResult<QueryResponse> {
        let collection = self.storage.get_or_create_collection(&insert.collection)?;

        let mut inserted_ids = Vec::new();
        for doc_value in &insert.documents {
            let mut doc = Document::from_boml_value(doc_value.clone())?;
            let id = collection.insert(&mut doc)?;
            inserted_ids.push(id.to_string());
        }

        Ok(QueryResponse::Insert {
            inserted_count: inserted_ids.len() as u64,
            inserted_ids,
        })
    }

    fn execute_find(&self, find: &FindStatement) -> QueryResult<QueryResponse> {
        let collection = self.storage.get_collection(&find.collection)?;

        let mut docs = collection.find_all()?;

        if let Some(filter_expr) = &find.filter {
            let filter = filter::Filter::new(filter_expr.clone());
            docs = docs
                .into_iter()
                .filter(|doc| filter.matches(doc).unwrap_or(false))
                .collect();
        }

        if let Some(sort_fields) = &find.sort {
            docs.sort_by(|a, b| {
                for sort_field in sort_fields {
                    let a_val = a.get_path(&sort_field.field);
                    let b_val = b.get_path(&sort_field.field);

                    let cmp = compare_boml_values(a_val, b_val);
                    if cmp != std::cmp::Ordering::Equal {
                        return match sort_field.order {
                            SortOrder::Ascending => cmp,
                            SortOrder::Descending => cmp.reverse(),
                        };
                    }
                }
                std::cmp::Ordering::Equal
            });
        }

        if let Some(skip) = find.skip {
            docs = docs.into_iter().skip(skip as usize).collect();
        }

        if let Some(limit) = find.limit {
            docs = docs.into_iter().take(limit as usize).collect();
        }

        if let Some(projection) = &find.projection {
            docs = docs
                .into_iter()
                .map(|doc| project_document(doc, projection))
                .collect();
        }

        Ok(QueryResponse::Documents(docs))
    }

    fn execute_update(&self, update: &UpdateStatement) -> QueryResult<QueryResponse> {
        let collection = self.storage.get_collection(&update.collection)?;

        let mut docs = collection.find_all()?;

        if let Some(filter_expr) = &update.filter {
            let filter = filter::Filter::new(filter_expr.clone());
            docs = docs
                .into_iter()
                .filter(|doc| filter.matches(doc).unwrap_or(false))
                .collect();
        }

        let mut modified_count = 0u64;
        for mut doc in docs {
            for op in &update.updates {
                apply_update_operation(&mut doc, op)?;
            }

            if let Some(id) = doc.id() {
                collection.update(id, &doc)?;
                modified_count += 1;
            }

            if !update.multi {
                break;
            }
        }

        Ok(QueryResponse::Update {
            matched_count: modified_count,
            modified_count,
        })
    }

    fn execute_delete(&self, delete: &DeleteStatement) -> QueryResult<QueryResponse> {
        let collection = self.storage.get_collection(&delete.collection)?;

        let mut docs = collection.find_all()?;

        if let Some(filter_expr) = &delete.filter {
            let filter = filter::Filter::new(filter_expr.clone());
            docs = docs
                .into_iter()
                .filter(|doc| filter.matches(doc).unwrap_or(false))
                .collect();
        }

        let mut deleted_count = 0u64;
        for doc in docs {
            if let Some(id) = doc.id() {
                if collection.delete(id)? {
                    deleted_count += 1;
                }
            }

            if !delete.multi {
                break;
            }
        }

        Ok(QueryResponse::Delete { deleted_count })
    }

    fn execute_aggregate(&self, agg: &AggregateStatement) -> QueryResult<QueryResponse> {
        let collection = self.storage.get_collection(&agg.collection)?;

        let mut docs = collection.find_all()?;

        for stage in &agg.pipeline {
            docs = self.apply_aggregate_stage(docs, stage)?;
        }

        Ok(QueryResponse::Documents(docs))
    }

    fn apply_aggregate_stage(
        &self,
        docs: Vec<Document>,
        stage: &AggregateStage,
    ) -> QueryResult<Vec<Document>> {
        match stage {
            AggregateStage::Match(expr) => {
                let filter = filter::Filter::new(expr.clone());
                Ok(docs
                    .into_iter()
                    .filter(|doc| filter.matches(doc).unwrap_or(false))
                    .collect())
            }

            AggregateStage::Sort(fields) => {
                let mut sorted = docs;
                sorted.sort_by(|a, b| {
                    for sort_field in fields {
                        let a_val = a.get_path(&sort_field.field);
                        let b_val = b.get_path(&sort_field.field);
                        let cmp = compare_boml_values(a_val, b_val);
                        if cmp != std::cmp::Ordering::Equal {
                            return match sort_field.order {
                                SortOrder::Ascending => cmp,
                                SortOrder::Descending => cmp.reverse(),
                            };
                        }
                    }
                    std::cmp::Ordering::Equal
                });
                Ok(sorted)
            }

            AggregateStage::Limit(n) => {
                Ok(docs.into_iter().take(*n as usize).collect())
            }

            AggregateStage::Skip(n) => {
                Ok(docs.into_iter().skip(*n as usize).collect())
            }

            AggregateStage::Project(fields) => {
                let field_names: Vec<String> = fields.iter().map(|f| f.name.clone()).collect();
                Ok(docs
                    .into_iter()
                    .map(|doc| project_document(doc, &field_names))
                    .collect())
            }

            AggregateStage::Group { by, accumulators } => {
                self.execute_group(docs, by, accumulators)
            }

            AggregateStage::Count(field_name) => {
                let count = docs.len() as i64;
                let mut result = Document::without_id();
                result.insert(field_name.clone(), count);
                Ok(vec![result])
            }

            _ => Ok(docs),
        }
    }

    fn execute_group(
        &self,
        docs: Vec<Document>,
        group_by: &[String],
        accumulators: &[Accumulator],
    ) -> QueryResult<Vec<Document>> {
        let mut groups: HashMap<String, Vec<Document>> = HashMap::new();

        for doc in docs {
            let key = group_by
                .iter()
                .map(|field| {
                    doc.get_path(field)
                        .map(|v| format!("{}", v))
                        .unwrap_or_else(|| "null".to_string())
                })
                .collect::<Vec<_>>()
                .join("|");

            groups.entry(key).or_default().push(doc);
        }

        let mut results = Vec::new();

        for (key, group_docs) in groups {
            let mut result = Document::without_id();

            let key_parts: Vec<&str> = key.split('|').collect();
            for (i, field) in group_by.iter().enumerate() {
                if let Some(first_doc) = group_docs.first() {
                    if let Some(value) = first_doc.get_path(field) {
                        result.insert(format!("_id.{}", field), value.clone());
                    }
                }
            }

            for acc in accumulators {
                let value = self.compute_aggregate(&group_docs, acc)?;
                result.insert(acc.name.clone(), value);
            }

            results.push(result);
        }

        Ok(results)
    }

    fn compute_aggregate(&self, docs: &[Document], acc: &Accumulator) -> QueryResult<BomlValue> {
        match &acc.function {
            AggregateFunction::Count => {
                Ok(BomlValue::Int64(docs.len() as i64))
            }

            AggregateFunction::Sum => {
                let field = acc.field.as_ref().ok_or_else(|| {
                    QueryError::Execution("SUM requires a field".to_string())
                })?;

                let mut sum = 0.0f64;
                for doc in docs {
                    if let Some(val) = doc.get_path(field) {
                        sum += value_to_f64(val);
                    }
                }
                Ok(BomlValue::Float64(sum))
            }

            AggregateFunction::Avg => {
                let field = acc.field.as_ref().ok_or_else(|| {
                    QueryError::Execution("AVG requires a field".to_string())
                })?;

                let mut sum = 0.0f64;
                let mut count = 0usize;
                for doc in docs {
                    if let Some(val) = doc.get_path(field) {
                        sum += value_to_f64(val);
                        count += 1;
                    }
                }
                Ok(BomlValue::Float64(if count > 0 {
                    sum / count as f64
                } else {
                    0.0
                }))
            }

            AggregateFunction::Min => {
                let field = acc.field.as_ref().ok_or_else(|| {
                    QueryError::Execution("MIN requires a field".to_string())
                })?;

                let mut min: Option<&BomlValue> = None;
                for doc in docs {
                    if let Some(val) = doc.get_path(field) {
                        min = Some(match min {
                            None => val,
                            Some(current) => {
                                if compare_boml_values(Some(val), Some(current))
                                    == std::cmp::Ordering::Less
                                {
                                    val
                                } else {
                                    current
                                }
                            }
                        });
                    }
                }
                Ok(min.cloned().unwrap_or(BomlValue::Null))
            }

            AggregateFunction::Max => {
                let field = acc.field.as_ref().ok_or_else(|| {
                    QueryError::Execution("MAX requires a field".to_string())
                })?;

                let mut max: Option<&BomlValue> = None;
                for doc in docs {
                    if let Some(val) = doc.get_path(field) {
                        max = Some(match max {
                            None => val,
                            Some(current) => {
                                if compare_boml_values(Some(val), Some(current))
                                    == std::cmp::Ordering::Greater
                                {
                                    val
                                } else {
                                    current
                                }
                            }
                        });
                    }
                }
                Ok(max.cloned().unwrap_or(BomlValue::Null))
            }

            AggregateFunction::First => {
                let field = acc.field.as_ref().ok_or_else(|| {
                    QueryError::Execution("FIRST requires a field".to_string())
                })?;

                Ok(docs
                    .first()
                    .and_then(|doc| doc.get_path(field))
                    .cloned()
                    .unwrap_or(BomlValue::Null))
            }

            AggregateFunction::Last => {
                let field = acc.field.as_ref().ok_or_else(|| {
                    QueryError::Execution("LAST requires a field".to_string())
                })?;

                Ok(docs
                    .last()
                    .and_then(|doc| doc.get_path(field))
                    .cloned()
                    .unwrap_or(BomlValue::Null))
            }

            AggregateFunction::Push => {
                let field = acc.field.as_ref().ok_or_else(|| {
                    QueryError::Execution("PUSH requires a field".to_string())
                })?;

                let values: Vec<BomlValue> = docs
                    .iter()
                    .filter_map(|doc| doc.get_path(field).cloned())
                    .collect();
                Ok(BomlValue::Array(values))
            }

            AggregateFunction::AddToSet => {
                let field = acc.field.as_ref().ok_or_else(|| {
                    QueryError::Execution("ADD_TO_SET requires a field".to_string())
                })?;

                let mut seen = std::collections::HashSet::new();
                let mut values = Vec::new();
                for doc in docs {
                    if let Some(val) = doc.get_path(field) {
                        let key = format!("{}", val);
                        if seen.insert(key) {
                            values.push(val.clone());
                        }
                    }
                }
                Ok(BomlValue::Array(values))
            }
        }
    }
}

fn project_document(doc: Document, fields: &[String]) -> Document {
    let mut result = Document::without_id();

    if let Some(id) = doc.id() {
        result.set_id(*id);
    }

    for field in fields {
        if let Some(value) = doc.get_path(field) {
            result.insert(field.clone(), value.clone());
        }
    }

    result
}

fn apply_update_operation(doc: &mut Document, op: &UpdateOperation) -> QueryResult<()> {
    match op {
        UpdateOperation::Set { field, value } => {
            doc.insert(field.clone(), value.clone());
        }
        UpdateOperation::Unset { field } => {
            doc.remove(field);
        }
        UpdateOperation::Inc { field, value } => {
            let current = doc.get(field).cloned().unwrap_or(BomlValue::Int64(0));
            let new_value = add_values(&current, value)?;
            doc.insert(field.clone(), new_value);
        }
        UpdateOperation::Push { field, value } => {
            let current = doc.get(field).cloned();
            match current {
                Some(BomlValue::Array(mut arr)) => {
                    arr.push(value.clone());
                    doc.insert(field.clone(), BomlValue::Array(arr));
                }
                None => {
                    doc.insert(field.clone(), BomlValue::Array(vec![value.clone()]));
                }
                _ => {
                    return Err(QueryError::TypeError(format!(
                        "Cannot push to non-array field: {}",
                        field
                    )));
                }
            }
        }
        UpdateOperation::Pull { field, value } => {
            if let Some(BomlValue::Array(arr)) = doc.get(field).cloned() {
                let filtered: Vec<BomlValue> = arr
                    .into_iter()
                    .filter(|v| v != value)
                    .collect();
                doc.insert(field.clone(), BomlValue::Array(filtered));
            }
        }
        UpdateOperation::Rename { from, to } => {
            if let Some(value) = doc.remove(from) {
                doc.insert(to.clone(), value);
            }
        }
    }
    Ok(())
}

fn add_values(a: &BomlValue, b: &BomlValue) -> QueryResult<BomlValue> {
    match (a, b) {
        (BomlValue::Int32(x), BomlValue::Int32(y)) => Ok(BomlValue::Int32(x + y)),
        (BomlValue::Int64(x), BomlValue::Int64(y)) => Ok(BomlValue::Int64(x + y)),
        (BomlValue::Int32(x), BomlValue::Int64(y)) => Ok(BomlValue::Int64(*x as i64 + y)),
        (BomlValue::Int64(x), BomlValue::Int32(y)) => Ok(BomlValue::Int64(x + *y as i64)),
        (BomlValue::Float64(x), BomlValue::Float64(y)) => Ok(BomlValue::Float64(x + y)),
        (BomlValue::Float64(x), BomlValue::Int32(y)) => Ok(BomlValue::Float64(x + *y as f64)),
        (BomlValue::Float64(x), BomlValue::Int64(y)) => Ok(BomlValue::Float64(x + *y as f64)),
        _ => Err(QueryError::TypeError(format!(
            "Cannot add {:?} and {:?}",
            a.type_name(),
            b.type_name()
        ))),
    }
}

fn value_to_f64(val: &BomlValue) -> f64 {
    match val {
        BomlValue::Int32(n) => *n as f64,
        BomlValue::Int64(n) => *n as f64,
        BomlValue::Float32(n) => *n as f64,
        BomlValue::Float64(n) => *n,
        _ => 0.0,
    }
}

fn compare_boml_values(a: Option<&BomlValue>, b: Option<&BomlValue>) -> std::cmp::Ordering {
    match (a, b) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (Some(_), None) => std::cmp::Ordering::Greater,
        (Some(BomlValue::Null), Some(BomlValue::Null)) => std::cmp::Ordering::Equal,
        (Some(BomlValue::Null), _) => std::cmp::Ordering::Less,
        (_, Some(BomlValue::Null)) => std::cmp::Ordering::Greater,
        (Some(BomlValue::Int32(a)), Some(BomlValue::Int32(b))) => a.cmp(b),
        (Some(BomlValue::Int64(a)), Some(BomlValue::Int64(b))) => a.cmp(b),
        (Some(BomlValue::Int32(a)), Some(BomlValue::Int64(b))) => (*a as i64).cmp(b),
        (Some(BomlValue::Int64(a)), Some(BomlValue::Int32(b))) => a.cmp(&(*b as i64)),
        (Some(BomlValue::Float64(a)), Some(BomlValue::Float64(b))) => {
            a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
        }
        (Some(BomlValue::String(a)), Some(BomlValue::String(b))) => a.cmp(b),
        (Some(BomlValue::DateTime(a)), Some(BomlValue::DateTime(b))) => a.cmp(b),
        _ => std::cmp::Ordering::Equal,
    }
}

/// 查询响应枚举
///
/// 表示查询执行后的各种响应类型
#[derive(Debug, Clone)]
pub enum QueryResponse {
    Ok {
        message: String,
    },
    Documents(Vec<Document>),
    Insert {
        inserted_count: u64,
        inserted_ids: Vec<String>,
    },
    Update {
        matched_count: u64,
        modified_count: u64,
    },
    Delete {
        deleted_count: u64,
    },
    Databases(Vec<String>),
    Collections(Vec<String>),
    Indexes(Vec<IndexInfo>),
    Status {
        size: u64,
        stats: String,
    },
}

/// 索引信息
///
/// 描述集合上的索引信息
#[derive(Debug, Clone)]
pub struct IndexInfo {
    pub name: String,
    pub collection: String,
    pub fields: Vec<String>,
    pub unique: bool,
}

impl QueryResponse {
    /// 转换为 JSON 字符串
    ///
    /// # Brief
    /// 将响应转换为 JSON 格式的字符串
    ///
    /// # Returns
    /// JSON 格式的响应字符串
    pub fn to_json(&self) -> String {
        match self {
            QueryResponse::Ok { message } => {
                serde_json::json!({ "ok": 1, "message": message }).to_string()
            }
            QueryResponse::Documents(docs) => {
                let values: Vec<serde_json::Value> = docs
                    .iter()
                    .map(|d| serde_json::from_str(&d.to_json()).unwrap_or(serde_json::Value::Null))
                    .collect();
                serde_json::to_string_pretty(&values).unwrap_or_default()
            }
            QueryResponse::Insert { inserted_count, inserted_ids } => {
                serde_json::json!({
                    "ok": 1,
                    "insertedCount": inserted_count,
                    "insertedIds": inserted_ids
                })
                .to_string()
            }
            QueryResponse::Update { matched_count, modified_count } => {
                serde_json::json!({
                    "ok": 1,
                    "matchedCount": matched_count,
                    "modifiedCount": modified_count
                })
                .to_string()
            }
            QueryResponse::Delete { deleted_count } => {
                serde_json::json!({
                    "ok": 1,
                    "deletedCount": deleted_count
                })
                .to_string()
            }
            QueryResponse::Databases(dbs) => {
                serde_json::json!({ "databases": dbs }).to_string()
            }
            QueryResponse::Collections(cols) => {
                serde_json::json!({ "collections": cols }).to_string()
            }
            QueryResponse::Indexes(idxs) => {
                let info: Vec<serde_json::Value> = idxs
                    .iter()
                    .map(|i| {
                        serde_json::json!({
                            "name": i.name,
                            "collection": i.collection,
                            "fields": i.fields,
                            "unique": i.unique
                        })
                    })
                    .collect();
                serde_json::json!({ "indexes": info }).to_string()
            }
            QueryResponse::Status { size, stats } => {
                serde_json::json!({
                    "size": size,
                    "stats": stats
                })
                .to_string()
            }
        }
    }
}
