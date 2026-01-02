use rustyline::Result;

pub struct MqlCompleter {
    keywords: Vec<&'static str>,
    commands: Vec<&'static str>,
}

impl MqlCompleter {
    pub fn new() -> Self {
        Self {
            keywords: vec![
                "FIND", "INSERT", "UPDATE", "DELETE", "INTO", "FROM", "WHERE",
                "SET", "AND", "OR", "NOT", "IN", "LIKE", "BETWEEN", "IS", "NULL",
                "SELECT", "ORDER", "BY", "ASC", "DESC", "LIMIT", "SKIP", "OFFSET",
                "CREATE", "DROP", "ALTER", "INDEX", "COLLECTION", "DATABASE",
                "SHOW", "USE", "STATUS", "USERS", "USER",
                "BEGIN", "COMMIT", "ROLLBACK", "TRANSACTION",
                "AGGREGATE", "MATCH", "GROUP", "SORT", "PROJECT", "LOOKUP",
                "UNWIND", "BUCKET", "AS", "ON", "UNIQUE", "TEXT", "TTL",
                "TRUE", "FALSE",
            ],
            commands: vec![
                "help", "exit", "quit", "clear", "status", "use",
            ],
        }
    }

    pub fn complete(&self, line: &str, pos: usize) -> Result<(usize, Vec<String>)> {
        let line_to_cursor = if pos <= line.len() {
            &line[..pos]
        } else {
            line
        };

        let word_start = line_to_cursor
            .char_indices()
            .rev()
            .find(|(_, c)| c.is_whitespace() || *c == '(' || *c == '{' || *c == '[' || *c == ',')
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(0);

        let prefix = if word_start <= line_to_cursor.len() {
            &line_to_cursor[word_start..]
        } else {
            ""
        };

        if prefix.is_empty() {
            return Ok((pos, vec![]));
        }

        let prefix_upper = prefix.to_uppercase();
        let mut matches: Vec<String> = Vec::new();

        for &keyword in &self.keywords {
            if keyword.starts_with(&prefix_upper) {
                if prefix.chars().next().map(|c| c.is_lowercase()).unwrap_or(false) {
                    matches.push(keyword.to_lowercase());
                } else {
                    matches.push(keyword.to_string());
                }
            }
        }

        if word_start == 0 {
            for &cmd in &self.commands {
                if cmd.starts_with(&prefix.to_lowercase()) {
                    matches.push(cmd.to_string());
                }
            }
        }

        let context_completions = self.context_completions(line_to_cursor, &prefix_upper);
        matches.extend(context_completions);

        matches.sort();
        matches.dedup();

        Ok((word_start, matches))
    }

    fn context_completions(&self, line: &str, _prefix: &str) -> Vec<String> {
        let upper = line.to_uppercase();
        let mut completions = Vec::new();

        if upper.ends_with("SHOW ") {
            completions.extend(vec![
                "DATABASE".to_string(),
                "COLLECTION".to_string(),
                "INDEX".to_string(),
                "STATUS".to_string(),
                "USERS".to_string(),
            ]);
        }

        if upper.ends_with("CREATE ") {
            completions.extend(vec![
                "COLLECTION".to_string(),
                "DATABASE".to_string(),
                "INDEX".to_string(),
                "UNIQUE".to_string(),
                "USER".to_string(),
            ]);
        }

        if upper.ends_with("DROP ") {
            completions.extend(vec![
                "COLLECTION".to_string(),
                "DATABASE".to_string(),
                "INDEX".to_string(),
                "USER".to_string(),
            ]);
        }

        if upper.ends_with("ORDER ") {
            completions.push("BY".to_string());
        }

        if upper.ends_with("GROUP ") {
            completions.push("BY".to_string());
        }

        if upper.ends_with("INSERT ") {
            completions.push("INTO".to_string());
        }

        if upper.ends_with("DELETE ") {
            completions.push("FROM".to_string());
        }

        completions
    }

    pub fn add_collection(&mut self, _name: &str) {
    }

    pub fn add_field(&mut self, _name: &str) {
    }
}

impl Default for MqlCompleter {
    fn default() -> Self {
        Self::new()
    }
}
