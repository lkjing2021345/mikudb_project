use crate::client::Client;
use crate::completer::MqlCompleter;
use crate::formatter::Formatter;
use crate::highlighter::MqlHighlighter;
use crate::{CliError, CliResult, Config};
use colored::Colorize;
use rustyline::config::Configurer;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::{CompletionType, EditMode, Editor};
use std::borrow::Cow;

pub struct Repl {
    client: Client,
    formatter: Formatter,
    editor: Editor<MqlHelper, DefaultHistory>,
    current_database: Option<String>,
    history_file: String,
}

#[derive(rustyline_derive::Helper)]
struct MqlHelper {
    completer: MqlCompleter,
    highlighter: MqlHighlighter,
}

impl rustyline::completion::Completer for MqlHelper {
    type Candidate = String;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<String>)> {
        self.completer.complete(line, pos)
    }
}

impl rustyline::hint::Hinter for MqlHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &rustyline::Context<'_>) -> Option<String> {
        if line.is_empty() || pos < line.len() {
            return None;
        }

        let hints = [
            ("FIND", " collection_name WHERE field = value"),
            ("INSERT", " INTO collection_name {field: value}"),
            ("UPDATE", " collection_name SET field = value WHERE condition"),
            ("DELETE", " FROM collection_name WHERE condition"),
            ("CREATE", " COLLECTION collection_name"),
            ("DROP", " COLLECTION collection_name"),
            ("SHOW", " COLLECTION"),
            ("USE", " database_name"),
        ];

        let upper = line.to_uppercase();
        for (prefix, hint) in hints {
            if prefix.starts_with(&upper) && upper.len() < prefix.len() {
                return Some(format!("{}{}", &prefix[upper.len()..], hint));
            }
            if upper == prefix {
                return Some(hint.to_string());
            }
        }

        None
    }
}

impl rustyline::highlight::Highlighter for MqlHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        Cow::Owned(self.highlighter.highlight(line))
    }

    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        _default: bool,
    ) -> Cow<'b, str> {
        Cow::Owned(prompt.green().bold().to_string())
    }

    fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
        Cow::Owned(hint.dimmed().to_string())
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _forced: bool) -> bool {
        true
    }
}

impl rustyline::validate::Validator for MqlHelper {
    fn validate(
        &self,
        ctx: &mut rustyline::validate::ValidationContext,
    ) -> rustyline::Result<rustyline::validate::ValidationResult> {
        let input = ctx.input();

        let open_braces = input.matches('{').count();
        let close_braces = input.matches('}').count();
        let open_brackets = input.matches('[').count();
        let close_brackets = input.matches(']').count();

        if open_braces != close_braces || open_brackets != close_brackets {
            Ok(rustyline::validate::ValidationResult::Incomplete)
        } else {
            Ok(rustyline::validate::ValidationResult::Valid(None))
        }
    }
}

impl Repl {
    pub async fn new(config: Config) -> CliResult<Self> {
        let client = Client::connect(&config).await?;
        let formatter = Formatter::new(&config.format, config.color);

        let helper = MqlHelper {
            completer: MqlCompleter::new(),
            highlighter: MqlHighlighter::new(),
        };

        let mut editor = Editor::new().map_err(|e| CliError::Other(e.to_string()))?;
        editor.set_helper(Some(helper));
        editor.set_completion_type(CompletionType::List);
        editor.set_edit_mode(EditMode::Emacs);

        let history_file = dirs::home_dir()
            .map(|h| h.join(".mikudb_history"))
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ".mikudb_history".to_string());

        let _ = editor.load_history(&history_file);

        Ok(Self {
            client,
            formatter,
            editor,
            current_database: config.database,
            history_file,
        })
    }

    pub async fn run(&mut self) -> CliResult<()> {
        self.print_welcome();

        loop {
            let prompt = self.get_prompt();

            match self.editor.readline(&prompt) {
                Ok(line) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    let _ = self.editor.add_history_entry(line);

                    if self.handle_builtin(line).await? {
                        continue;
                    }

                    match self.client.query(line).await {
                        Ok(result) => {
                            self.formatter.print(&result);
                        }
                        Err(e) => {
                            eprintln!("{} {}", "Error:".red().bold(), e);
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("^C");
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    println!("Bye!");
                    break;
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    break;
                }
            }
        }

        let _ = self.editor.save_history(&self.history_file);
        Ok(())
    }

    fn print_welcome(&self) {
        println!(
            r#"
  __  __ _ _          ____  ____
 |  \/  (_) | ___   _|  _ \| __ )
 | |\/| | | |/ / | | | | | |  _ \
 | |  | | |   <| |_| | |_| | |_) |
 |_|  |_|_|_|\_\\__,_|____/|____/

 MikuDB CLI v{}
 Type 'help' for commands, 'exit' to quit.
"#,
            env!("CARGO_PKG_VERSION")
        );
    }

    fn get_prompt(&self) -> String {
        match &self.current_database {
            Some(db) => format!("mikudb:{}> ", db.cyan()),
            None => "mikudb> ".to_string(),
        }
    }

    async fn handle_builtin(&mut self, line: &str) -> CliResult<bool> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(true);
        }

        match parts[0].to_lowercase().as_str() {
            "exit" | "quit" | "\\q" => {
                println!("Bye!");
                std::process::exit(0);
            }
            "help" | "\\h" | "?" => {
                self.print_help();
                Ok(true)
            }
            "clear" | "\\c" => {
                print!("\x1B[2J\x1B[1;1H");
                Ok(true)
            }
            "use" => {
                if parts.len() > 1 {
                    self.current_database = Some(parts[1].to_string());
                    println!("Switched to database {}", parts[1].cyan());
                } else {
                    println!("Usage: use <database>");
                }
                Ok(true)
            }
            "status" | "\\s" => {
                self.print_status().await;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn print_help(&self) {
        println!(
            r#"
{}
  FIND <collection> [WHERE <condition>]   - Query documents
  INSERT INTO <collection> {{...}}          - Insert document
  UPDATE <collection> SET ... WHERE ...   - Update documents
  DELETE FROM <collection> WHERE ...      - Delete documents

{}
  SHOW DATABASE                           - List databases
  SHOW COLLECTION                         - List collections
  CREATE COLLECTION <name>                - Create collection
  DROP COLLECTION <name>                  - Drop collection
  CREATE INDEX <name> ON <col> (fields)   - Create index

{}
  USE <database>                          - Switch database
  help, \h, ?                             - Show this help
  clear, \c                               - Clear screen
  status, \s                              - Show connection status
  exit, quit, \q                          - Exit CLI

{}
  Ctrl+C                                  - Cancel current input
  Ctrl+D                                  - Exit CLI
  Tab                                     - Auto-complete
  Up/Down                                 - History navigation
"#,
            "Query Commands:".green().bold(),
            "Database Commands:".green().bold(),
            "Built-in Commands:".green().bold(),
            "Keyboard Shortcuts:".green().bold()
        );
    }

    async fn print_status(&self) {
        println!("{}", "Connection Status:".green().bold());
        println!("  Host: {}:{}", self.client.host(), self.client.port());
        println!("  User: {}", self.client.user());
        println!(
            "  Database: {}",
            self.current_database
                .as_deref()
                .unwrap_or("(none)")
        );
        println!("  Connected: {}", "Yes".green());
    }
}
