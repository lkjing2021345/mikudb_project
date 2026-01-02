//! MQL 抽象语法树 (AST)
//!
//! 本模块定义 MQL 的所有语法结构:
//! - Statement: 顶层语句类型
//! - Expression: 表达式和条件
//! - CRUD 操作结构
//! - DDL 操作结构
//! - 聚合管道结构
//! - 用户管理结构
//!
//! AST 节点设计为可序列化,支持网络传输和持久化。

use mikudb_boml::BomlValue;
use serde::{Deserialize, Serialize};

/// MQL 语句
///
/// 表示 MQL 查询的顶层语句,包含所有支持的操作类型。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    // 数据库管理
    /// 切换当前数据库
    Use(UseStatement),
    /// 显示所有数据库
    ShowDatabases,
    /// 显示当前数据库的所有集合
    ShowCollections,
    /// 显示指定集合的索引
    ShowIndexes(String),
    /// 显示数据库状态
    ShowStatus,
    /// 显示所有用户
    ShowUsers,

    // DDL 操作
    /// 创建数据库
    CreateDatabase(String),
    /// 删除数据库
    DropDatabase(String),
    /// 创建集合
    CreateCollection(String),
    /// 删除集合
    DropCollection(String),
    /// 创建索引
    CreateIndex(CreateIndexStatement),
    /// 删除索引
    DropIndex(DropIndexStatement),

    // CRUD 操作
    /// 插入文档
    Insert(InsertStatement),
    /// 查询文档
    Find(FindStatement),
    /// 更新文档
    Update(UpdateStatement),
    /// 删除文档
    Delete(DeleteStatement),
    /// 聚合查询
    Aggregate(AggregateStatement),

    // 事务
    /// 开始事务
    BeginTransaction,
    /// 提交事务
    Commit,
    /// 回滚事务
    Rollback,

    // 用户管理
    /// 创建用户
    CreateUser(CreateUserStatement),
    /// 删除用户
    DropUser(String),
    /// 授予权限
    Grant(GrantStatement),
    /// 撤销权限
    Revoke(RevokeStatement),

    // AI 功能(实验性)
    /// AI 查询
    AiQuery(String),
    /// AI 分析
    AiAnalyze(String),
    /// AI 索引建议
    AiSuggestIndex(String),
}

/// USE 语句
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UseStatement {
    /// 数据库名称
    pub database: String,
}

/// CREATE INDEX 语句
///
/// 在集合上创建索引以加速查询。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateIndexStatement {
    /// 索引名称
    pub name: String,
    /// 集合名称
    pub collection: String,
    /// 索引字段列表
    pub fields: Vec<IndexField>,
    /// 是否唯一索引
    pub unique: bool,
    /// 索引类型
    pub index_type: IndexType,
}

/// 索引字段
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexField {
    /// 字段名称
    pub name: String,
    /// 排序顺序
    pub order: SortOrder,
}

/// 索引类型
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum IndexType {
    /// B-Tree 索引(默认,适合范围查询)
    BTree,
    /// 哈希索引(适合等值查询)
    Hash,
    /// 全文索引
    Text,
    /// 2D 地理空间索引
    Geo2d,
    /// 球面地理空间索引
    Geo2dsphere,
}

impl Default for IndexType {
    fn default() -> Self {
        IndexType::BTree
    }
}

/// DROP INDEX 语句
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropIndexStatement {
    /// 索引名称
    pub name: String,
    /// 集合名称
    pub collection: String,
}

/// INSERT 语句
///
/// 向集合插入一个或多个文档。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InsertStatement {
    /// 集合名称
    pub collection: String,
    /// 要插入的文档列表
    pub documents: Vec<BomlValue>,
}

/// FIND 语句
///
/// 从集合中查询文档,支持过滤、投影、排序、分页。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FindStatement {
    /// 集合名称
    pub collection: String,
    /// 过滤条件(WHERE 子句)
    pub filter: Option<Expression>,
    /// 投影字段(SELECT 子句)
    pub projection: Option<Vec<String>>,
    /// 排序字段(ORDER BY 子句)
    pub sort: Option<Vec<SortField>>,
    /// 限制返回数量
    pub limit: Option<u64>,
    /// 跳过记录数(分页偏移)
    pub skip: Option<u64>,
}

impl Default for FindStatement {
    fn default() -> Self {
        Self {
            collection: String::new(),
            filter: None,
            projection: None,
            sort: None,
            limit: None,
            skip: None,
        }
    }
}

/// 排序字段
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SortField {
    /// 字段名称
    pub field: String,
    /// 排序顺序
    pub order: SortOrder,
}

/// 排序顺序
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SortOrder {
    /// 升序(ASC)
    Ascending,
    /// 降序(DESC)
    Descending,
}

impl Default for SortOrder {
    fn default() -> Self {
        SortOrder::Ascending
    }
}

/// UPDATE 语句
///
/// 更新集合中匹配条件的文档。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateStatement {
    /// 集合名称
    pub collection: String,
    /// 过滤条件
    pub filter: Option<Expression>,
    /// 更新操作列表
    pub updates: Vec<UpdateOperation>,
    /// 如果不存在则插入(upsert)
    pub upsert: bool,
    /// 是否更新多条记录
    pub multi: bool,
}

/// 更新操作
///
/// 类似 MongoDB 的更新操作符。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UpdateOperation {
    /// $set - 设置字段值
    Set { field: String, value: BomlValue },
    /// $unset - 删除字段
    Unset { field: String },
    /// $inc - 增加数值
    Inc { field: String, value: BomlValue },
    /// $push - 向数组添加元素
    Push { field: String, value: BomlValue },
    /// $pull - 从数组移除元素
    Pull { field: String, value: BomlValue },
    /// $rename - 重命名字段
    Rename { from: String, to: String },
}

/// DELETE 语句
///
/// 从集合中删除匹配条件的文档。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeleteStatement {
    /// 集合名称
    pub collection: String,
    /// 过滤条件
    pub filter: Option<Expression>,
    /// 是否删除多条记录
    pub multi: bool,
}

/// AGGREGATE 语句
///
/// 聚合管道查询,支持多阶段数据处理。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AggregateStatement {
    /// 集合名称
    pub collection: String,
    /// 聚合管道阶段
    pub pipeline: Vec<AggregateStage>,
}

/// 聚合管道阶段
///
/// 类似 MongoDB 的聚合管道操作符。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AggregateStage {
    /// $match - 过滤文档
    Match(Expression),
    /// $project - 投影字段
    Project(Vec<ProjectField>),
    /// $group - 分组聚合
    Group {
        by: Vec<String>,
        accumulators: Vec<Accumulator>,
    },
    /// $sort - 排序
    Sort(Vec<SortField>),
    /// $limit - 限制数量
    Limit(u64),
    /// $skip - 跳过记录
    Skip(u64),
    /// $unwind - 展开数组
    Unwind {
        path: String,
        preserve_null: bool,
    },
    /// $lookup - 关联查询(类似 JOIN)
    Lookup {
        from: String,
        local_field: String,
        foreign_field: String,
        as_field: String,
    },
    /// $count - 计数
    Count(String),
}

/// 投影字段
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectField {
    /// 字段名称
    pub name: String,
    /// 计算表达式
    pub expression: Option<Expression>,
    /// 是否包含该字段
    pub include: bool,
}

/// 聚合累加器
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Accumulator {
    /// 累加器名称
    pub name: String,
    /// 聚合函数
    pub function: AggregateFunction,
    /// 源字段
    pub field: Option<String>,
}

/// 聚合函数
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AggregateFunction {
    /// COUNT - 计数
    Count,
    /// SUM - 求和
    Sum,
    /// AVG - 平均值
    Avg,
    /// MIN - 最小值
    Min,
    /// MAX - 最大值
    Max,
    /// FIRST - 第一个值
    First,
    /// LAST - 最后一个值
    Last,
    /// PUSH - 收集到数组
    Push,
    /// ADDTOSET - 收集到集合(去重)
    AddToSet,
}

/// 表达式
///
/// 表示查询条件、计算表达式等。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    /// 字面量值
    Literal(BomlValue),
    /// 字段引用
    Field(String),
    /// 二元运算
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },
    /// 一元运算
    Unary {
        op: UnaryOp,
        expr: Box<Expression>,
    },
    /// IN 运算
    In {
        expr: Box<Expression>,
        list: Vec<Expression>,
    },
    /// BETWEEN 运算
    Between {
        expr: Box<Expression>,
        low: Box<Expression>,
        high: Box<Expression>,
    },
    /// LIKE 模式匹配
    Like {
        expr: Box<Expression>,
        pattern: String,
    },
    /// IS NULL 检查
    IsNull {
        expr: Box<Expression>,
        negated: bool,
    },
    /// 字段存在性检查
    Exists {
        field: String,
        negated: bool,
    },
    /// 函数调用
    Call {
        function: String,
        args: Vec<Expression>,
    },
    /// 数组字面量
    Array(Vec<Expression>),
    /// 文档字面量
    Document(Vec<(String, Expression)>),
}

impl Expression {
    /// # Brief
    /// 创建字面量表达式
    pub fn literal(value: impl Into<BomlValue>) -> Self {
        Expression::Literal(value.into())
    }

    /// # Brief
    /// 创建字段引用表达式
    pub fn field(name: impl Into<String>) -> Self {
        Expression::Field(name.into())
    }

    /// # Brief
    /// 创建 AND 表达式
    pub fn and(left: Expression, right: Expression) -> Self {
        Expression::Binary {
            left: Box::new(left),
            op: BinaryOp::And,
            right: Box::new(right),
        }
    }

    /// # Brief
    /// 创建 OR 表达式
    pub fn or(left: Expression, right: Expression) -> Self {
        Expression::Binary {
            left: Box::new(left),
            op: BinaryOp::Or,
            right: Box::new(right),
        }
    }

    /// # Brief
    /// 创建等于表达式
    pub fn eq(left: Expression, right: Expression) -> Self {
        Expression::Binary {
            left: Box::new(left),
            op: BinaryOp::Eq,
            right: Box::new(right),
        }
    }

    /// # Brief
    /// 创建不等于表达式
    pub fn ne(left: Expression, right: Expression) -> Self {
        Expression::Binary {
            left: Box::new(left),
            op: BinaryOp::Ne,
            right: Box::new(right),
        }
    }

    /// # Brief
    /// 创建小于表达式
    pub fn lt(left: Expression, right: Expression) -> Self {
        Expression::Binary {
            left: Box::new(left),
            op: BinaryOp::Lt,
            right: Box::new(right),
        }
    }

    /// # Brief
    /// 创建小于等于表达式
    pub fn le(left: Expression, right: Expression) -> Self {
        Expression::Binary {
            left: Box::new(left),
            op: BinaryOp::Le,
            right: Box::new(right),
        }
    }

    /// # Brief
    /// 创建大于表达式
    pub fn gt(left: Expression, right: Expression) -> Self {
        Expression::Binary {
            left: Box::new(left),
            op: BinaryOp::Gt,
            right: Box::new(right),
        }
    }

    /// # Brief
    /// 创建大于等于表达式
    pub fn ge(left: Expression, right: Expression) -> Self {
        Expression::Binary {
            left: Box::new(left),
            op: BinaryOp::Ge,
            right: Box::new(right),
        }
    }

    /// # Brief
    /// 创建 NOT 表达式
    pub fn not(expr: Expression) -> Self {
        Expression::Unary {
            op: UnaryOp::Not,
            expr: Box::new(expr),
        }
    }
}

/// 二元操作符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    /// 逻辑与
    And,
    /// 逻辑或
    Or,
    /// 等于
    Eq,
    /// 不等于
    Ne,
    /// 小于
    Lt,
    /// 小于等于
    Le,
    /// 大于
    Gt,
    /// 大于等于
    Ge,
    /// 加法
    Add,
    /// 减法
    Sub,
    /// 乘法
    Mul,
    /// 除法
    Div,
    /// 取模
    Mod,
    /// 正则匹配
    Regex,
}

impl std::fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryOp::And => write!(f, "AND"),
            BinaryOp::Or => write!(f, "OR"),
            BinaryOp::Eq => write!(f, "="),
            BinaryOp::Ne => write!(f, "!="),
            BinaryOp::Lt => write!(f, "<"),
            BinaryOp::Le => write!(f, "<="),
            BinaryOp::Gt => write!(f, ">"),
            BinaryOp::Ge => write!(f, ">="),
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Sub => write!(f, "-"),
            BinaryOp::Mul => write!(f, "*"),
            BinaryOp::Div => write!(f, "/"),
            BinaryOp::Mod => write!(f, "%"),
            BinaryOp::Regex => write!(f, "~"),
        }
    }
}

/// 一元操作符
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    /// 逻辑非
    Not,
    /// 负号
    Neg,
}

/// CREATE USER 语句
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateUserStatement {
    /// 用户名
    pub username: String,
    /// 密码
    pub password: String,
    /// 角色列表
    pub roles: Vec<String>,
}

/// GRANT 语句
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrantStatement {
    /// 权限类型
    pub privilege: String,
    /// 资源(数据库.集合)
    pub resource: String,
    /// 用户名
    pub username: String,
}

/// REVOKE 语句
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RevokeStatement {
    /// 权限类型
    pub privilege: String,
    /// 资源(数据库.集合)
    pub resource: String,
    /// 用户名
    pub username: String,
}
