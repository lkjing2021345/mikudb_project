//! 过滤器模块
//!
//! 本模块实现 MQL 表达式求值和过滤逻辑:
//! - 布尔表达式求值 (AND, OR, NOT)
//! - 比较运算 (=, !=, <, <=, >, >=)
//! - 特殊运算符 (IN, BETWEEN, LIKE, IS NULL, EXISTS)
//! - 算术运算 (+, -, *, /, %)
//! - 内置函数 (UPPER, LOWER, LENGTH, ABS, FLOOR, CEIL, ROUND, COALESCE)
//! - 正则表达式匹配
//!
//! 求值规则:
//! - 字段路径支持嵌套(使用点分隔,如 "user.profile.name")
//! - 类型自动转换(Int32/Int64, Float64)
//! - 浮点数相等比较使用 EPSILON 精度
//! - Null 值排序始终在最前面

use crate::ast::*;
use crate::{QueryError, QueryResult};
use mikudb_boml::{BomlValue, Document};
use regex::Regex;

/// # Brief
/// 求值表达式为布尔值
///
/// 将表达式应用到文档上,返回 true/false 结果。
/// 支持:
/// - 字面量: true 返回 true, 其他值返回 true
/// - 字段引用: 字段存在且非 Null 返回 true
/// - 二元运算: 比较、逻辑运算
/// - 一元运算: NOT
/// - IN/BETWEEN/LIKE/IS NULL/EXISTS
/// - 函数调用
///
/// # Arguments
/// * `expr` - 表达式
/// * `doc` - 文档
///
/// # Returns
/// 布尔值结果
pub fn evaluate(expr: &Expression, doc: &Document) -> QueryResult<bool> {
    match expr {
        Expression::Literal(BomlValue::Boolean(b)) => Ok(*b),
        Expression::Literal(_) => Ok(true),

        // 字段存在性检查
        Expression::Field(path) => {
            let value = doc.get_path(path);
            Ok(!matches!(value, None | Some(BomlValue::Null)))
        }

        Expression::Binary { left, op, right } => {
            evaluate_binary(left, *op, right, doc)
        }

        Expression::Unary { op, expr } => match op {
            UnaryOp::Not => Ok(!evaluate(expr, doc)?),
            UnaryOp::Neg => Err(QueryError::TypeError(
                "Cannot negate in boolean context".to_string(),
            )),
        },

        // IN 运算符: value IN [list]
        Expression::In { expr, list } => {
            let value = evaluate_value(expr, doc)?;
            for item in list {
                let item_value = evaluate_value(item, doc)?;
                if values_equal(&value, &item_value) {
                    return Ok(true);
                }
            }
            Ok(false)
        }

        // BETWEEN 运算符: value BETWEEN low AND high
        Expression::Between { expr, low, high } => {
            let value = evaluate_value(expr, doc)?;
            let low_val = evaluate_value(low, doc)?;
            let high_val = evaluate_value(high, doc)?;
            Ok(compare_values(&value, &low_val) >= 0
                && compare_values(&value, &high_val) <= 0)
        }

        // LIKE 模式匹配: value LIKE "pattern"
        // % 匹配任意字符序列, _ 匹配单个字符
        Expression::Like { expr, pattern } => {
            let value = evaluate_value(expr, doc)?;
            if let BomlValue::String(s) = value {
                // 将 SQL LIKE 模式转换为正则表达式
                let regex_pattern = pattern
                    .replace('%', ".*")
                    .replace('_', ".");
                let regex = Regex::new(&format!("^{}$", regex_pattern))
                    .map_err(|e| QueryError::InvalidOperator(format!("Invalid pattern: {}", e)))?;
                Ok(regex.is_match(s.as_str()))
            } else {
                Ok(false)
            }
        }

        // IS NULL / IS NOT NULL
        Expression::IsNull { expr, negated } => {
            let value = evaluate_value(expr, doc)?;
            let is_null = matches!(value, BomlValue::Null);
            Ok(if *negated { !is_null } else { is_null })
        }

        // EXISTS(field): 字段存在性检查
        Expression::Exists { field, negated } => {
            let exists = doc.get_path(field).is_some();
            Ok(if *negated { !exists } else { exists })
        }

        Expression::Call { function, args } => {
            evaluate_function(function, args, doc)
        }

        Expression::Array(_) | Expression::Document(_) => Ok(true),
    }
}

/// # Brief
/// 求值二元运算表达式
///
/// 支持:
/// - 逻辑运算: AND, OR (短路求值)
/// - 比较运算: =, !=, <, <=, >, >=
/// - 正则匹配: ~ (Regex 运算符)
///
/// # Arguments
/// * `left` - 左操作数
/// * `op` - 二元操作符
/// * `right` - 右操作数
/// * `doc` - 文档
///
/// # Returns
/// 布尔值结果
fn evaluate_binary(
    left: &Expression,
    op: BinaryOp,
    right: &Expression,
    doc: &Document,
) -> QueryResult<bool> {
    match op {
        // 逻辑运算使用短路求值
        BinaryOp::And => Ok(evaluate(left, doc)? && evaluate(right, doc)?),
        BinaryOp::Or => Ok(evaluate(left, doc)? || evaluate(right, doc)?),
        _ => {
            let left_val = evaluate_value(left, doc)?;
            let right_val = evaluate_value(right, doc)?;

            match op {
                BinaryOp::Eq => Ok(values_equal(&left_val, &right_val)),
                BinaryOp::Ne => Ok(!values_equal(&left_val, &right_val)),
                BinaryOp::Lt => Ok(compare_values(&left_val, &right_val) < 0),
                BinaryOp::Le => Ok(compare_values(&left_val, &right_val) <= 0),
                BinaryOp::Gt => Ok(compare_values(&left_val, &right_val) > 0),
                BinaryOp::Ge => Ok(compare_values(&left_val, &right_val) >= 0),
                // 正则表达式匹配
                BinaryOp::Regex => {
                    if let (BomlValue::String(s), BomlValue::String(pattern)) =
                        (&left_val, &right_val)
                    {
                        let regex = Regex::new(pattern.as_str()).map_err(|e| {
                            QueryError::InvalidOperator(format!("Invalid regex: {}", e))
                        })?;
                        Ok(regex.is_match(s.as_str()))
                    } else {
                        Ok(false)
                    }
                }
                _ => Err(QueryError::InvalidOperator(format!(
                    "Operator {} not supported in filter",
                    op
                ))),
            }
        }
    }
}

/// # Brief
/// 求值表达式为 BOML 值
///
/// 将表达式计算为具体的值,用于比较和算术运算。
///
/// # Arguments
/// * `expr` - 表达式
/// * `doc` - 文档
///
/// # Returns
/// BomlValue 结果
fn evaluate_value(expr: &Expression, doc: &Document) -> QueryResult<BomlValue> {
    match expr {
        Expression::Literal(v) => Ok(v.clone()),
        // 字段路径解析(支持嵌套路径,如 "user.name")
        Expression::Field(path) => Ok(doc.get_path(path).cloned().unwrap_or(BomlValue::Null)),
        // 算术运算
        Expression::Binary { left, op, right } => {
            let left_val = evaluate_value(left, doc)?;
            let right_val = evaluate_value(right, doc)?;
            compute_arithmetic(&left_val, *op, &right_val)
        }
        // 一元运算
        Expression::Unary { op, expr } => {
            let val = evaluate_value(expr, doc)?;
            match op {
                UnaryOp::Neg => negate_value(&val),
                UnaryOp::Not => {
                    if let BomlValue::Boolean(b) = val {
                        Ok(BomlValue::Boolean(!b))
                    } else {
                        Err(QueryError::TypeError("Cannot negate non-boolean".to_string()))
                    }
                }
            }
        }
        Expression::Call { function, args } => {
            evaluate_function_value(function, args, doc)
        }
        // 数组字面量(递归求值所有元素)
        Expression::Array(items) => {
            let values: QueryResult<Vec<BomlValue>> = items
                .iter()
                .map(|e| evaluate_value(e, doc))
                .collect();
            Ok(BomlValue::Array(values?))
        }
        // 文档字面量(递归求值所有字段值)
        Expression::Document(fields) => {
            let mut map = indexmap::IndexMap::new();
            for (key, expr) in fields {
                let value = evaluate_value(expr, doc)?;
                map.insert(compact_str::CompactString::from(key.as_str()), value);
            }
            Ok(BomlValue::Document(map))
        }
        _ => Err(QueryError::TypeError(
            "Cannot evaluate expression as value".to_string(),
        )),
    }
}

/// # Brief
/// 判断两个 BOML 值是否相等
///
/// 相等规则:
/// - 浮点数使用 EPSILON 精度比较
/// - Int32/Int64 自动类型转换
/// - 数组按元素逐个比较
/// - 不同类型返回 false
///
/// # Arguments
/// * `a` - 第一个值
/// * `b` - 第二个值
///
/// # Returns
/// 是否相等
fn values_equal(a: &BomlValue, b: &BomlValue) -> bool {
    match (a, b) {
        (BomlValue::Null, BomlValue::Null) => true,
        (BomlValue::Boolean(a), BomlValue::Boolean(b)) => a == b,
        (BomlValue::Int32(a), BomlValue::Int32(b)) => a == b,
        (BomlValue::Int64(a), BomlValue::Int64(b)) => a == b,
        // Int32/Int64 混合比较
        (BomlValue::Int32(a), BomlValue::Int64(b)) => (*a as i64) == *b,
        (BomlValue::Int64(a), BomlValue::Int32(b)) => *a == (*b as i64),
        // 浮点数使用 EPSILON 精度
        (BomlValue::Float64(a), BomlValue::Float64(b)) => (a - b).abs() < f64::EPSILON,
        (BomlValue::String(a), BomlValue::String(b)) => a == b,
        (BomlValue::ObjectId(a), BomlValue::ObjectId(b)) => a == b,
        // 数组按元素递归比较
        (BomlValue::Array(a), BomlValue::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| values_equal(x, y))
        }
        _ => false,
    }
}

/// # Brief
/// 比较两个 BOML 值
///
/// 比较规则:
/// - Null < 所有其他值
/// - 同类型值按自然顺序比较
/// - Int32/Int64/Float64 混合比较
/// - 浮点数 NaN 视为 Equal
/// - 不同类型返回 0 (Equal)
///
/// # Arguments
/// * `a` - 第一个值
/// * `b` - 第二个值
///
/// # Returns
/// 比较结果: -1 (小于), 0 (等于), 1 (大于)
fn compare_values(a: &BomlValue, b: &BomlValue) -> i32 {
    match (a, b) {
        (BomlValue::Null, BomlValue::Null) => 0,
        (BomlValue::Null, _) => -1,
        (_, BomlValue::Null) => 1,

        (BomlValue::Int32(a), BomlValue::Int32(b)) => a.cmp(b) as i32,
        (BomlValue::Int64(a), BomlValue::Int64(b)) => a.cmp(b) as i32,
        // Int32/Int64 混合比较
        (BomlValue::Int32(a), BomlValue::Int64(b)) => (*a as i64).cmp(b) as i32,
        (BomlValue::Int64(a), BomlValue::Int32(b)) => a.cmp(&(*b as i64)) as i32,

        // 浮点数比较(NaN 视为 Equal)
        (BomlValue::Float64(a), BomlValue::Float64(b)) => {
            a.partial_cmp(b).map(|o| o as i32).unwrap_or(0)
        }
        // 整数与浮点数混合比较
        (BomlValue::Int32(a), BomlValue::Float64(b)) => {
            (*a as f64).partial_cmp(b).map(|o| o as i32).unwrap_or(0)
        }
        (BomlValue::Float64(a), BomlValue::Int32(b)) => {
            a.partial_cmp(&(*b as f64)).map(|o| o as i32).unwrap_or(0)
        }

        (BomlValue::String(a), BomlValue::String(b)) => a.cmp(b) as i32,

        (BomlValue::DateTime(a), BomlValue::DateTime(b)) => a.cmp(b) as i32,

        _ => 0,
    }
}

/// # Brief
/// 执行算术运算
///
/// 支持:
/// - 加法 (+): 数值相加, 字符串拼接
/// - 减法 (-)
/// - 乘法 (*)
/// - 除法 (/): 除零检查
/// - 取模 (%): 除零检查
/// - Int32/Int64/Float64 自动类型提升
///
/// # Arguments
/// * `a` - 左操作数
/// * `op` - 算术操作符
/// * `b` - 右操作数
///
/// # Returns
/// 计算结果
fn compute_arithmetic(a: &BomlValue, op: BinaryOp, b: &BomlValue) -> QueryResult<BomlValue> {
    match (a, b) {
        (BomlValue::Int32(a), BomlValue::Int32(b)) => {
            let result = match op {
                BinaryOp::Add => a + b,
                BinaryOp::Sub => a - b,
                BinaryOp::Mul => a * b,
                BinaryOp::Div => {
                    // 除零检查
                    if *b == 0 {
                        return Err(QueryError::Execution("Division by zero".to_string()));
                    }
                    a / b
                }
                BinaryOp::Mod => {
                    if *b == 0 {
                        return Err(QueryError::Execution("Division by zero".to_string()));
                    }
                    a % b
                }
                _ => return Err(QueryError::InvalidOperator(format!("Invalid operator: {}", op))),
            };
            Ok(BomlValue::Int32(result))
        }
        (BomlValue::Int64(a), BomlValue::Int64(b)) => {
            let result = match op {
                BinaryOp::Add => a + b,
                BinaryOp::Sub => a - b,
                BinaryOp::Mul => a * b,
                BinaryOp::Div => {
                    if *b == 0 {
                        return Err(QueryError::Execution("Division by zero".to_string()));
                    }
                    a / b
                }
                BinaryOp::Mod => {
                    if *b == 0 {
                        return Err(QueryError::Execution("Division by zero".to_string()));
                    }
                    a % b
                }
                _ => return Err(QueryError::InvalidOperator(format!("Invalid operator: {}", op))),
            };
            Ok(BomlValue::Int64(result))
        }
        (BomlValue::Float64(a), BomlValue::Float64(b)) => {
            let result = match op {
                BinaryOp::Add => a + b,
                BinaryOp::Sub => a - b,
                BinaryOp::Mul => a * b,
                BinaryOp::Div => a / b,  // 浮点数除法不检查除零(返回 Inf)
                BinaryOp::Mod => a % b,
                _ => return Err(QueryError::InvalidOperator(format!("Invalid operator: {}", op))),
            };
            Ok(BomlValue::Float64(result))
        }
        // 字符串拼接
        (BomlValue::String(a), BomlValue::String(b)) if op == BinaryOp::Add => {
            Ok(BomlValue::String(compact_str::CompactString::from(
                format!("{}{}", a, b),
            )))
        }
        _ => Err(QueryError::TypeError(format!(
            "Cannot perform {} on {:?} and {:?}",
            op,
            a.type_name(),
            b.type_name()
        ))),
    }
}

/// # Brief
/// 对数值取反
///
/// 支持 Int32, Int64, Float64。
///
/// # Arguments
/// * `v` - 数值
///
/// # Returns
/// 取反后的值
fn negate_value(v: &BomlValue) -> QueryResult<BomlValue> {
    match v {
        BomlValue::Int32(n) => Ok(BomlValue::Int32(-n)),
        BomlValue::Int64(n) => Ok(BomlValue::Int64(-n)),
        BomlValue::Float64(n) => Ok(BomlValue::Float64(-n)),
        _ => Err(QueryError::TypeError(format!(
            "Cannot negate {:?}",
            v.type_name()
        ))),
    }
}

/// # Brief
/// 在布尔上下文中求值函数(不支持)
///
/// 函数调用应返回值,不应在布尔上下文中使用。
fn evaluate_function(name: &str, _args: &[Expression], _doc: &Document) -> QueryResult<bool> {
    Err(QueryError::Execution(format!(
        "Function {} not supported in boolean context",
        name
    )))
}

/// # Brief
/// 求值函数调用为 BOML 值
///
/// 支持的函数:
/// - 字符串函数: UPPER, LOWER, LENGTH
/// - 数学函数: ABS, FLOOR, CEIL, ROUND
/// - 工具函数: COALESCE (返回第一个非 Null 值)
///
/// # Arguments
/// * `name` - 函数名(大小写不敏感)
/// * `args` - 参数列表
/// * `doc` - 文档
///
/// # Returns
/// 函数返回值
fn evaluate_function_value(
    name: &str,
    args: &[Expression],
    doc: &Document,
) -> QueryResult<BomlValue> {
    let name_lower = name.to_lowercase();
    match name_lower.as_str() {
        // 字符串转大写
        "upper" | "toupper" => {
            if args.len() != 1 {
                return Err(QueryError::Execution("UPPER requires 1 argument".to_string()));
            }
            let val = evaluate_value(&args[0], doc)?;
            if let BomlValue::String(s) = val {
                Ok(BomlValue::String(compact_str::CompactString::from(
                    s.to_uppercase(),
                )))
            } else {
                Err(QueryError::TypeError("UPPER requires string argument".to_string()))
            }
        }
        // 字符串转小写
        "lower" | "tolower" => {
            if args.len() != 1 {
                return Err(QueryError::Execution("LOWER requires 1 argument".to_string()));
            }
            let val = evaluate_value(&args[0], doc)?;
            if let BomlValue::String(s) = val {
                Ok(BomlValue::String(compact_str::CompactString::from(
                    s.to_lowercase(),
                )))
            } else {
                Err(QueryError::TypeError("LOWER requires string argument".to_string()))
            }
        }
        // 字符串或数组长度
        "length" | "len" => {
            if args.len() != 1 {
                return Err(QueryError::Execution("LENGTH requires 1 argument".to_string()));
            }
            let val = evaluate_value(&args[0], doc)?;
            match val {
                BomlValue::String(s) => Ok(BomlValue::Int64(s.len() as i64)),
                BomlValue::Array(a) => Ok(BomlValue::Int64(a.len() as i64)),
                _ => Err(QueryError::TypeError(
                    "LENGTH requires string or array argument".to_string(),
                )),
            }
        }
        // 绝对值
        "abs" => {
            if args.len() != 1 {
                return Err(QueryError::Execution("ABS requires 1 argument".to_string()));
            }
            let val = evaluate_value(&args[0], doc)?;
            match val {
                BomlValue::Int32(n) => Ok(BomlValue::Int32(n.abs())),
                BomlValue::Int64(n) => Ok(BomlValue::Int64(n.abs())),
                BomlValue::Float64(n) => Ok(BomlValue::Float64(n.abs())),
                _ => Err(QueryError::TypeError("ABS requires numeric argument".to_string())),
            }
        }
        // 向下取整
        "floor" => {
            if args.len() != 1 {
                return Err(QueryError::Execution("FLOOR requires 1 argument".to_string()));
            }
            let val = evaluate_value(&args[0], doc)?;
            match val {
                BomlValue::Float64(n) => Ok(BomlValue::Float64(n.floor())),
                BomlValue::Int32(n) => Ok(BomlValue::Int32(n)),
                BomlValue::Int64(n) => Ok(BomlValue::Int64(n)),
                _ => Err(QueryError::TypeError("FLOOR requires numeric argument".to_string())),
            }
        }
        // 向上取整
        "ceil" => {
            if args.len() != 1 {
                return Err(QueryError::Execution("CEIL requires 1 argument".to_string()));
            }
            let val = evaluate_value(&args[0], doc)?;
            match val {
                BomlValue::Float64(n) => Ok(BomlValue::Float64(n.ceil())),
                BomlValue::Int32(n) => Ok(BomlValue::Int32(n)),
                BomlValue::Int64(n) => Ok(BomlValue::Int64(n)),
                _ => Err(QueryError::TypeError("CEIL requires numeric argument".to_string())),
            }
        }
        // 四舍五入
        "round" => {
            if args.len() != 1 {
                return Err(QueryError::Execution("ROUND requires 1 argument".to_string()));
            }
            let val = evaluate_value(&args[0], doc)?;
            match val {
                BomlValue::Float64(n) => Ok(BomlValue::Float64(n.round())),
                BomlValue::Int32(n) => Ok(BomlValue::Int32(n)),
                BomlValue::Int64(n) => Ok(BomlValue::Int64(n)),
                _ => Err(QueryError::TypeError("ROUND requires numeric argument".to_string())),
            }
        }
        // 返回第一个非 Null 值
        "coalesce" => {
            for arg in args {
                let val = evaluate_value(arg, doc)?;
                if !matches!(val, BomlValue::Null) {
                    return Ok(val);
                }
            }
            Ok(BomlValue::Null)
        }
        _ => Err(QueryError::Execution(format!("Unknown function: {}", name))),
    }
}

/// 过滤器
///
/// 封装表达式,提供文档匹配和批量过滤功能。
pub struct Filter {
    /// 过滤表达式
    expression: Expression,
}

impl Filter {
    /// # Brief
    /// 创建过滤器
    ///
    /// # Arguments
    /// * `expression` - 过滤表达式
    pub fn new(expression: Expression) -> Self {
        Self { expression }
    }

    /// # Brief
    /// 判断文档是否匹配过滤条件
    ///
    /// # Arguments
    /// * `doc` - 文档
    ///
    /// # Returns
    /// 是否匹配
    pub fn matches(&self, doc: &Document) -> QueryResult<bool> {
        evaluate(&self.expression, doc)
    }

    /// # Brief
    /// 批量过滤文档
    ///
    /// 返回匹配过滤条件的文档迭代器。
    /// 不匹配的文档被跳过,求值错误作为 Err 返回。
    ///
    /// # Arguments
    /// * `docs` - 文档迭代器
    ///
    /// # Returns
    /// 过滤后的文档迭代器
    pub fn filter_documents<'a>(
        &self,
        docs: impl Iterator<Item = Document> + 'a,
    ) -> impl Iterator<Item = QueryResult<Document>> + 'a
    where
        Self: 'a,
    {
        let expr = self.expression.clone();
        docs.filter_map(move |doc| {
            match evaluate(&expr, &doc) {
                Ok(true) => Some(Ok(doc)),   // 匹配,返回文档
                Ok(false) => None,            // 不匹配,跳过
                Err(e) => Some(Err(e)),       // 错误,传播
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mikudb_boml::Document;

    fn make_doc() -> Document {
        let mut doc = Document::new();
        doc.insert("name", "Alice");
        doc.insert("age", 30);
        doc.insert("active", true);
        doc
    }

    #[test]
    fn test_field_equality() {
        let doc = make_doc();
        let expr = Expression::eq(
            Expression::field("name"),
            Expression::literal("Alice"),
        );
        assert!(evaluate(&expr, &doc).unwrap());
    }

    #[test]
    fn test_numeric_comparison() {
        let doc = make_doc();
        let expr = Expression::gt(Expression::field("age"), Expression::literal(25));
        assert!(evaluate(&expr, &doc).unwrap());
    }

    #[test]
    fn test_and_expression() {
        let doc = make_doc();
        let expr = Expression::and(
            Expression::eq(Expression::field("name"), Expression::literal("Alice")),
            Expression::eq(Expression::field("active"), Expression::literal(true)),
        );
        assert!(evaluate(&expr, &doc).unwrap());
    }

    #[test]
    fn test_or_expression() {
        let doc = make_doc();
        let expr = Expression::or(
            Expression::eq(Expression::field("name"), Expression::literal("Bob")),
            Expression::eq(Expression::field("age"), Expression::literal(30)),
        );
        assert!(evaluate(&expr, &doc).unwrap());
    }

    #[test]
    fn test_in_expression() {
        let doc = make_doc();
        let expr = Expression::In {
            expr: Box::new(Expression::field("age")),
            list: vec![
                Expression::literal(25),
                Expression::literal(30),
                Expression::literal(35),
            ],
        };
        assert!(evaluate(&expr, &doc).unwrap());
    }
}
