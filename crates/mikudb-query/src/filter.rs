use crate::ast::*;
use crate::{QueryError, QueryResult};
use mikudb_boml::{BomlValue, Document};
use regex::Regex;

pub fn evaluate(expr: &Expression, doc: &Document) -> QueryResult<bool> {
    match expr {
        Expression::Literal(BomlValue::Boolean(b)) => Ok(*b),
        Expression::Literal(_) => Ok(true),

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

        Expression::Between { expr, low, high } => {
            let value = evaluate_value(expr, doc)?;
            let low_val = evaluate_value(low, doc)?;
            let high_val = evaluate_value(high, doc)?;
            Ok(compare_values(&value, &low_val) >= 0
                && compare_values(&value, &high_val) <= 0)
        }

        Expression::Like { expr, pattern } => {
            let value = evaluate_value(expr, doc)?;
            if let BomlValue::String(s) = value {
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

        Expression::IsNull { expr, negated } => {
            let value = evaluate_value(expr, doc)?;
            let is_null = matches!(value, BomlValue::Null);
            Ok(if *negated { !is_null } else { is_null })
        }

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

fn evaluate_binary(
    left: &Expression,
    op: BinaryOp,
    right: &Expression,
    doc: &Document,
) -> QueryResult<bool> {
    match op {
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

fn evaluate_value(expr: &Expression, doc: &Document) -> QueryResult<BomlValue> {
    match expr {
        Expression::Literal(v) => Ok(v.clone()),
        Expression::Field(path) => Ok(doc.get_path(path).cloned().unwrap_or(BomlValue::Null)),
        Expression::Binary { left, op, right } => {
            let left_val = evaluate_value(left, doc)?;
            let right_val = evaluate_value(right, doc)?;
            compute_arithmetic(&left_val, *op, &right_val)
        }
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
        Expression::Array(items) => {
            let values: QueryResult<Vec<BomlValue>> = items
                .iter()
                .map(|e| evaluate_value(e, doc))
                .collect();
            Ok(BomlValue::Array(values?))
        }
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

fn values_equal(a: &BomlValue, b: &BomlValue) -> bool {
    match (a, b) {
        (BomlValue::Null, BomlValue::Null) => true,
        (BomlValue::Boolean(a), BomlValue::Boolean(b)) => a == b,
        (BomlValue::Int32(a), BomlValue::Int32(b)) => a == b,
        (BomlValue::Int64(a), BomlValue::Int64(b)) => a == b,
        (BomlValue::Int32(a), BomlValue::Int64(b)) => (*a as i64) == *b,
        (BomlValue::Int64(a), BomlValue::Int32(b)) => *a == (*b as i64),
        (BomlValue::Float64(a), BomlValue::Float64(b)) => (a - b).abs() < f64::EPSILON,
        (BomlValue::String(a), BomlValue::String(b)) => a == b,
        (BomlValue::ObjectId(a), BomlValue::ObjectId(b)) => a == b,
        (BomlValue::Array(a), BomlValue::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| values_equal(x, y))
        }
        _ => false,
    }
}

fn compare_values(a: &BomlValue, b: &BomlValue) -> i32 {
    match (a, b) {
        (BomlValue::Null, BomlValue::Null) => 0,
        (BomlValue::Null, _) => -1,
        (_, BomlValue::Null) => 1,

        (BomlValue::Int32(a), BomlValue::Int32(b)) => a.cmp(b) as i32,
        (BomlValue::Int64(a), BomlValue::Int64(b)) => a.cmp(b) as i32,
        (BomlValue::Int32(a), BomlValue::Int64(b)) => (*a as i64).cmp(b) as i32,
        (BomlValue::Int64(a), BomlValue::Int32(b)) => a.cmp(&(*b as i64)) as i32,

        (BomlValue::Float64(a), BomlValue::Float64(b)) => {
            a.partial_cmp(b).map(|o| o as i32).unwrap_or(0)
        }
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

fn compute_arithmetic(a: &BomlValue, op: BinaryOp, b: &BomlValue) -> QueryResult<BomlValue> {
    match (a, b) {
        (BomlValue::Int32(a), BomlValue::Int32(b)) => {
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
                BinaryOp::Div => a / b,
                BinaryOp::Mod => a % b,
                _ => return Err(QueryError::InvalidOperator(format!("Invalid operator: {}", op))),
            };
            Ok(BomlValue::Float64(result))
        }
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

fn evaluate_function(name: &str, _args: &[Expression], _doc: &Document) -> QueryResult<bool> {
    Err(QueryError::Execution(format!(
        "Function {} not supported in boolean context",
        name
    )))
}

fn evaluate_function_value(
    name: &str,
    args: &[Expression],
    doc: &Document,
) -> QueryResult<BomlValue> {
    let name_lower = name.to_lowercase();
    match name_lower.as_str() {
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

pub struct Filter {
    expression: Expression,
}

impl Filter {
    pub fn new(expression: Expression) -> Self {
        Self { expression }
    }

    pub fn matches(&self, doc: &Document) -> QueryResult<bool> {
        evaluate(&self.expression, doc)
    }

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
                Ok(true) => Some(Ok(doc)),
                Ok(false) => None,
                Err(e) => Some(Err(e)),
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
