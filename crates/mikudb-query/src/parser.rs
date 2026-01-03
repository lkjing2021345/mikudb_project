//! MQL 解析器模块
//!
//! 本模块使用递归下降解析算法将 MQL 查询字符串转换为 AST:
//! - 支持 SQL 风格的查询语法 (FIND, INSERT, UPDATE, DELETE)
//! - 支持 MongoDB 风格的聚合管道 (AGGREGATE)
//! - 表达式优先级: OR < AND < NOT < 比较 < 加减 < 乘除模 < 一元 < 主表达式
//! - 错误处理: 语法错误时提供详细的位置和错误信息
//! - Peekable 迭代器: 支持前向查看 Token 而不消费

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

    /// # Brief
    /// 前向查看下一个 Token 而不消费
    ///
    /// 用于判断下一步的解析方向,不移动迭代器位置。
    fn peek(&mut self) -> Option<&Token> {
        self.tokens.peek().map(|(t, _)| t)
    }

    /// # Brief
    /// 消费并返回下一个 Token
    ///
    /// 移动迭代器位置,返回当前 Token。
    fn next(&mut self) -> Option<Token> {
        self.tokens.next().map(|(t, _)| t)
    }

    /// # Brief
    /// 期望下一个 Token 为指定类型,否则返回错误
    ///
    /// 消费 Token,如果不匹配则生成语法错误。
    ///
    /// # Arguments
    /// * `expected` - 期望的 Token 类型
    ///
    /// # Returns
    /// 匹配成功返回 Ok,否则返回 QueryError::Syntax
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

    /// # Brief
    /// 如果下一个 Token 匹配则跳过,否则不消费
    ///
    /// 用于处理可选的 Token,例如可选的 ASC/DESC。
    ///
    /// # Arguments
    /// * `token` - 要匹配的 Token
    ///
    /// # Returns
    /// 匹配成功返回 true,否则返回 false
    fn skip_if(&mut self, token: Token) -> bool {
        if self.peek() == Some(&token) {
            self.next();
            true
        } else {
            false
        }
    }

    /// # Brief
    /// 解析标识符(字段名、集合名等)
    ///
    /// 支持普通标识符和引号标识符,同时允许关键字作为标识符使用。
    ///
    /// # Returns
    /// 标识符字符串
    fn parse_identifier(&mut self) -> QueryResult<String> {
        match self.next() {
            // 普通标识符或引号标识符
            Some(Token::Identifier(s)) | Some(Token::QuotedIdentifier(s)) => Ok(s),
            // 字符串字面量也可以作为标识符（用于用户名等）
            Some(Token::String(s)) => Ok(s),
            // 允许关键字作为标识符(例如集合名为 "users")
            Some(Token::Users) => Ok("users".to_string()),
            Some(Token::User) => Ok("user".to_string()),
            Some(Token::Status) => Ok("status".to_string()),
            Some(Token::Index) => Ok("index".to_string()),
            Some(Token::Collection) => Ok("collection".to_string()),
            Some(Token::Database) => Ok("database".to_string()),
            Some(t) => Err(QueryError::Syntax(format!(
                "Expected identifier, got {:?}",
                t
            ))),
            None => Err(QueryError::Syntax("Expected identifier".to_string())),
        }
    }

    fn parse_string_literal(&mut self, label: &str) -> QueryResult<String> {
        match self.next() {
            Some(Token::String(s)) => Ok(s),
            Some(t) => Err(QueryError::Syntax(format!(
                "Expected {} string, got {:?}",
                label, t
            ))),
            None => Err(QueryError::Syntax(format!("Expected {} string", label))),
        }
    }

    /// # Brief
    /// 解析顶层语句
    ///
    /// 根据首个 Token 分发到对应的解析函数:
    /// - USE: 切换数据库
    /// - SHOW: 显示元数据
    /// - CREATE/DROP: DDL 操作
    /// - INSERT/FIND/UPDATE/DELETE: CRUD 操作
    /// - AGGREGATE: 聚合管道
    /// - BEGIN/COMMIT/ROLLBACK: 事务
    /// - GRANT/REVOKE: 权限管理
    /// - AI: AI 功能
    fn parse_statement(&mut self) -> QueryResult<Statement> {
        match self.peek() {
            Some(Token::Use) => self.parse_use(),
            Some(Token::Show) => self.parse_show(),
            Some(Token::Create) => self.parse_create(),
            Some(Token::Alter) => self.parse_alter(),
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

    /// # Brief
    /// 解析 USE 语句
    ///
    /// 语法: USE <database>
    fn parse_use(&mut self) -> QueryResult<Statement> {
        self.expect(Token::Use)?;
        let database = self.parse_identifier()?;
        Ok(Statement::Use(UseStatement { database }))
    }

    /// # Brief
    /// 解析 SHOW 语句
    ///
    /// 语法:
    /// - SHOW DATABASE: 列出所有数据库
    /// - SHOW COLLECTION: 列出当前数据库的所有集合
    /// - SHOW INDEX ON <collection>: 列出集合的索引
    /// - SHOW STATUS: 显示数据库状态
    /// - SHOW USERS: 列出所有用户
    fn parse_show(&mut self) -> QueryResult<Statement> {
        self.expect(Token::Show)?;
        match self.peek() {
            Some(Token::Database) => {
                self.next();
                Ok(Statement::ShowDatabases)
            }
            Some(Token::Collection) => {
                self.next();
                Ok(Statement::ShowCollections)
            }
            Some(Token::Index) => {
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
            Some(Token::Grants) => {
                self.next();
                let username = if self.skip_if(Token::From) {
                    Some(self.parse_string_literal("username")?)
                } else if matches!(self.peek(), Some(Token::String(_))) {
                    Some(self.parse_string_literal("username")?)
                } else {
                    None
                };
                Ok(Statement::ShowGrants(username))
            }
            _ => Err(QueryError::Syntax(
                "Expected DATABASE, COLLECTION, INDEX, STATUS, USERS, or GRANTS".to_string(),
            )),
        }
    }

    /// # Brief
    /// 解析 CREATE 语句
    ///
    /// 语法:
    /// - CREATE DATABASE <name>
    /// - CREATE COLLECTION <name>
    /// - CREATE [UNIQUE] [TEXT] INDEX <name> ON <collection> (fields)
    /// - CREATE USER <name> WITH PASSWORD <password> [ROLE roles]
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

    /// # Brief
    /// 解析 CREATE INDEX 语句
    ///
    /// 语法: CREATE [UNIQUE] [TEXT] INDEX <name> ON <collection> (field1 [ASC|DESC], field2, ...)
    /// - UNIQUE: 唯一索引
    /// - TEXT: 全文索引
    /// - 默认索引类型为 BTree
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

    /// # Brief
    /// 解析 CREATE USER 语句
    ///
    /// 语法: CREATE USER <username> WITH PASSWORD <password> [ROLE role1, role2, ...]
    fn parse_create_user(&mut self) -> QueryResult<Statement> {
        self.expect(Token::User)?;
        let username = self.parse_string_literal("username")?;
        self.expect(Token::With)?;
        self.expect(Token::Password)?;
        let password = self.parse_string_literal("password")?;

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

    /// # Brief
    /// 解析 DROP 语句
    ///
    /// 语法:
    /// - DROP DATABASE <name>
    /// - DROP COLLECTION <name>
    /// - DROP INDEX <name> ON <collection>
    /// - DROP USER <name>
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
                let name = self.parse_string_literal("username")?;
                Ok(Statement::DropUser(name))
            }
            _ => Err(QueryError::Syntax(
                "Expected DATABASE, COLLECTION, INDEX, or USER".to_string(),
            )),
        }
    }

    /// # Brief
    /// 解析 INSERT 语句
    ///
    /// 语法:
    /// - INSERT INTO <collection> {doc}
    /// - INSERT INTO <collection> [{doc1}, {doc2}, ...]
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

    /// # Brief
    /// 解析 FIND 语句
    ///
    /// 语法: FIND <collection> [WHERE expr] [SELECT fields] [ORDER BY fields] [LIMIT n] [SKIP n]
    /// - WHERE: 过滤条件
    /// - SELECT: 投影字段
    /// - ORDER BY: 排序
    /// - LIMIT: 限制返回数量
    /// - SKIP: 跳过记录数
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

    /// # Brief
    /// 解析 UPDATE 语句
    ///
    /// 语法: UPDATE <collection> SET field1 = value1, field2 += value2 [UNSET field3] [PUSH field4 = value4] [WHERE expr]
    /// - SET field = value: 设置字段值
    /// - SET field += value: 增加数值 ($inc)
    /// - UNSET field: 删除字段
    /// - PUSH field = value: 向数组添加元素
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

    /// # Brief
    /// 解析 DELETE 语句
    ///
    /// 语法: DELETE FROM <collection> [WHERE expr]
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

    /// # Brief
    /// 解析 AGGREGATE 语句
    ///
    /// 语法: AGGREGATE <collection> | stage1 | stage2 | ...
    /// - 使用管道符 | 分隔聚合阶段
    /// - 支持 MATCH, GROUP, SORT, LIMIT, SKIP, PROJECT, UNWIND 等阶段
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

    /// # Brief
    /// 解析单个聚合管道阶段
    ///
    /// 支持的阶段:
    /// - MATCH: 过滤文档
    /// - GROUP BY fields AS {accumulator}
    /// - SORT: 排序
    /// - LIMIT/SKIP: 分页
    /// - PROJECT: 投影
    /// - UNWIND: 展开数组
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

    /// # Brief
    /// 解析聚合函数
    ///
    /// 语法: FUNCTION(field)
    /// 支持的函数: COUNT, SUM, AVG, MIN, MAX, FIRST, LAST
    ///
    /// # Returns
    /// (聚合函数类型, 可选的字段名)
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

    /// # Brief
    /// 解析 GRANT 语句
    ///
    /// 语法: GRANT <privilege> ON <resource> TO <username>
    fn parse_grant(&mut self) -> QueryResult<Statement> {
        self.expect(Token::Grant)?;
        let privilege = self.parse_identifier()?;
        self.expect(Token::On)?;
        let resource = self.parse_identifier()?;
        self.expect(Token::To)?;
        let username = self.parse_string_literal("username")?;

        Ok(Statement::Grant(GrantStatement {
            privilege,
            resource,
            username,
        }))
    }

    /// # Brief
    /// 解析 REVOKE 语句
    ///
    /// 语法: REVOKE <privilege> ON <resource> FROM <username>
    fn parse_revoke(&mut self) -> QueryResult<Statement> {
        self.expect(Token::Revoke)?;
        let privilege = self.parse_identifier()?;
        self.expect(Token::On)?;
        let resource = self.parse_identifier()?;
        self.expect(Token::From)?;
        let username = self.parse_string_literal("username")?;

        Ok(Statement::Revoke(RevokeStatement {
            privilege,
            resource,
            username,
        }))
    }

    /// # Brief
    /// 解析 ALTER USER 语句
    ///
    /// 语法: ALTER USER <username> PASSWORD <new_password>
    fn parse_alter(&mut self) -> QueryResult<Statement> {
        self.expect(Token::Alter)?;
        self.expect(Token::User)?;
        let username = self.parse_string_literal("username")?;

        let mut password = None;
        let mut add_roles = None;
        let mut remove_roles = None;

        if self.skip_if(Token::Password) {
            password = Some(self.parse_string_literal("password")?);
        }

        Ok(Statement::AlterUser(AlterUserStatement {
            username,
            password,
            add_roles,
            remove_roles,
        }))
    }

    /// # Brief
    /// 解析 AI 功能语句(实验性)
    ///
    /// 语法:
    /// - AI QUERY "natural language query"
    /// - AI ANALYZE <collection>
    /// - AI SUGGEST INDEX <collection>
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

    /// # Brief
    /// 解析表达式(入口函数)
    ///
    /// 调用 parse_or_expression 开始递归下降解析。
    /// 表达式优先级从低到高: OR < AND < NOT < 比较 < 加减 < 乘除模 < 一元 < 主表达式
    fn parse_expression(&mut self) -> QueryResult<Expression> {
        self.parse_or_expression()
    }

    /// # Brief
    /// 解析 OR 表达式(优先级最低)
    ///
    /// 语法: expr1 OR expr2 OR expr3 ...
    /// 左结合,解析所有 OR 连接的 AND 表达式。
    fn parse_or_expression(&mut self) -> QueryResult<Expression> {
        let mut left = self.parse_and_expression()?;

        while self.skip_if(Token::Or) {
            let right = self.parse_and_expression()?;
            left = Expression::or(left, right);
        }

        Ok(left)
    }

    /// # Brief
    /// 解析 AND 表达式
    ///
    /// 语法: expr1 AND expr2 AND expr3 ...
    /// 左结合,解析所有 AND 连接的 NOT 表达式。
    fn parse_and_expression(&mut self) -> QueryResult<Expression> {
        let mut left = self.parse_not_expression()?;

        while self.skip_if(Token::And) {
            let right = self.parse_not_expression()?;
            left = Expression::and(left, right);
        }

        Ok(left)
    }

    /// # Brief
    /// 解析 NOT 表达式
    ///
    /// 语法: NOT expr
    /// 一元前缀操作符,右结合。
    fn parse_not_expression(&mut self) -> QueryResult<Expression> {
        if self.skip_if(Token::Not) {
            let expr = self.parse_comparison_expression()?;
            Ok(Expression::not(expr))
        } else {
            self.parse_comparison_expression()
        }
    }

    /// # Brief
    /// 解析比较表达式
    ///
    /// 支持的操作符:
    /// - 比较: =, !=, <>, <, <=, >, >=
    /// - IN: expr IN [values]
    /// - LIKE: expr LIKE "pattern"
    /// - BETWEEN: expr BETWEEN low AND high
    /// - IS NULL / IS NOT NULL
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

    /// # Brief
    /// 解析加减表达式
    ///
    /// 语法: expr1 + expr2 - expr3 ...
    /// 左结合,优先级高于比较操作符。
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

    /// # Brief
    /// 解析乘除模表达式
    ///
    /// 语法: expr1 * expr2 / expr3 % expr4 ...
    /// 左结合,优先级高于加减操作符。
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

    /// # Brief
    /// 解析一元表达式
    ///
    /// 语法: -expr
    /// 负号前缀操作符,右结合。
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

    /// # Brief
    /// 解析主表达式(优先级最高)
    ///
    /// 支持:
    /// - 括号表达式: (expr)
    /// - 字面量: true, false, null, 整数, 浮点数, 字符串
    /// - 数组字面量: [value1, value2, ...]
    /// - 文档字面量: {field1: value1, field2: value2, ...}
    /// - 字段引用: field 或 field.subfield
    /// - 函数调用: function(args)
    /// - EXISTS(field): 字段存在性检查
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

    /// # Brief
    /// 解析 BOML 值
    ///
    /// 支持:
    /// - 基本类型: 整数, 浮点数, 字符串, 布尔值, null
    /// - 数组: [value1, value2, ...]
    /// - 文档: {field1: value1, field2: value2, ...}
    ///
    /// # Returns
    /// BomlValue 实例
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

    /// # Brief
    /// 解析表达式列表
    ///
    /// 语法: [expr1, expr2, ...]
    /// 用于 IN 操作符的值列表。
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

    /// # Brief
    /// 解析数组字面量
    ///
    /// 语法: [value1, value2, ...]
    /// 返回 BOML 值的向量。
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

    /// # Brief
    /// 解析文档字面量
    ///
    /// 语法: {field1: value1, field2: value2, ...}
    /// 字段名可以是标识符或字符串。
    ///
    /// # Returns
    /// BomlValue::Document 实例
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

    /// # Brief
    /// 解析字段列表
    ///
    /// 语法: field1, field2, field3, ...
    /// 用于 SELECT 子句。
    fn parse_field_list(&mut self) -> QueryResult<Vec<String>> {
        let mut fields = Vec::new();
        fields.push(self.parse_identifier()?);
        while self.skip_if(Token::Comma) {
            fields.push(self.parse_identifier()?);
        }
        Ok(fields)
    }

    /// # Brief
    /// 解析排序字段列表
    ///
    /// 语法: field1 [ASC|DESC], field2 [ASC|DESC], ...
    /// - ASC: 升序(默认)
    /// - DESC: 降序
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

    /// # Brief
    /// 解析投影字段列表
    ///
    /// 语法: field1, field2, field3, ...
    /// 用于聚合管道的 PROJECT 阶段。
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

    /// # Brief
    /// 解析整数
    ///
    /// 期望下一个 Token 为 Integer 类型。
    ///
    /// # Returns
    /// i64 整数值
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

    #[test]
    fn test_parse_create_user_string() {
        let stmt = Parser::parse(r#"CREATE USER "alice" WITH PASSWORD "secret""#).unwrap();
        match stmt {
            Statement::CreateUser(create_user) => {
                assert_eq!(create_user.username, "alice");
                assert_eq!(create_user.password, "secret");
            }
            _ => panic!("Expected CreateUser statement"),
        }
    }
}
