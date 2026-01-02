//! 帮助信息模块
//!
//! 提供完善的命令帮助系统,支持:
//! - 通用帮助信息(help 命令)
//! - 命令详细帮助(命令 ? 语法)
//! - 多语言支持

use crate::i18n::{current_language, Language};
use colored::Colorize;

/// 打印主帮助信息
pub fn print_main_help() {
    match current_language() {
        Language::English => print_main_help_en(),
        Language::Chinese => print_main_help_zh(),
    }
}

/// 打印主帮助信息(英文)
fn print_main_help_en() {
    println!();
    println!("{}", "MikuDB Interactive Shell - Help".green().bold());
    println!();
    println!("{}", "GETTING STARTED".cyan().bold());
    println!("  Use 'command ?' to get detailed help for any command");
    println!("  Example: FIND ?, INSERT ?, AGGREGATE ?, LANG ?");
    println!();

    println!("{}", "CRUD OPERATIONS".cyan().bold());
    println!("  {}          - Query documents from collection", "FIND".yellow());
    println!("  {}        - Insert document into collection", "INSERT".yellow());
    println!("  {}        - Update documents in collection", "UPDATE".yellow());
    println!("  {}        - Delete documents from collection", "DELETE".yellow());
    println!("  {}     - Aggregation pipeline operations", "AGGREGATE".yellow());
    println!();

    println!("{}", "DATABASE & COLLECTION MANAGEMENT".cyan().bold());
    println!("  {} - Show databases/collections/indexes/users/status", "SHOW".yellow());
    println!("  {}       - Create collection/database/index/user", "CREATE".yellow());
    println!("  {}         - Drop collection/database/index/user", "DROP".yellow());
    println!();

    println!("{}", "TRANSACTION COMMANDS".cyan().bold());
    println!("  {}         - Start a new transaction", "BEGIN".yellow());
    println!("  {}        - Commit current transaction", "COMMIT".yellow());
    println!("  {}      - Rollback current transaction", "ROLLBACK".yellow());
    println!();

    println!("{}", "USER & PERMISSION MANAGEMENT".cyan().bold());
    println!("  {}   - Create database user", "CREATE USER".yellow());
    println!("  {}     - Delete database user", "DROP USER".yellow());
    println!("  {}         - Grant privileges to user", "GRANT".yellow());
    println!("  {}        - Revoke privileges from user", "REVOKE".yellow());
    println!("  {}    - List all users", "SHOW USERS".yellow());
    println!();

    println!("{}", "BUILT-IN COMMANDS".cyan().bold());
    println!("  {}            - Switch database", "USE <db>".yellow());
    println!("  {}        - Change language (en/zh)", "LANG <lang>".yellow());
    println!("  {}         - Show connection status", "STATUS".yellow());
    println!("  {}           - Show this help", "HELP".yellow());
    println!("  {}          - Clear screen", "CLEAR".yellow());
    println!("  {}           - Exit CLI", "EXIT".yellow());
    println!();

    println!("{}", "KEYBOARD SHORTCUTS".cyan().bold());
    println!("  Tab         - Auto-complete");
    println!("  Ctrl+C      - Cancel current input");
    println!("  Ctrl+D      - Exit CLI");
    println!("  Up/Down     - Navigate history");
    println!();

    println!("{}", "EXAMPLES".cyan().bold());
    println!("  FIND users WHERE age > 18");
    println!("  INSERT INTO users {{name: \"Miku\", age: 16}}");
    println!("  AGGREGATE sales [{{$group: {{_id: \"$product\", total: {{$sum: \"$amount\"}}}}}}]");
    println!("  CREATE INDEX idx_name ON users (name)");
    println!("  BEGIN TRANSACTION");
    println!();
}

/// 打印主帮助信息(中文)
fn print_main_help_zh() {
    println!();
    println!("{}", "MikuDB 交互式命令行 - 帮助".green().bold());
    println!();
    println!("{}", "快速开始".cyan().bold());
    println!("  使用 '命令 ?' 查看任何命令的详细帮助");
    println!("  示例: FIND ?, INSERT ?, AGGREGATE ?, LANG ?");
    println!();

    println!("{}", "CRUD 操作".cyan().bold());
    println!("  {}          - 从集合查询文档", "FIND".yellow());
    println!("  {}        - 向集合插入文档", "INSERT".yellow());
    println!("  {}        - 更新集合中的文档", "UPDATE".yellow());
    println!("  {}        - 从集合删除文档", "DELETE".yellow());
    println!("  {}     - 聚合管道操作", "AGGREGATE".yellow());
    println!();

    println!("{}", "数据库和集合管理".cyan().bold());
    println!("  {}   - 显示数据库/集合/索引/用户/状态", "SHOW".yellow());
    println!("  {}       - 创建集合/数据库/索引/用户", "CREATE".yellow());
    println!("  {}         - 删除集合/数据库/索引/用户", "DROP".yellow());
    println!();

    println!("{}", "事务命令".cyan().bold());
    println!("  {}         - 开始新事务", "BEGIN".yellow());
    println!("  {}        - 提交当前事务", "COMMIT".yellow());
    println!("  {}      - 回滚当前事务", "ROLLBACK".yellow());
    println!();

    println!("{}", "用户和权限管理".cyan().bold());
    println!("  {}   - 创建数据库用户", "CREATE USER".yellow());
    println!("  {}     - 删除数据库用户", "DROP USER".yellow());
    println!("  {}         - 授予用户权限", "GRANT".yellow());
    println!("  {}        - 撤销用户权限", "REVOKE".yellow());
    println!("  {}    - 列出所有用户", "SHOW USERS".yellow());
    println!();

    println!("{}", "内置命令".cyan().bold());
    println!("  {}         - 切换数据库", "USE <数据库>".yellow());
    println!("  {}     - 切换语言 (en/zh)", "LANG <语言>".yellow());
    println!("  {}         - 显示连接状态", "STATUS".yellow());
    println!("  {}           - 显示此帮助", "HELP".yellow());
    println!("  {}          - 清空屏幕", "CLEAR".yellow());
    println!("  {}           - 退出命令行", "EXIT".yellow());
    println!();

    println!("{}", "快捷键".cyan().bold());
    println!("  Tab         - 自动补全");
    println!("  Ctrl+C      - 取消当前输入");
    println!("  Ctrl+D      - 退出命令行");
    println!("  上/下方向键  - 浏览历史记录");
    println!();

    println!("{}", "示例".cyan().bold());
    println!("  FIND users WHERE age > 18");
    println!("  INSERT INTO users {{name: \"初音未来\", age: 16}}");
    println!("  AGGREGATE sales [{{$group: {{_id: \"$product\", total: {{$sum: \"$amount\"}}}}}}]");
    println!("  CREATE INDEX idx_name ON users (name)");
    println!("  BEGIN TRANSACTION");
    println!();
}

/// 获取命令详细帮助
pub fn get_command_help(cmd: &str) -> Option<String> {
    let cmd_upper = cmd.trim().to_uppercase();

    match current_language() {
        Language::English => get_command_help_en(&cmd_upper),
        Language::Chinese => get_command_help_zh(&cmd_upper),
    }
}

/// 获取命令详细帮助(英文)
fn get_command_help_en(cmd: &str) -> Option<String> {
    let help = match cmd {
        "FIND" => {
            format!(
                "\n{}\n\n{}\n  FIND <collection> [WHERE <condition>] [ORDER BY <field>] [LIMIT <n>]\n\n{}\n  Query documents from a collection with optional filtering and sorting.\n\n{}\n  - collection: Name of the collection to query\n  - WHERE: Optional filter condition (supports =, !=, >, <, >=, <=, AND, OR)\n  - ORDER BY: Optional sorting (ASC or DESC)\n  - LIMIT: Limit number of results\n\n{}\n  FIND users\n  FIND users WHERE age > 18\n  FIND users WHERE age > 18 AND city = \"Beijing\"\n  FIND users WHERE age > 18 ORDER BY name ASC LIMIT 10\n",
                "FIND - Query Documents".green().bold(),
                "SYNTAX".cyan().bold(),
                "DESCRIPTION".cyan().bold(),
                "PARAMETERS".cyan().bold(),
                "EXAMPLES".cyan().bold()
            )
        }
        "INSERT" | "INSERT INTO" => {
            format!(
                "\n{}\n\n{}\n  INSERT INTO <collection> {{field1: value1, field2: value2, ...}}\n\n{}\n  Insert a new document into a collection.\n  An _id field will be automatically generated if not provided.\n\n{}\n  - collection: Name of the collection\n  - {{...}}: Document to insert (BOML format)\n\n{}\n  INSERT INTO users {{name: \"Miku\", age: 16, city: \"Tokyo\"}}\n  INSERT INTO products {{name: \"Laptop\", price: 999.99, stock: 50}}\n  INSERT INTO users {{_id: \"custom_id\", name: \"Test\"}}\n",
                "INSERT - Insert Document".green().bold(),
                "SYNTAX".cyan().bold(),
                "DESCRIPTION".cyan().bold(),
                "PARAMETERS".cyan().bold(),
                "EXAMPLES".cyan().bold()
            )
        }
        "UPDATE" => {
            format!(
                "\n{}\n\n{}\n  UPDATE <collection> SET <field> = <value> [, ...] WHERE <condition>\n\n{}\n  Update existing documents in a collection.\n\n{}\n  - collection: Name of the collection\n  - SET: Fields to update with new values\n  - WHERE: Condition to match documents\n\n{}\n  UPDATE users SET age = 17 WHERE name = \"Miku\"\n  UPDATE products SET price = 899.99, stock = 45 WHERE name = \"Laptop\"\n  UPDATE users SET status = \"active\" WHERE age >= 18\n",
                "UPDATE - Update Documents".green().bold(),
                "SYNTAX".cyan().bold(),
                "DESCRIPTION".cyan().bold(),
                "PARAMETERS".cyan().bold(),
                "EXAMPLES".cyan().bold()
            )
        }
        "DELETE" | "DELETE FROM" => {
            format!(
                "\n{}\n\n{}\n  DELETE FROM <collection> WHERE <condition>\n\n{}\n  Delete documents from a collection.\n\n{}\n  - collection: Name of the collection\n  - WHERE: Condition to match documents to delete\n\n{}\n  DELETE FROM users WHERE age < 13\n  DELETE FROM products WHERE stock = 0\n  DELETE FROM logs WHERE timestamp < \"2024-01-01\"\n",
                "DELETE - Delete Documents".green().bold(),
                "SYNTAX".cyan().bold(),
                "DESCRIPTION".cyan().bold(),
                "PARAMETERS".cyan().bold(),
                "EXAMPLES".cyan().bold()
            )
        }
        "SHOW" => {
            format!(
                "\n{}\n\n{}\n  SHOW DATABASE\n  SHOW COLLECTION\n  SHOW STATUS\n\n{}\n  Display information about databases, collections, or server status.\n\n{}\n  SHOW DATABASE     - List all databases\n  SHOW COLLECTION   - List all collections in current database\n  SHOW STATUS       - Show server status and statistics\n",
                "SHOW - Show Information".green().bold(),
                "SYNTAX".cyan().bold(),
                "DESCRIPTION".cyan().bold(),
                "EXAMPLES".cyan().bold()
            )
        }
        "CREATE" => {
            format!(
                "\n{}\n\n{}\n  CREATE COLLECTION <name>\n  CREATE DATABASE <name>\n  CREATE INDEX <name> ON <collection> (field1, field2, ...)\n\n{}\n  Create a new collection, database, or index.\n\n{}\n  CREATE COLLECTION users\n  CREATE DATABASE myapp\n  CREATE INDEX idx_name ON users (name)\n  CREATE UNIQUE INDEX idx_email ON users (email)\n",
                "CREATE - Create Object".green().bold(),
                "SYNTAX".cyan().bold(),
                "DESCRIPTION".cyan().bold(),
                "EXAMPLES".cyan().bold()
            )
        }
        "DROP" => {
            format!(
                "\n{}\n\n{}\n  DROP COLLECTION <name>\n  DROP DATABASE <name>\n  DROP INDEX <name> ON <collection>\n\n{}\n  Delete a collection, database, or index.\n  {} This operation cannot be undone!\n\n{}\n  DROP COLLECTION old_users\n  DROP DATABASE test_db\n  DROP INDEX idx_name ON users\n",
                "DROP - Drop Object".green().bold(),
                "SYNTAX".cyan().bold(),
                "DESCRIPTION".cyan().bold(),
                "WARNING:".red().bold(),
                "EXAMPLES".cyan().bold()
            )
        }
        "USE" => {
            format!(
                "\n{}\n\n{}\n  USE <database>\n\n{}\n  Switch to a different database.\n  All subsequent commands will operate on this database.\n\n{}\n  USE myapp\n  USE test\n",
                "USE - Switch Database".green().bold(),
                "SYNTAX".cyan().bold(),
                "DESCRIPTION".cyan().bold(),
                "EXAMPLES".cyan().bold()
            )
        }
        "LANG" | "LANGUAGE" => {
            format!(
                "\n{}\n\n{}\n  LANG <language>\n  LANG\n\n{}\n  Change the CLI display language or show current language.\n  Supported languages: en (English), zh (Chinese)\n  Language preference is saved to ~/.mikudb_config\n\n{}\n  - language: en, zh, english, chinese\n\n{}\n  LANG zh        - Switch to Chinese\n  LANG en        - Switch to English\n  LANG           - Show current language\n",
                "LANG - Change Language".green().bold(),
                "SYNTAX".cyan().bold(),
                "DESCRIPTION".cyan().bold(),
                "PARAMETERS".cyan().bold(),
                "EXAMPLES".cyan().bold()
            )
        }
        "STATUS" => {
            format!(
                "\n{}\n\n{}\n  STATUS\n\n{}\n  Display current connection information including:\n  - Server host and port\n  - Connected user\n  - Current database\n  - Connection status\n",
                "STATUS - Connection Status".green().bold(),
                "SYNTAX".cyan().bold(),
                "DESCRIPTION".cyan().bold()
            )
        }
        "AGGREGATE" => {
            format!(
                "\n{}\n\n{}\n  AGGREGATE <collection> [<pipeline>]\n\n{}\n  Perform aggregation operations on documents using a pipeline of stages.\n  Supports: $match, $group, $sort, $project, $limit, $skip, $lookup, $unwind\n\n{}\n  - collection: Name of the collection\n  - pipeline: Array of aggregation stages\n\n{}\n  AGGREGATE users [{{$match: {{age: {{$gt: 18}}}}}}\n  AGGREGATE sales [{{$group: {{_id: \"$product\", total: {{$sum: \"$amount\"}}}}}}\n  AGGREGATE orders [{{$lookup: {{from: \"products\", localField: \"productId\", foreignField: \"_id\", as: \"product\"}}}}]\n",
                "AGGREGATE - Aggregation Pipeline".green().bold(),
                "SYNTAX".cyan().bold(),
                "DESCRIPTION".cyan().bold(),
                "PARAMETERS".cyan().bold(),
                "EXAMPLES".cyan().bold()
            )
        }
        "BEGIN" | "BEGIN TRANSACTION" => {
            format!(
                "\n{}\n\n{}\n  BEGIN TRANSACTION\n  BEGIN\n\n{}\n  Start a new transaction. All subsequent operations will be part of this transaction\n  until COMMIT or ROLLBACK is executed.\n\n{}\n  BEGIN TRANSACTION\n  INSERT INTO users {{name: \"Test\"}}\n  UPDATE users SET status = \"active\" WHERE name = \"Test\"\n  COMMIT\n",
                "BEGIN - Start Transaction".green().bold(),
                "SYNTAX".cyan().bold(),
                "DESCRIPTION".cyan().bold(),
                "EXAMPLES".cyan().bold()
            )
        }
        "COMMIT" => {
            format!(
                "\n{}\n\n{}\n  COMMIT\n\n{}\n  Commit the current transaction, making all changes permanent.\n\n{}\n  BEGIN TRANSACTION\n  INSERT INTO users {{name: \"Test\"}}\n  COMMIT\n",
                "COMMIT - Commit Transaction".green().bold(),
                "SYNTAX".cyan().bold(),
                "DESCRIPTION".cyan().bold(),
                "EXAMPLES".cyan().bold()
            )
        }
        "ROLLBACK" => {
            format!(
                "\n{}\n\n{}\n  ROLLBACK\n\n{}\n  Rollback the current transaction, discarding all changes made in the transaction.\n\n{}\n  BEGIN TRANSACTION\n  DELETE FROM users WHERE age < 13\n  ROLLBACK  -- Undo the deletion\n",
                "ROLLBACK - Rollback Transaction".green().bold(),
                "SYNTAX".cyan().bold(),
                "DESCRIPTION".cyan().bold(),
                "EXAMPLES".cyan().bold()
            )
        }
        "CREATE USER" => {
            format!(
                "\n{}\n\n{}\n  CREATE USER <username> PASSWORD <password> [ROLE <role>]\n\n{}\n  Create a new database user with specified password and optional role.\n\n{}\n  - username: Username for the new user\n  - password: Password for the user\n  - role: Optional role (admin, readWrite, read)\n\n{}\n  CREATE USER john PASSWORD \"secret123\"\n  CREATE USER admin PASSWORD \"admin123\" ROLE admin\n  CREATE USER reader PASSWORD \"read123\" ROLE read\n",
                "CREATE USER - Create Database User".green().bold(),
                "SYNTAX".cyan().bold(),
                "DESCRIPTION".cyan().bold(),
                "PARAMETERS".cyan().bold(),
                "EXAMPLES".cyan().bold()
            )
        }
        "DROP USER" => {
            format!(
                "\n{}\n\n{}\n  DROP USER <username>\n\n{}\n  Delete a database user.\n  {} This operation cannot be undone!\n\n{}\n  DROP USER john\n  DROP USER old_admin\n",
                "DROP USER - Delete User".green().bold(),
                "SYNTAX".cyan().bold(),
                "DESCRIPTION".cyan().bold(),
                "WARNING:".red().bold(),
                "EXAMPLES".cyan().bold()
            )
        }
        "GRANT" => {
            format!(
                "\n{}\n\n{}\n  GRANT <privilege> ON <database>.<collection> TO <user>\n\n{}\n  Grant privileges to a user for a specific database or collection.\n\n{}\n  - privilege: read, write, admin\n  - database: Database name\n  - collection: Collection name (use * for all)\n  - user: Username\n\n{}\n  GRANT read ON mydb.* TO john\n  GRANT write ON mydb.users TO john\n  GRANT admin ON *.* TO admin_user\n",
                "GRANT - Grant Privileges".green().bold(),
                "SYNTAX".cyan().bold(),
                "DESCRIPTION".cyan().bold(),
                "PARAMETERS".cyan().bold(),
                "EXAMPLES".cyan().bold()
            )
        }
        "REVOKE" => {
            format!(
                "\n{}\n\n{}\n  REVOKE <privilege> ON <database>.<collection> FROM <user>\n\n{}\n  Revoke privileges from a user.\n\n{}\n  - privilege: read, write, admin\n  - database: Database name\n  - collection: Collection name (use * for all)\n  - user: Username\n\n{}\n  REVOKE write ON mydb.users FROM john\n  REVOKE admin ON *.* FROM old_admin\n",
                "REVOKE - Revoke Privileges".green().bold(),
                "SYNTAX".cyan().bold(),
                "DESCRIPTION".cyan().bold(),
                "PARAMETERS".cyan().bold(),
                "EXAMPLES".cyan().bold()
            )
        }
        "SHOW USERS" => {
            format!(
                "\n{}\n\n{}\n  SHOW USERS\n\n{}\n  List all database users and their roles.\n",
                "SHOW USERS - List Users".green().bold(),
                "SYNTAX".cyan().bold(),
                "DESCRIPTION".cyan().bold()
            )
        }
        "SHOW INDEXES" => {
            format!(
                "\n{}\n\n{}\n  SHOW INDEXES ON <collection>\n\n{}\n  Display all indexes on a specific collection.\n\n{}\n  SHOW INDEXES ON users\n  SHOW INDEXES ON products\n",
                "SHOW INDEXES - List Indexes".green().bold(),
                "SYNTAX".cyan().bold(),
                "DESCRIPTION".cyan().bold(),
                "EXAMPLES".cyan().bold()
            )
        }
        "HELP" => {
            format!(
                "\n{}\n\n{}\n  HELP\n  <command> ?\n\n{}\n  Display help information.\n  Use 'HELP' for general help, or 'command ?' for specific command help.\n\n{}\n  HELP\n  FIND ?\n  INSERT ?\n  LANG ?\n",
                "HELP - Show Help".green().bold(),
                "SYNTAX".cyan().bold(),
                "DESCRIPTION".cyan().bold(),
                "EXAMPLES".cyan().bold()
            )
        }
        _ => return None,
    };

    Some(help)
}

/// 获取命令详细帮助(中文)
fn get_command_help_zh(cmd: &str) -> Option<String> {
    let help = match cmd {
        "FIND" => {
            format!(
                "\n{}\n\n{}\n  FIND <集合名> [WHERE <条件>] [ORDER BY <字段>] [LIMIT <数量>]\n\n{}\n  从集合中查询文档,支持可选的过滤和排序。\n\n{}\n  - 集合名: 要查询的集合名称\n  - WHERE: 可选的过滤条件 (支持 =, !=, >, <, >=, <=, AND, OR)\n  - ORDER BY: 可选的排序 (ASC 升序或 DESC 降序)\n  - LIMIT: 限制结果数量\n\n{}\n  FIND users\n  FIND users WHERE age > 18\n  FIND users WHERE age > 18 AND city = \"北京\"\n  FIND users WHERE age > 18 ORDER BY name ASC LIMIT 10\n",
                "FIND - 查询文档".green().bold(),
                "语法".cyan().bold(),
                "描述".cyan().bold(),
                "参数".cyan().bold(),
                "示例".cyan().bold()
            )
        }
        "INSERT" | "INSERT INTO" => {
            format!(
                "\n{}\n\n{}\n  INSERT INTO <集合名> {{字段1: 值1, 字段2: 值2, ...}}\n\n{}\n  向集合中插入新文档。\n  如果未提供 _id 字段,系统会自动生成。\n\n{}\n  - 集合名: 集合的名称\n  - {{...}}: 要插入的文档 (BOML 格式)\n\n{}\n  INSERT INTO users {{name: \"初音未来\", age: 16, city: \"东京\"}}\n  INSERT INTO products {{name: \"笔记本电脑\", price: 999.99, stock: 50}}\n  INSERT INTO users {{_id: \"custom_id\", name: \"测试\"}}\n",
                "INSERT - 插入文档".green().bold(),
                "语法".cyan().bold(),
                "描述".cyan().bold(),
                "参数".cyan().bold(),
                "示例".cyan().bold()
            )
        }
        "UPDATE" => {
            format!(
                "\n{}\n\n{}\n  UPDATE <集合名> SET <字段> = <值> [, ...] WHERE <条件>\n\n{}\n  更新集合中的现有文档。\n\n{}\n  - 集合名: 集合的名称\n  - SET: 要更新的字段及新值\n  - WHERE: 匹配文档的条件\n\n{}\n  UPDATE users SET age = 17 WHERE name = \"初音未来\"\n  UPDATE products SET price = 899.99, stock = 45 WHERE name = \"笔记本电脑\"\n  UPDATE users SET status = \"active\" WHERE age >= 18\n",
                "UPDATE - 更新文档".green().bold(),
                "语法".cyan().bold(),
                "描述".cyan().bold(),
                "参数".cyan().bold(),
                "示例".cyan().bold()
            )
        }
        "DELETE" | "DELETE FROM" => {
            format!(
                "\n{}\n\n{}\n  DELETE FROM <集合名> WHERE <条件>\n\n{}\n  从集合中删除文档。\n\n{}\n  - 集合名: 集合的名称\n  - WHERE: 匹配要删除文档的条件\n\n{}\n  DELETE FROM users WHERE age < 13\n  DELETE FROM products WHERE stock = 0\n  DELETE FROM logs WHERE timestamp < \"2024-01-01\"\n",
                "DELETE - 删除文档".green().bold(),
                "语法".cyan().bold(),
                "描述".cyan().bold(),
                "参数".cyan().bold(),
                "示例".cyan().bold()
            )
        }
        "SHOW" => {
            format!(
                "\n{}\n\n{}\n  SHOW DATABASE\n  SHOW COLLECTION\n  SHOW STATUS\n\n{}\n  显示数据库、集合或服务器状态信息。\n\n{}\n  SHOW DATABASE     - 列出所有数据库\n  SHOW COLLECTION   - 列出当前数据库的所有集合\n  SHOW STATUS       - 显示服务器状态和统计信息\n",
                "SHOW - 显示信息".green().bold(),
                "语法".cyan().bold(),
                "描述".cyan().bold(),
                "示例".cyan().bold()
            )
        }
        "CREATE" => {
            format!(
                "\n{}\n\n{}\n  CREATE COLLECTION <名称>\n  CREATE DATABASE <名称>\n  CREATE INDEX <索引名> ON <集合> (字段1, 字段2, ...)\n\n{}\n  创建新的集合、数据库或索引。\n\n{}\n  CREATE COLLECTION users\n  CREATE DATABASE myapp\n  CREATE INDEX idx_name ON users (name)\n  CREATE UNIQUE INDEX idx_email ON users (email)\n",
                "CREATE - 创建对象".green().bold(),
                "语法".cyan().bold(),
                "描述".cyan().bold(),
                "示例".cyan().bold()
            )
        }
        "DROP" => {
            format!(
                "\n{}\n\n{}\n  DROP COLLECTION <名称>\n  DROP DATABASE <名称>\n  DROP INDEX <索引名> ON <集合>\n\n{}\n  删除集合、数据库或索引。\n  {} 此操作无法撤销!\n\n{}\n  DROP COLLECTION old_users\n  DROP DATABASE test_db\n  DROP INDEX idx_name ON users\n",
                "DROP - 删除对象".green().bold(),
                "语法".cyan().bold(),
                "描述".cyan().bold(),
                "警告:".red().bold(),
                "示例".cyan().bold()
            )
        }
        "USE" => {
            format!(
                "\n{}\n\n{}\n  USE <数据库名>\n\n{}\n  切换到不同的数据库。\n  后续所有命令将在该数据库上操作。\n\n{}\n  USE myapp\n  USE test\n",
                "USE - 切换数据库".green().bold(),
                "语法".cyan().bold(),
                "描述".cyan().bold(),
                "示例".cyan().bold()
            )
        }
        "LANG" | "LANGUAGE" => {
            format!(
                "\n{}\n\n{}\n  LANG <语言>\n  LANG\n\n{}\n  更改命令行显示语言或显示当前语言。\n  支持的语言: en (英文), zh (中文)\n  语言偏好设置保存在 ~/.mikudb_config\n\n{}\n  - 语言: en, zh, english, chinese\n\n{}\n  LANG zh        - 切换到中文\n  LANG en        - 切换到英文\n  LANG           - 显示当前语言\n",
                "LANG - 切换语言".green().bold(),
                "语法".cyan().bold(),
                "描述".cyan().bold(),
                "参数".cyan().bold(),
                "示例".cyan().bold()
            )
        }
        "STATUS" => {
            format!(
                "\n{}\n\n{}\n  STATUS\n\n{}\n  显示当前连接信息,包括:\n  - 服务器主机和端口\n  - 已连接的用户\n  - 当前数据库\n  - 连接状态\n",
                "STATUS - 连接状态".green().bold(),
                "语法".cyan().bold(),
                "描述".cyan().bold()
            )
        }
        "AGGREGATE" => {
            format!(
                "\n{}\n\n{}\n  AGGREGATE <集合名> [<管道>]\n\n{}\n  使用管道阶段对文档执行聚合操作。\n  支持: $match, $group, $sort, $project, $limit, $skip, $lookup, $unwind\n\n{}\n  - 集合名: 集合的名称\n  - 管道: 聚合阶段数组\n\n{}\n  AGGREGATE users [{{$match: {{age: {{$gt: 18}}}}}}\n  AGGREGATE sales [{{$group: {{_id: \"$product\", total: {{$sum: \"$amount\"}}}}}}\n  AGGREGATE orders [{{$lookup: {{from: \"products\", localField: \"productId\", foreignField: \"_id\", as: \"product\"}}}}]\n",
                "AGGREGATE - 聚合管道".green().bold(),
                "语法".cyan().bold(),
                "描述".cyan().bold(),
                "参数".cyan().bold(),
                "示例".cyan().bold()
            )
        }
        "BEGIN" | "BEGIN TRANSACTION" => {
            format!(
                "\n{}\n\n{}\n  BEGIN TRANSACTION\n  BEGIN\n\n{}\n  开始一个新事务。所有后续操作将成为此事务的一部分,\n  直到执行 COMMIT 或 ROLLBACK。\n\n{}\n  BEGIN TRANSACTION\n  INSERT INTO users {{name: \"测试\"}}\n  UPDATE users SET status = \"active\" WHERE name = \"测试\"\n  COMMIT\n",
                "BEGIN - 开始事务".green().bold(),
                "语法".cyan().bold(),
                "描述".cyan().bold(),
                "示例".cyan().bold()
            )
        }
        "COMMIT" => {
            format!(
                "\n{}\n\n{}\n  COMMIT\n\n{}\n  提交当前事务,使所有更改永久生效。\n\n{}\n  BEGIN TRANSACTION\n  INSERT INTO users {{name: \"测试\"}}\n  COMMIT\n",
                "COMMIT - 提交事务".green().bold(),
                "语法".cyan().bold(),
                "描述".cyan().bold(),
                "示例".cyan().bold()
            )
        }
        "ROLLBACK" => {
            format!(
                "\n{}\n\n{}\n  ROLLBACK\n\n{}\n  回滚当前事务,放弃事务中所做的所有更改。\n\n{}\n  BEGIN TRANSACTION\n  DELETE FROM users WHERE age < 13\n  ROLLBACK  -- 撤销删除\n",
                "ROLLBACK - 回滚事务".green().bold(),
                "语法".cyan().bold(),
                "描述".cyan().bold(),
                "示例".cyan().bold()
            )
        }
        "CREATE USER" => {
            format!(
                "\n{}\n\n{}\n  CREATE USER <用户名> PASSWORD <密码> [ROLE <角色>]\n\n{}\n  创建新的数据库用户,指定密码和可选角色。\n\n{}\n  - 用户名: 新用户的用户名\n  - 密码: 用户密码\n  - 角色: 可选角色 (admin, readWrite, read)\n\n{}\n  CREATE USER john PASSWORD \"secret123\"\n  CREATE USER admin PASSWORD \"admin123\" ROLE admin\n  CREATE USER reader PASSWORD \"read123\" ROLE read\n",
                "CREATE USER - 创建数据库用户".green().bold(),
                "语法".cyan().bold(),
                "描述".cyan().bold(),
                "参数".cyan().bold(),
                "示例".cyan().bold()
            )
        }
        "DROP USER" => {
            format!(
                "\n{}\n\n{}\n  DROP USER <用户名>\n\n{}\n  删除数据库用户。\n  {} 此操作无法撤销!\n\n{}\n  DROP USER john\n  DROP USER old_admin\n",
                "DROP USER - 删除用户".green().bold(),
                "语法".cyan().bold(),
                "描述".cyan().bold(),
                "警告:".red().bold(),
                "示例".cyan().bold()
            )
        }
        "GRANT" => {
            format!(
                "\n{}\n\n{}\n  GRANT <权限> ON <数据库>.<集合> TO <用户>\n\n{}\n  为用户授予特定数据库或集合的权限。\n\n{}\n  - 权限: read, write, admin\n  - 数据库: 数据库名称\n  - 集合: 集合名称 (使用 * 表示所有)\n  - 用户: 用户名\n\n{}\n  GRANT read ON mydb.* TO john\n  GRANT write ON mydb.users TO john\n  GRANT admin ON *.* TO admin_user\n",
                "GRANT - 授予权限".green().bold(),
                "语法".cyan().bold(),
                "描述".cyan().bold(),
                "参数".cyan().bold(),
                "示例".cyan().bold()
            )
        }
        "REVOKE" => {
            format!(
                "\n{}\n\n{}\n  REVOKE <权限> ON <数据库>.<集合> FROM <用户>\n\n{}\n  撤销用户的权限。\n\n{}\n  - 权限: read, write, admin\n  - 数据库: 数据库名称\n  - 集合: 集合名称 (使用 * 表示所有)\n  - 用户: 用户名\n\n{}\n  REVOKE write ON mydb.users FROM john\n  REVOKE admin ON *.* FROM old_admin\n",
                "REVOKE - 撤销权限".green().bold(),
                "语法".cyan().bold(),
                "描述".cyan().bold(),
                "参数".cyan().bold(),
                "示例".cyan().bold()
            )
        }
        "SHOW USERS" => {
            format!(
                "\n{}\n\n{}\n  SHOW USERS\n\n{}\n  列出所有数据库用户及其角色。\n",
                "SHOW USERS - 列出用户".green().bold(),
                "语法".cyan().bold(),
                "描述".cyan().bold()
            )
        }
        "SHOW INDEXES" => {
            format!(
                "\n{}\n\n{}\n  SHOW INDEXES ON <集合名>\n\n{}\n  显示指定集合上的所有索引。\n\n{}\n  SHOW INDEXES ON users\n  SHOW INDEXES ON products\n",
                "SHOW INDEXES - 列出索引".green().bold(),
                "语法".cyan().bold(),
                "描述".cyan().bold(),
                "示例".cyan().bold()
            )
        }
        "HELP" => {
            format!(
                "\n{}\n\n{}\n  HELP\n  <命令> ?\n\n{}\n  显示帮助信息。\n  使用 'HELP' 查看通用帮助,或使用 '命令 ?' 查看特定命令帮助。\n\n{}\n  HELP\n  FIND ?\n  INSERT ?\n  LANG ?\n",
                "HELP - 显示帮助".green().bold(),
                "语法".cyan().bold(),
                "描述".cyan().bold(),
                "示例".cyan().bold()
            )
        }
        _ => return None,
    };

    Some(help)
}
