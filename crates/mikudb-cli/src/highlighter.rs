use colored::Colorize;

pub struct MqlHighlighter {
    keywords: Vec<&'static str>,
    functions: Vec<&'static str>,
    operators: Vec<&'static str>,
}

impl MqlHighlighter {
    pub fn new() -> Self {
        Self {
            keywords: vec![
                "FIND", "INSERT", "UPDATE", "DELETE", "INTO", "FROM", "WHERE",
                "SET", "AND", "OR", "NOT", "IN", "LIKE", "BETWEEN", "IS", "NULL",
                "SELECT", "ORDER", "BY", "ASC", "DESC", "LIMIT", "SKIP", "OFFSET",
                "CREATE", "DROP", "ALTER", "INDEX", "COLLECTION", "DATABASE",
                "SHOW", "USE", "DATABASES", "COLLECTIONS", "INDEXES",
                "BEGIN", "COMMIT", "ROLLBACK", "TRANSACTION",
                "AGGREGATE", "MATCH", "GROUP", "SORT", "PROJECT", "LOOKUP",
                "UNWIND", "BUCKET", "AS", "ON", "UNIQUE", "TEXT", "TTL",
                "TRUE", "FALSE",
            ],
            functions: vec![
                "COUNT", "SUM", "AVG", "MIN", "MAX", "FIRST", "LAST",
                "PUSH", "PULL", "ADDTOSET", "POP", "UNSET", "INC", "MUL",
                "NOW", "DATE", "YEAR", "MONTH", "DAY", "HOUR", "MINUTE", "SECOND",
                "UPPER", "LOWER", "TRIM", "SUBSTR", "CONCAT", "SPLIT",
                "SIZE", "TYPE", "OBJECTID",
            ],
            operators: vec![
                "=", "!=", "<>", "<", ">", "<=", ">=", "+", "-", "*", "/", "%",
            ],
        }
    }

    pub fn highlight(&self, input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars().peekable();
        let mut current_word = String::new();

        while let Some(ch) = chars.next() {
            if ch == '"' || ch == '\'' {
                if !current_word.is_empty() {
                    result.push_str(&self.highlight_word(&current_word));
                    current_word.clear();
                }

                let quote = ch;
                let mut string_content = String::new();
                string_content.push(quote);

                while let Some(c) = chars.next() {
                    string_content.push(c);
                    if c == quote {
                        break;
                    }
                    if c == '\\' {
                        if let Some(escaped) = chars.next() {
                            string_content.push(escaped);
                        }
                    }
                }

                result.push_str(&string_content.yellow().to_string());
            } else if ch == '{' || ch == '}' || ch == '[' || ch == ']' {
                if !current_word.is_empty() {
                    result.push_str(&self.highlight_word(&current_word));
                    current_word.clear();
                }
                result.push_str(&ch.to_string().magenta().bold().to_string());
            } else if ch == ':' || ch == ',' {
                if !current_word.is_empty() {
                    result.push_str(&self.highlight_word(&current_word));
                    current_word.clear();
                }
                result.push(ch);
            } else if ch.is_whitespace() {
                if !current_word.is_empty() {
                    result.push_str(&self.highlight_word(&current_word));
                    current_word.clear();
                }
                result.push(ch);
            } else if self.is_operator_char(ch) {
                if !current_word.is_empty() {
                    result.push_str(&self.highlight_word(&current_word));
                    current_word.clear();
                }

                let mut op = String::new();
                op.push(ch);
                while let Some(&next) = chars.peek() {
                    if self.is_operator_char(next) {
                        op.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                result.push_str(&op.red().to_string());
            } else {
                current_word.push(ch);
            }
        }

        if !current_word.is_empty() {
            result.push_str(&self.highlight_word(&current_word));
        }

        result
    }

    fn highlight_word(&self, word: &str) -> String {
        let upper = word.to_uppercase();

        if self.keywords.contains(&upper.as_str()) {
            return word.blue().bold().to_string();
        }

        if self.functions.contains(&upper.as_str()) {
            return word.cyan().to_string();
        }

        if word.starts_with('$') {
            return word.green().to_string();
        }

        if word.parse::<f64>().is_ok() {
            return word.bright_magenta().to_string();
        }

        word.to_string()
    }

    fn is_operator_char(&self, ch: char) -> bool {
        matches!(ch, '=' | '!' | '<' | '>' | '+' | '-' | '*' | '/' | '%')
    }
}

impl Default for MqlHighlighter {
    fn default() -> Self {
        Self::new()
    }
}
