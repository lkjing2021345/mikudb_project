//! MQL 解析器模块
//!
//! 提供 MQL (Miku Query Language) 的词法和语法分析功能。
//! 支持 SQL 风格的查询语法，同时支持 MongoDB 风格的聚合管道。

use crate::ast::*;
use crate::lexer::{Lexer, Token};
use crate::{QueryError, QueryResult};
use compact_str::CompactString;
use indexmap::IndexMap;
use mikudb_boml::BomlValue;
use std::iter::Peekable;

/// MQL 解析器
///
/// 使用递归下降算法将 MQL 查询字符串解析为 AST
pub struct Parser<'a> {
    tokens: Peekable<std::vec::IntoIter<(Token, std::ops::Range<usize>)>>,
    input: &'a str,
}

impl<'a> Parser<'a> {
    /// 创建新解析器
    ///
    /// # Brief
    /// 从查询字符串创建解析器实例
    ///
    /// # Arguments
    /// * `input` - MQL 查询字符串
    ///
    /// # Returns
    /// 新的 Parser 实例
    pub fn new(input: &'a str) -> Self {
        let tokens = Lexer::tokenize(input);
        Self {
            tokens: tokens.into_iter().peekable(),
            input,
        }
    }

    /// 解析单个语句
    ///
    /// # Brief
    /// 解析输入字符串为单个 Statement
    ///
    /// # Arguments
    /// * `input` - MQL 查询字符串
    ///
    /// # Returns
    /// 成功返回 Statement，语法错误返回 QueryError
    pub fn parse(input: &str) -> QueryResult<Statement> {
        let mut parser = Parser::new(input);
        parser.parse_statement()
    }

    /// 解析多个语句
    ///
    /// # Brief
    /// 解析输入字符串为多个 Statement（以分号分隔）
    ///
    /// # Arguments
    /// * `input` - MQL 查询字符串
    ///
    /// # Returns
    /// 成功返回 Statement 向量，语法错误返回 QueryError
    pub fn parse_multiple(input: &str) -> QueryResult<Vec<Statement>> {
        let mut parser = Parser::new(input);
        let mut statements = Vec::new();

        while parser.peek().is_some() {
            statements.push(parser.parse_statement()?);
            parser.skip_if(Token::Semicolon);
        }

        Ok(statements)
    }

    fn peek(&mut self) -> Option<&Token> {
        self.tokens.peek().map(|(t, _)| t)
    }

    fn next(&mut self) -> Option<Token> {
        self.tokens.next().map(|(t, _)| t)
    }

    fn expect(&mut self, expected: Token) -> QueryResult<()> {
        match self.next() {
            Some(ref t) if *t == expected => Ok(()),
            Some(t) => Err(QueryError::Syntax(format!(
                "Expected {:?}, got {:?}",
                expected, t
            ))),
            None => Err(QueryError::Syntax(format!(
                "Expected {:?}, got end of input",
                expected
            ))),
        }
    }

    fn skip_if(&mut self, token: Token) -> bool {
        if self.peek() == Some(&token) {
            self.next();
            true
        } else {
            false
        }
    }

    fn parse_identifier(&mut self) -> QueryResult<String> {
        match self.next() {
            Some(Token::Identifier(s)) | Some(Token::QuotedIdentifier(s)) => Ok(s),
            Some(t) => Err(QueryError::Syntax(format!(
                "Expected identifier, got {:?}",
                t
            ))),
            None => Err(QueryError::Syntax("Expected identifier".to_string())),
        }
    }

    fn parse_statement(&mut self) -> QueryResult<Statement> {
        match self.peek() {
            Some(Token::Use) => self.parse_use(),
            Some(Token::Show) => self.parse_show(),
            Some(Token::Create) => self.parse_create(),
            Some(Token::Drop) => self.parse_drop(),
            Some(Token::Insert) => self.parse_insert(),
            Some(Token::Find) => self.parse_find(),
            Some(Token::Update) => self.parse_update(),
            Some(Token::Delete) => self.parse_delete(),
            Some(Token::Aggregate) => self.parse_aggregate(),
            Some(Token::Begin) => {
                self.next();
                self.expect(Token::Transaction)?;
                Ok(Statement::BeginTransaction)
            }
            Some(Token::Commit) => {
                self.next();
                Ok(Statement::Commit)
            }
            Some(Token::Rollback) => {
                self.next();
                Ok(Statement::Rollback)
            }
            Some(Token::Grant) => self.parse_grant(),
            Some(Token::Revoke) => self.parse_revoke(),
            Some(Token::Ai) => self.parse_ai(),
            Some(t) => Err(QueryError::Syntax(format!("Unexpected token: {:?}", t))),
            None => Err(QueryError::Syntax("Empty query".to_string())),
        }
    }

    fn parse_use(&mut self) -> QueryResult<Statement> {
        self.expect(Token::Use)?;
        let database = self.parse_identifier()?;
        Ok(Statement::Use(UseStatement { database }))
    }

    fn parse_show(&mut self) -> QueryResult<Statement> {
        self.expect(Token::Show)?;
        match self.peek() {
            Some(Token::Databases) => {
                self.next();
                Ok(Statement::ShowDatabases)
            }
            Some(Token::Collections) => {
                self.next();
                Ok(Statement::ShowCollections)
            }
            Some(Token::Indexes) => {
                self.next();
                self.expect(Token::On)?;
                let collection = self.parse_identifier()?;
                Ok(Statement::ShowIndexes(collection))
            }
            Some(Token::Status) => {
                self.next();
                Ok(Statement::ShowStatus)
            }
            Some(Token::Users) => {
                self.next();
                Ok(Statement::ShowUsers)
            }
            _ => Err(QueryError::Syntax(
                "Expected DATABASES, COLLECTIONS, INDEXES, STATUS, or USERS".to_string(),
            )),
        }
    }

    fn parse_create(&mut self) -> QueryResult<Statement> {
        self.expect(Token::Create)?;
        match self.peek() {
            Some(Token::Database) => {
                self.next();
                let name = self.parse_identifier()?;
                Ok(Statement::CreateDatabase(name))
            }
            Some(Token::Collection) => {
                self.next();
                let name = self.parse_identifier()?;
                Ok(Statement::CreateCollection(name))
            }
            Some(Token::Index) | Some(Token::Unique) | Some(Token::Text) => {
                self.parse_create_index()
            }
            Some(Token::User) => self.parse_create_user(),
            _ => Err(QueryError::Syntax(
                "Expected DATABASE, COLLECTION, INDEX, or USER".to_string(),
            )),
        }
    }

    fn parse_create_index(&mut self) -> QueryResult<Statement> {
        let mut unique = false;
        let mut index_type = IndexType::BTree;

        if self.skip_if(Token::Unique) {
            unique = true;
        }
        if self.skip_if(Token::Text) {
            index_type = IndexType::Text;
        }

        self.expect(Token::Index)?;
        let name = self.parse_identifier()?;
        self.expect(Token::On)?;
        let collection = self.parse_identifier()?;
        self.expect(Token::LParen)?;

        let mut fields = Vec::new();
        loop {
            let field_name = self.parse_identifier()?;
            let order = if self.skip_if(Token::Desc) {
                SortOrder::Descending
            } else {
                self.skip_if(Token::Asc);
                SortOrder::Ascending
            };
            fields.push(IndexField {
                name: field_name,
                order,
            });

            if !self.skip_if(Token::Comma) {
                break;
            }
        }

        self.expect(Token::RParen)?;

        Ok(Statement::CreateIndex(CreateIndexStatement {
            name,
            collection,
            fields,
            unique,
            index_type,
        }))
    }

    fn parse_create_user(&mut self) -> QueryResult<Statement> {
        self.expect(Token::User)?;
        let username = self.parse_identifier()?;
        self.expect(Token::With)?;
        self.expect(Token::Password)?;
        let password = match self.next() {
            Some(Token::String(s)) => s,
            _ => return Err(QueryError::Syntax("Expected password string".to_string())),
        };

        let mut roles = Vec::new();
        if self.skip_if(Token::Role) {
            roles.push(self.parse_identifier()?);
            while self.skip_if(Token::Comma) {
                roles.push(self.parse_identifier()?);
            }
        }

        Ok(Statement::CreateUser(CreateUserStatement {
            username,
            password,
            roles,
        }))
    }

    fn parse_drop(&mut self) -> QueryResult<Statement> {
        self.expect(Token::Drop)?;
        match self.peek() {
            Some(Token::Database) => {
                self.next();
                let name = self.parse_identifier()?;
                Ok(Statement::DropDatabase(name))
            }
            Some(Token::Collection) => {
                self.next();
                let name = self.parse_identifier()?;
                Ok(Statement::DropCollection(name))
            }
            Some(Token::Index) => {
                self.next();
                let name = self.parse_identifier()?;
                self.expect(Token::On)?;
                let collection = self.parse_identifier()?;
                Ok(Statement::DropIndex(DropIndexStatement { name, collection }))
            }
            Some(Token::User) => {
                self.next();
                let name = self.parse_identifier()?;
                Ok(Statement::DropUser(name))
            }
            _ => Err(QueryError::Syntax(
                "Expected DATABASE, COLLECTION, INDEX, or USER".to_string(),
            )),
        }
    }

    fn parse_insert(&mut self) -> QueryResult<Statement> {
        self.expect(Token::Insert)?;
        self.expect(Token::Into)?;
        let collection = self.parse_identifier()?;

        let documents = if self.peek() == Some(&Token::LBracket) {
            self.parse_array_literal()?
        } else {
            vec![self.parse_document_literal()?]
        };

        Ok(Statement::Insert(InsertStatement {
            collection,
            documents,
        }))
    }

    fn parse_find(&mut self) -> QueryResult<Statement> {
        self.expect(Token::Find)?;
        let collection = self.parse_identifier()?;

        let mut stmt = FindStatement {
            collection,
            ..Default::default()
        };

        loop {
            match self.peek() {
                Some(Token::Where) => {
                    self.next();
                    stmt.filter = Some(self.parse_expression()?);
                }
                Some(Token::Select) => {
                    self.next();
                    stmt.projection = Some(self.parse_field_list()?);
                }
                Some(Token::Order) => {
                    self.next();
                    self.expect(Token::By)?;
                    stmt.sort = Some(self.parse_sort_fields()?);
                }
                Some(Token::Limit) => {
                    self.next();
                    stmt.limit = Some(self.parse_integer()? as u64);
                }
                Some(Token::Skip) => {
                    self.next();
                    stmt.skip = Some(self.parse_integer()? as u64);
                }
                _ => break,
            }
        }

        Ok(Statement::Find(stmt))
    }

    fn parse_update(&mut self) -> QueryResult<Statement> {
        self.expect(Token::Update)?;
        let collection = self.parse_identifier()?;

        let mut updates = Vec::new();

        if self.skip_if(Token::Set) {
            loop {
                let field = self.parse_identifier()?;

                let op = match self.peek() {
                    Some(Token::PlusEq) => {
                        self.next();
                        let value = self.parse_value()?;
                        UpdateOperation::Inc { field, value }
                    }
                    Some(Token::Eq) => {
                        self.next();
                        let value = self.parse_value()?;
                        UpdateOperation::Set { field, value }
                    }
                    _ => {
                        self.expect(Token::Eq)?;
                        let value = self.parse_value()?;
                        UpdateOperation::Set { field, value }
                    }
                };

                updates.push(op);

                if !self.skip_if(Token::Comma) {
                    break;
                }
            }
        }

        if self.skip_if(Token::Unset) {
            loop {
                let field = self.parse_identifier()?;
                updates.push(UpdateOperation::Unset { field });
                if !self.skip_if(Token::Comma) {
                    break;
                }
            }
        }

        if self.skip_if(Token::Push) {
            let field = self.parse_identifier()?;
            self.expect(Token::Eq)?;
            let value = self.parse_value()?;
            updates.push(UpdateOperation::Push { field, value });
        }

        let filter = if self.skip_if(Token::Where) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Statement::Update(UpdateStatement {
            collection,
            filter,
            updates,
            upsert: false,
            multi: true,
        }))
    }

    fn parse_delete(&mut self) -> QueryResult<Statement> {
        self.expect(Token::Delete)?;
        self.expect(Token::From)?;
        let collection = self.parse_identifier()?;

        let filter = if self.skip_if(Token::Where) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Statement::Delete(DeleteStatement {
            collection,
            filter,
            multi: true,
        }))
    }

    fn parse_aggregate(&mut self) -> QueryResult<Statement> {
        self.expect(Token::Aggregate)?;
        let collection = self.parse_identifier()?;

        let mut pipeline = Vec::new();

        while self.skip_if(Token::Pipe) {
            let stage = self.parse_aggregate_stage()?;
            pipeline.push(stage);
        }

        Ok(Statement::Aggregate(AggregateStatement {
            collection,
            pipeline,
        }))
    }

    fn parse_aggregate_stage(&mut self) -> QueryResult<AggregateStage> {
        match self.peek() {
            Some(Token::Match) => {
                self.next();
                let expr = self.parse_expression()?;
                Ok(AggregateStage::Match(expr))
            }
            Some(Token::Group) => {
                self.next();
                self.expect(Token::By)?;

                let mut by = Vec::new();
                by.push(self.parse_identifier()?);
                while self.skip_if(Token::Comma) {
                    by.push(self.parse_identifier()?);
                }

                let mut accumulators = Vec::new();
                if self.skip_if(Token::As) {
                    self.expect(Token::LBrace)?;
                    loop {
                        let name = self.parse_identifier()?;
                        self.expect(Token::Colon)?;
                        let (function, field) = self.parse_aggregate_function()?;
                        accumulators.push(Accumulator {
                            name,
                            function,
                            field,
                        });
                        if !self.skip_if(Token::Comma) {
                            break;
                        }
                    }
                    self.expect(Token::RBrace)?;
                }

                Ok(AggregateStage::Group { by, accumulators })
            }
            Some(Token::Sort) => {
                self.next();
                let fields = self.parse_sort_fields()?;
                Ok(AggregateStage::Sort(fields))
            }
            Some(Token::Limit) => {
                self.next();
                let n = self.parse_integer()? as u64;
                Ok(AggregateStage::Limit(n))
            }
            Some(Token::Skip) => {
                self.next();
                let n = self.parse_integer()? as u64;
                Ok(AggregateStage::Skip(n))
            }
            Some(Token::Project) => {
                self.next();
                let fields = self.parse_project_fields()?;
                Ok(AggregateStage::Project(fields))
            }
            Some(Token::Unwind) => {
                self.next();
                let path = self.parse_identifier()?;
                Ok(AggregateStage::Unwind {
                    path,
                    preserve_null: false,
                })
            }
            _ => Err(QueryError::Syntax("Expected aggregate stage".to_string())),
        }
    }

    fn parse_aggregate_function(&mut self) -> QueryResult<(AggregateFunction, Option<String>)> {
        let func = match self.peek() {
            Some(Token::Count) => {
                self.next();
                AggregateFunction::Count
            }
            Some(Token::Sum) => {
                self.next();
                AggregateFunction::Sum
            }
            Some(Token::Avg) => {
                self.next();
                AggregateFunction::Avg
            }
            Some(Token::Min) => {
                self.next();
                AggregateFunction::Min
            }
            Some(Token::Max) => {
                self.next();
                AggregateFunction::Max
            }
            Some(Token::First) => {
                self.next();
                AggregateFunction::First
            }
            Some(Token::Last) => {
                self.next();
                AggregateFunction::Last
            }
            _ => return Err(QueryError::Syntax("Expected aggregate function".to_string())),
        };

        self.expect(Token::LParen)?;
        let field = if self.peek() != Some(&Token::RParen) {
            Some(self.parse_identifier()?)
        } else {
            None
        };
        self.expect(Token::RParen)?;

        Ok((func, field))
    }

    fn parse_grant(&mut self) -> QueryResult<Statement> {
        self.expect(Token::Grant)?;
        let privilege = self.parse_identifier()?;
        self.expect(Token::On)?;
        let resource = self.parse_identifier()?;
        self.expect(Token::To)?;
        let username = self.parse_identifier()?;

        Ok(Statement::Grant(GrantStatement {
            privilege,
            resource,
            username,
        }))
    }

    fn parse_revoke(&mut self) -> QueryResult<Statement> {
        self.expect(Token::Revoke)?;
        let privilege = self.parse_identifier()?;
        self.expect(Token::On)?;
        let resource = self.parse_identifier()?;
        self.expect(Token::From)?;
        let username = self.parse_identifier()?;

        Ok(Statement::Revoke(RevokeStatement {
            privilege,
            resource,
            username,
        }))
    }

    fn parse_ai(&mut self) -> QueryResult<Statement> {
        self.expect(Token::Ai)?;
        match self.peek() {
            Some(Token::Query) => {
                self.next();
                let query = match self.next() {
                    Some(Token::String(s)) => s,
                    _ => return Err(QueryError::Syntax("Expected query string".to_string())),
                };
                Ok(Statement::AiQuery(query))
            }
            Some(Token::Analyze) => {
                self.next();
                let collection = self.parse_identifier()?;
                Ok(Statement::AiAnalyze(collection))
            }
            Some(Token::Suggest) => {
                self.next();
                self.expect(Token::Index)?;
                let collection = self.parse_identifier()?;
                Ok(Statement::AiSuggestIndex(collection))
            }
            _ => Err(QueryError::Syntax(
                "Expected QUERY, ANALYZE, or SUGGEST".to_string(),
            )),
        }
    }

    fn parse_expression(&mut self) -> QueryResult<Expression> {
        self.parse_or_expression()
    }

    fn parse_or_expression(&mut self) -> QueryResult<Expression> {
        let mut left = self.parse_and_expression()?;

        while self.skip_if(Token::Or) {
            let right = self.parse_and_expression()?;
            left = Expression::or(left, right);
        }

        Ok(left)
    }

    fn parse_and_expression(&mut self) -> QueryResult<Expression> {
        let mut left = self.parse_not_expression()?;

        while self.skip_if(Token::And) {
            let right = self.parse_not_expression()?;
            left = Expression::and(left, right);
        }

        Ok(left)
    }

    fn parse_not_expression(&mut self) -> QueryResult<Expression> {
        if self.skip_if(Token::Not) {
            let expr = self.parse_comparison_expression()?;
            Ok(Expression::not(expr))
        } else {
            self.parse_comparison_expression()
        }
    }

    fn parse_comparison_expression(&mut self) -> QueryResult<Expression> {
        let left = self.parse_additive_expression()?;

        let op = match self.peek() {
            Some(Token::Eq) => {
                self.next();
                Some(BinaryOp::Eq)
            }
            Some(Token::Ne) | Some(Token::Ne2) => {
                self.next();
                Some(BinaryOp::Ne)
            }
            Some(Token::Lt) => {
                self.next();
                Some(BinaryOp::Lt)
            }
            Some(Token::Le) => {
                self.next();
                Some(BinaryOp::Le)
            }
            Some(Token::Gt) => {
                self.next();
                Some(BinaryOp::Gt)
            }
            Some(Token::Ge) => {
                self.next();
                Some(BinaryOp::Ge)
            }
            Some(Token::In) => {
                self.next();
                let list = self.parse_value_list()?;
                return Ok(Expression::In {
                    expr: Box::new(left),
                    list,
                });
            }
            Some(Token::Like) => {
                self.next();
                let pattern = match self.next() {
                    Some(Token::String(s)) => s,
                    _ => return Err(QueryError::Syntax("Expected pattern string".to_string())),
                };
                return Ok(Expression::Like {
                    expr: Box::new(left),
                    pattern,
                });
            }
            Some(Token::Between) => {
                self.next();
                let low = self.parse_additive_expression()?;
                self.expect(Token::And)?;
                let high = self.parse_additive_expression()?;
                return Ok(Expression::Between {
                    expr: Box::new(left),
                    low: Box::new(low),
                    high: Box::new(high),
                });
            }
            Some(Token::Is) => {
                self.next();
                let negated = self.skip_if(Token::Not);
                self.expect(Token::Null)?;
                return Ok(Expression::IsNull {
                    expr: Box::new(left),
                    negated,
                });
            }
            _ => None,
        };

        if let Some(op) = op {
            let right = self.parse_additive_expression()?;
            Ok(Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            })
        } else {
            Ok(left)
        }
    }

    fn parse_additive_expression(&mut self) -> QueryResult<Expression> {
        let mut left = self.parse_multiplicative_expression()?;

        loop {
            let op = match self.peek() {
                Some(Token::Plus) => {
                    self.next();
                    BinaryOp::Add
                }
                Some(Token::Minus) => {
                    self.next();
                    BinaryOp::Sub
                }
                _ => break,
            };

            let right = self.parse_multiplicative_expression()?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_multiplicative_expression(&mut self) -> QueryResult<Expression> {
        let mut left = self.parse_unary_expression()?;

        loop {
            let op = match self.peek() {
                Some(Token::Star) => {
                    self.next();
                    BinaryOp::Mul
                }
                Some(Token::Slash) => {
                    self.next();
                    BinaryOp::Div
                }
                Some(Token::Percent) => {
                    self.next();
                    BinaryOp::Mod
                }
                _ => break,
            };

            let right = self.parse_unary_expression()?;
            left = Expression::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
            };
        }

        Ok(left)
    }

    fn parse_unary_expression(&mut self) -> QueryResult<Expression> {
        if self.skip_if(Token::Minus) {
            let expr = self.parse_primary_expression()?;
            Ok(Expression::Unary {
                op: UnaryOp::Neg,
                expr: Box::new(expr),
            })
        } else {
            self.parse_primary_expression()
        }
    }

    fn parse_primary_expression(&mut self) -> QueryResult<Expression> {
        match self.peek() {
            Some(Token::LParen) => {
                self.next();
                let expr = self.parse_expression()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            Some(Token::True) => {
                self.next();
                Ok(Expression::Literal(BomlValue::Boolean(true)))
            }
            Some(Token::False) => {
                self.next();
                Ok(Expression::Literal(BomlValue::Boolean(false)))
            }
            Some(Token::Null) => {
                self.next();
                Ok(Expression::Literal(BomlValue::Null))
            }
            Some(Token::Integer(_)) | Some(Token::Float(_)) | Some(Token::String(_)) => {
                let value = self.parse_value()?;
                Ok(Expression::Literal(value))
            }
            Some(Token::LBracket) => {
                let values = self.parse_array_literal()?;
                Ok(Expression::Array(
                    values.into_iter().map(Expression::Literal).collect(),
                ))
            }
            Some(Token::LBrace) => {
                let doc = self.parse_document_literal()?;
                Ok(Expression::Literal(doc))
            }
            Some(Token::Identifier(_)) | Some(Token::QuotedIdentifier(_)) => {
                let name = self.parse_identifier()?;

                if self.skip_if(Token::LParen) {
                    let mut args = Vec::new();
                    if self.peek() != Some(&Token::RParen) {
                        args.push(self.parse_expression()?);
                        while self.skip_if(Token::Comma) {
                            args.push(self.parse_expression()?);
                        }
                    }
                    self.expect(Token::RParen)?;
                    Ok(Expression::Call {
                        function: name,
                        args,
                    })
                } else {
                    let mut path = name;
                    while self.skip_if(Token::Dot) {
                        let next = self.parse_identifier()?;
                        path = format!("{}.{}", path, next);
                    }
                    Ok(Expression::Field(path))
                }
            }
            Some(Token::Exists) => {
                self.next();
                self.expect(Token::LParen)?;
                let field = self.parse_identifier()?;
                self.expect(Token::RParen)?;
                Ok(Expression::Exists {
                    field,
                    negated: false,
                })
            }
            _ => Err(QueryError::Syntax("Expected expression".to_string())),
        }
    }

    fn parse_value(&mut self) -> QueryResult<BomlValue> {
        match self.next() {
            Some(Token::Integer(n)) => Ok(BomlValue::Int64(n)),
            Some(Token::Float(n)) => Ok(BomlValue::Float64(n)),
            Some(Token::String(s)) => Ok(BomlValue::String(CompactString::from(s))),
            Some(Token::True) => Ok(BomlValue::Boolean(true)),
            Some(Token::False) => Ok(BomlValue::Boolean(false)),
            Some(Token::Null) => Ok(BomlValue::Null),
            Some(Token::LBracket) => {
                let mut arr = Vec::new();
                if self.peek() != Some(&Token::RBracket) {
                    arr.push(self.parse_value()?);
                    while self.skip_if(Token::Comma) {
                        arr.push(self.parse_value()?);
                    }
                }
                self.expect(Token::RBracket)?;
                Ok(BomlValue::Array(arr))
            }
            Some(Token::LBrace) => {
                let mut doc = IndexMap::new();
                if self.peek() != Some(&Token::RBrace) {
                    loop {
                        let key = match self.next() {
                            Some(Token::String(s)) | Some(Token::Identifier(s)) => s,
                            _ => return Err(QueryError::Syntax("Expected field name".to_string())),
                        };
                        self.expect(Token::Colon)?;
                        let value = self.parse_value()?;
                        doc.insert(CompactString::from(key), value);
                        if !self.skip_if(Token::Comma) {
                            break;
                        }
                    }
                }
                self.expect(Token::RBrace)?;
                Ok(BomlValue::Document(doc))
            }
            _ => Err(QueryError::Syntax("Expected value".to_string())),
        }
    }

    fn parse_value_list(&mut self) -> QueryResult<Vec<Expression>> {
        self.expect(Token::LBracket)?;
        let mut list = Vec::new();
        if self.peek() != Some(&Token::RBracket) {
            list.push(self.parse_expression()?);
            while self.skip_if(Token::Comma) {
                list.push(self.parse_expression()?);
            }
        }
        self.expect(Token::RBracket)?;
        Ok(list)
    }

    fn parse_array_literal(&mut self) -> QueryResult<Vec<BomlValue>> {
        self.expect(Token::LBracket)?;
        let mut arr = Vec::new();
        if self.peek() != Some(&Token::RBracket) {
            arr.push(self.parse_value()?);
            while self.skip_if(Token::Comma) {
                arr.push(self.parse_value()?);
            }
        }
        self.expect(Token::RBracket)?;
        Ok(arr)
    }

    fn parse_document_literal(&mut self) -> QueryResult<BomlValue> {
        self.expect(Token::LBrace)?;
        let mut doc = IndexMap::new();
        if self.peek() != Some(&Token::RBrace) {
            loop {
                let key = match self.next() {
                    Some(Token::String(s)) | Some(Token::Identifier(s)) => s,
                    _ => return Err(QueryError::Syntax("Expected field name".to_string())),
                };
                self.expect(Token::Colon)?;
                let value = self.parse_value()?;
                doc.insert(CompactString::from(key), value);
                if !self.skip_if(Token::Comma) {
                    break;
                }
            }
        }
        self.expect(Token::RBrace)?;
        Ok(BomlValue::Document(doc))
    }

    fn parse_field_list(&mut self) -> QueryResult<Vec<String>> {
        let mut fields = Vec::new();
        fields.push(self.parse_identifier()?);
        while self.skip_if(Token::Comma) {
            fields.push(self.parse_identifier()?);
        }
        Ok(fields)
    }

    fn parse_sort_fields(&mut self) -> QueryResult<Vec<SortField>> {
        let mut fields = Vec::new();
        loop {
            let field = self.parse_identifier()?;
            let order = if self.skip_if(Token::Desc) {
                SortOrder::Descending
            } else {
                self.skip_if(Token::Asc);
                SortOrder::Ascending
            };
            fields.push(SortField { field, order });
            if !self.skip_if(Token::Comma) {
                break;
            }
        }
        Ok(fields)
    }

    fn parse_project_fields(&mut self) -> QueryResult<Vec<ProjectField>> {
        let mut fields = Vec::new();
        loop {
            let name = self.parse_identifier()?;
            fields.push(ProjectField {
                name,
                expression: None,
                include: true,
            });
            if !self.skip_if(Token::Comma) {
                break;
            }
        }
        Ok(fields)
    }

    fn parse_integer(&mut self) -> QueryResult<i64> {
        match self.next() {
            Some(Token::Integer(n)) => Ok(n),
            _ => Err(QueryError::Syntax("Expected integer".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_find() {
        let stmt = Parser::parse("FIND users WHERE age > 18").unwrap();
        assert!(matches!(stmt, Statement::Find(_)));
    }

    #[test]
    fn test_parse_insert() {
        let stmt = Parser::parse(r#"INSERT INTO users {"name": "test", "age": 25}"#).unwrap();
        assert!(matches!(stmt, Statement::Insert(_)));
    }

    #[test]
    fn test_parse_update() {
        let stmt = Parser::parse("UPDATE users SET active = true WHERE id = 1").unwrap();
        assert!(matches!(stmt, Statement::Update(_)));
    }

    #[test]
    fn test_parse_delete() {
        let stmt = Parser::parse("DELETE FROM users WHERE active = false").unwrap();
        assert!(matches!(stmt, Statement::Delete(_)));
    }

    #[test]
    fn test_parse_aggregate() {
        let stmt = Parser::parse(
            "AGGREGATE orders | MATCH status = 'completed' | GROUP BY customer_id AS {total: SUM(amount)} | SORT total DESC | LIMIT 10"
        ).unwrap();
        assert!(matches!(stmt, Statement::Aggregate(_)));
    }

    #[test]
    fn test_parse_create_index() {
        let stmt = Parser::parse("CREATE UNIQUE INDEX idx_email ON users (email ASC)").unwrap();
        assert!(matches!(stmt, Statement::CreateIndex(_)));
    }
}
