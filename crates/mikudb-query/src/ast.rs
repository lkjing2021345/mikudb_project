use compact_str::CompactString;
use mikudb_boml::BomlValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    Use(UseStatement),
    ShowDatabases,
    ShowCollections,
    ShowIndexes(String),
    ShowStatus,
    ShowUsers,

    CreateDatabase(String),
    DropDatabase(String),
    CreateCollection(String),
    DropCollection(String),
    CreateIndex(CreateIndexStatement),
    DropIndex(DropIndexStatement),

    Insert(InsertStatement),
    Find(FindStatement),
    Update(UpdateStatement),
    Delete(DeleteStatement),
    Aggregate(AggregateStatement),

    BeginTransaction,
    Commit,
    Rollback,

    CreateUser(CreateUserStatement),
    DropUser(String),
    Grant(GrantStatement),
    Revoke(RevokeStatement),

    AiQuery(String),
    AiAnalyze(String),
    AiSuggestIndex(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UseStatement {
    pub database: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateIndexStatement {
    pub name: String,
    pub collection: String,
    pub fields: Vec<IndexField>,
    pub unique: bool,
    pub index_type: IndexType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexField {
    pub name: String,
    pub order: SortOrder,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum IndexType {
    BTree,
    Hash,
    Text,
    Geo2d,
    Geo2dsphere,
}

impl Default for IndexType {
    fn default() -> Self {
        IndexType::BTree
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropIndexStatement {
    pub name: String,
    pub collection: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InsertStatement {
    pub collection: String,
    pub documents: Vec<BomlValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FindStatement {
    pub collection: String,
    pub filter: Option<Expression>,
    pub projection: Option<Vec<String>>,
    pub sort: Option<Vec<SortField>>,
    pub limit: Option<u64>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SortField {
    pub field: String,
    pub order: SortOrder,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl Default for SortOrder {
    fn default() -> Self {
        SortOrder::Ascending
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateStatement {
    pub collection: String,
    pub filter: Option<Expression>,
    pub updates: Vec<UpdateOperation>,
    pub upsert: bool,
    pub multi: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UpdateOperation {
    Set { field: String, value: BomlValue },
    Unset { field: String },
    Inc { field: String, value: BomlValue },
    Push { field: String, value: BomlValue },
    Pull { field: String, value: BomlValue },
    Rename { from: String, to: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeleteStatement {
    pub collection: String,
    pub filter: Option<Expression>,
    pub multi: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AggregateStatement {
    pub collection: String,
    pub pipeline: Vec<AggregateStage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AggregateStage {
    Match(Expression),
    Project(Vec<ProjectField>),
    Group {
        by: Vec<String>,
        accumulators: Vec<Accumulator>,
    },
    Sort(Vec<SortField>),
    Limit(u64),
    Skip(u64),
    Unwind {
        path: String,
        preserve_null: bool,
    },
    Lookup {
        from: String,
        local_field: String,
        foreign_field: String,
        as_field: String,
    },
    Count(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectField {
    pub name: String,
    pub expression: Option<Expression>,
    pub include: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Accumulator {
    pub name: String,
    pub function: AggregateFunction,
    pub field: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
    First,
    Last,
    Push,
    AddToSet,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    Literal(BomlValue),
    Field(String),
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expression>,
    },
    In {
        expr: Box<Expression>,
        list: Vec<Expression>,
    },
    Between {
        expr: Box<Expression>,
        low: Box<Expression>,
        high: Box<Expression>,
    },
    Like {
        expr: Box<Expression>,
        pattern: String,
    },
    IsNull {
        expr: Box<Expression>,
        negated: bool,
    },
    Exists {
        field: String,
        negated: bool,
    },
    Call {
        function: String,
        args: Vec<Expression>,
    },
    Array(Vec<Expression>),
    Document(Vec<(String, Expression)>),
}

impl Expression {
    pub fn literal(value: impl Into<BomlValue>) -> Self {
        Expression::Literal(value.into())
    }

    pub fn field(name: impl Into<String>) -> Self {
        Expression::Field(name.into())
    }

    pub fn and(left: Expression, right: Expression) -> Self {
        Expression::Binary {
            left: Box::new(left),
            op: BinaryOp::And,
            right: Box::new(right),
        }
    }

    pub fn or(left: Expression, right: Expression) -> Self {
        Expression::Binary {
            left: Box::new(left),
            op: BinaryOp::Or,
            right: Box::new(right),
        }
    }

    pub fn eq(left: Expression, right: Expression) -> Self {
        Expression::Binary {
            left: Box::new(left),
            op: BinaryOp::Eq,
            right: Box::new(right),
        }
    }

    pub fn ne(left: Expression, right: Expression) -> Self {
        Expression::Binary {
            left: Box::new(left),
            op: BinaryOp::Ne,
            right: Box::new(right),
        }
    }

    pub fn lt(left: Expression, right: Expression) -> Self {
        Expression::Binary {
            left: Box::new(left),
            op: BinaryOp::Lt,
            right: Box::new(right),
        }
    }

    pub fn le(left: Expression, right: Expression) -> Self {
        Expression::Binary {
            left: Box::new(left),
            op: BinaryOp::Le,
            right: Box::new(right),
        }
    }

    pub fn gt(left: Expression, right: Expression) -> Self {
        Expression::Binary {
            left: Box::new(left),
            op: BinaryOp::Gt,
            right: Box::new(right),
        }
    }

    pub fn ge(left: Expression, right: Expression) -> Self {
        Expression::Binary {
            left: Box::new(left),
            op: BinaryOp::Ge,
            right: Box::new(right),
        }
    }

    pub fn not(expr: Expression) -> Self {
        Expression::Unary {
            op: UnaryOp::Not,
            expr: Box::new(expr),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    And,
    Or,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateUserStatement {
    pub username: String,
    pub password: String,
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrantStatement {
    pub privilege: String,
    pub resource: String,
    pub username: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RevokeStatement {
    pub privilege: String,
    pub resource: String,
    pub username: String,
}
