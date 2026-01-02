//! MQL 词法分析器
//!
//! 本模块使用 logos 库实现高性能词法分析:
//! - 自动跳过空白字符和注释
//! - 支持单行注释(//)和多行注释(/* */)
//! - 大小写不敏感的关键字
//! - 字符串、数字、标识符的词法规则
//! - 所有 MQL 操作符和符号

use logos::Logos;

/// MQL Token
///
/// 定义 MQL 的所有词法单元,使用 logos 宏自动生成词法分析器。
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(skip r"[ \t\r\n]+")]          // 跳过空白字符
#[logos(skip r"//[^\n]*")]            // 跳过单行注释
#[logos(skip r"/\*[^*]*\*+([^/*][^*]*\*+)*/")]  // 跳过多行注释
pub enum Token {
    // 数据库管理关键字
    #[token("USE", ignore(ascii_case))]
    Use,
    #[token("SHOW", ignore(ascii_case))]
    Show,
    #[token("CREATE", ignore(ascii_case))]
    Create,
    #[token("DROP", ignore(ascii_case))]
    Drop,
    #[token("DATABASE", ignore(ascii_case))]
    Database,
    #[token("COLLECTION", ignore(ascii_case))]
    Collection,
    #[token("INDEX", ignore(ascii_case))]
    Index,
    #[token("UNIQUE", ignore(ascii_case))]
    Unique,
    #[token("TEXT", ignore(ascii_case))]
    Text,
    #[token("ON", ignore(ascii_case))]
    On,

    // CRUD 操作关键字
    #[token("INSERT", ignore(ascii_case))]
    Insert,
    #[token("INTO", ignore(ascii_case))]
    Into,
    #[token("FIND", ignore(ascii_case))]
    Find,
    #[token("UPDATE", ignore(ascii_case))]
    Update,
    #[token("DELETE", ignore(ascii_case))]
    Delete,
    #[token("FROM", ignore(ascii_case))]
    From,

    // 查询子句关键字
    #[token("WHERE", ignore(ascii_case))]
    Where,
    #[token("SELECT", ignore(ascii_case))]
    Select,
    #[token("ORDER", ignore(ascii_case))]
    Order,
    #[token("BY", ignore(ascii_case))]
    By,
    #[token("ASC", ignore(ascii_case))]
    Asc,
    #[token("DESC", ignore(ascii_case))]
    Desc,
    #[token("LIMIT", ignore(ascii_case))]
    Limit,
    #[token("SKIP", ignore(ascii_case))]
    Skip,
    #[token("SET", ignore(ascii_case))]
    Set,
    #[token("UNSET", ignore(ascii_case))]
    Unset,
    #[token("PUSH", ignore(ascii_case))]
    Push,
    #[token("PULL", ignore(ascii_case))]
    Pull,

    // 聚合管道关键字
    #[token("AGGREGATE", ignore(ascii_case))]
    Aggregate,
    #[token("MATCH", ignore(ascii_case))]
    Match,
    #[token("GROUP", ignore(ascii_case))]
    Group,
    #[token("SORT", ignore(ascii_case))]
    Sort,
    #[token("PROJECT", ignore(ascii_case))]
    Project,
    #[token("UNWIND", ignore(ascii_case))]
    Unwind,
    #[token("LOOKUP", ignore(ascii_case))]
    Lookup,
    #[token("AS", ignore(ascii_case))]
    As,

    // 逻辑操作符
    #[token("AND", ignore(ascii_case))]
    And,
    #[token("OR", ignore(ascii_case))]
    Or,
    #[token("NOT", ignore(ascii_case))]
    Not,
    #[token("IN", ignore(ascii_case))]
    In,
    #[token("LIKE", ignore(ascii_case))]
    Like,
    #[token("BETWEEN", ignore(ascii_case))]
    Between,
    #[token("IS", ignore(ascii_case))]
    Is,
    #[token("NULL", ignore(ascii_case))]
    Null,
    #[token("EXISTS", ignore(ascii_case))]
    Exists,

    // 事务关键字
    #[token("BEGIN", ignore(ascii_case))]
    Begin,
    #[token("TRANSACTION", ignore(ascii_case))]
    Transaction,
    #[token("COMMIT", ignore(ascii_case))]
    Commit,
    #[token("ROLLBACK", ignore(ascii_case))]
    Rollback,

    // AI 功能关键字(实验性)
    #[token("AI", ignore(ascii_case))]
    Ai,
    #[token("QUERY", ignore(ascii_case))]
    Query,
    #[token("ANALYZE", ignore(ascii_case))]
    Analyze,
    #[token("SUGGEST", ignore(ascii_case))]
    Suggest,

    // 系统命令关键字
    #[token("STATUS", ignore(ascii_case))]
    Status,
    #[token("USERS", ignore(ascii_case))]
    Users,
    #[token("USER", ignore(ascii_case))]
    User,
    #[token("WITH", ignore(ascii_case))]
    With,
    #[token("PASSWORD", ignore(ascii_case))]
    Password,
    #[token("ROLE", ignore(ascii_case))]
    Role,
    #[token("GRANT", ignore(ascii_case))]
    Grant,
    #[token("REVOKE", ignore(ascii_case))]
    Revoke,
    #[token("TO", ignore(ascii_case))]
    To,

    // 聚合函数关键字
    #[token("COUNT", ignore(ascii_case))]
    Count,
    #[token("SUM", ignore(ascii_case))]
    Sum,
    #[token("AVG", ignore(ascii_case))]
    Avg,
    #[token("MIN", ignore(ascii_case))]
    Min,
    #[token("MAX", ignore(ascii_case))]
    Max,
    #[token("FIRST", ignore(ascii_case))]
    First,
    #[token("LAST", ignore(ascii_case))]
    Last,

    // 布尔字面量
    #[token("true", ignore(ascii_case))]
    True,
    #[token("false", ignore(ascii_case))]
    False,

    // 比较操作符
    #[token("=")]
    Eq,
    #[token("!=")]
    Ne,
    #[token("<>")]
    Ne2,
    #[token("<")]
    Lt,
    #[token("<=")]
    Le,
    #[token(">")]
    Gt,
    #[token(">=")]
    Ge,
    #[token("+=")]
    PlusEq,
    #[token("-=")]
    MinusEq,

    // 算术操作符
    #[token("+")]
    Plus,
    #[token("-")]
    Minus,
    #[token("*")]
    Star,
    #[token("/")]
    Slash,
    #[token("%")]
    Percent,

    // 分隔符和括号
    #[token("(")]
    LParen,
    #[token(")")]
    RParen,
    #[token("{")]
    LBrace,
    #[token("}")]
    RBrace,
    #[token("[")]
    LBracket,
    #[token("]")]
    RBracket,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,
    #[token(";")]
    Semicolon,
    #[token(".")]
    Dot,
    #[token("|")]
    Pipe,
    #[token("$")]
    Dollar,

    // 字符串字面量(支持双引号和单引号,转义字符)
    #[regex(r#""([^"\\]|\\.)*""#, |lex| {
        let s = lex.slice();
        s[1..s.len()-1].to_string()
    })]
    #[regex(r#"'([^'\\]|\\.)*'"#, |lex| {
        let s = lex.slice();
        s[1..s.len()-1].to_string()
    })]
    String(String),

    // 浮点数(支持科学计数法)
    #[regex(r"-?[0-9]+\.[0-9]+([eE][+-]?[0-9]+)?", |lex| lex.slice().parse::<f64>().ok())]
    Float(f64),

    // 整数(优先级高于浮点数)
    #[regex(r"-?[0-9]+", |lex| lex.slice().parse::<i64>().ok(), priority = 2)]
    Integer(i64),

    // 标识符(字段名、集合名等)
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Identifier(String),

    // 引号标识符(用于包含特殊字符的名称)
    #[regex(r"`[^`]+`", |lex| {
        let s = lex.slice();
        s[1..s.len()-1].to_string()
    })]
    QuotedIdentifier(String),
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::String(s) => write!(f, "\"{}\"", s),
            Token::Float(n) => write!(f, "{}", n),
            Token::Integer(n) => write!(f, "{}", n),
            Token::Identifier(s) => write!(f, "{}", s),
            Token::QuotedIdentifier(s) => write!(f, "`{}`", s),
            _ => write!(f, "{:?}", self),
        }
    }
}

/// 词法分析器
///
/// 封装 logos 生成的词法分析器,提供迭代器接口。
pub struct Lexer<'a> {
    /// logos 生成的底层词法分析器
    inner: logos::Lexer<'a, Token>,
}

impl<'a> Lexer<'a> {
    /// # Brief
    /// 创建词法分析器
    ///
    /// # Arguments
    /// * `input` - 输入的 MQL 查询字符串
    pub fn new(input: &'a str) -> Self {
        Self {
            inner: Token::lexer(input),
        }
    }

    /// # Brief
    /// 一次性词法分析整个输入
    ///
    /// 返回所有 Token 及其位置范围,用于调试和测试。
    ///
    /// # Arguments
    /// * `input` - 输入字符串
    ///
    /// # Returns
    /// Token 和位置范围的列表
    pub fn tokenize(input: &str) -> Vec<(Token, std::ops::Range<usize>)> {
        let lexer = Token::lexer(input);
        lexer
            .spanned()
            .filter_map(|(result, span)| result.ok().map(|token| (token, span)))
            .collect()
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Token, ()>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|r| r.map_err(|_| ()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_tokens() {
        let tokens = Lexer::tokenize("FIND users WHERE age > 18");
        assert_eq!(tokens.len(), 5);
        assert_eq!(tokens[0].0, Token::Find);
        assert!(matches!(tokens[1].0, Token::Identifier(_)));
        assert_eq!(tokens[2].0, Token::Where);
    }

    #[test]
    fn test_string_literal() {
        let tokens = Lexer::tokenize(r#"INSERT INTO users {"name": "test"}"#);
        assert!(tokens.iter().any(|(t, _)| matches!(t, Token::String(s) if s == "name")));
        assert!(tokens.iter().any(|(t, _)| matches!(t, Token::String(s) if s == "test")));
    }

    #[test]
    fn test_numbers() {
        let tokens = Lexer::tokenize("LIMIT 10 SKIP 3.14");
        assert!(tokens.iter().any(|(t, _)| matches!(t, Token::Integer(10))));
        assert!(tokens.iter().any(|(t, _)| matches!(t, Token::Float(n) if (*n - 3.14).abs() < 0.001)));
    }
}
