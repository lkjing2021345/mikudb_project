# MQL (Miku Query Language) 完全指南

## 目录

- [基础概念](#基础概念)
- [数据库操作](#数据库操作)
- [集合操作](#集合操作)
- [文档CRUD操作](#文档crud操作)
- [查询语法](#查询语法)
- [索引管理](#索引管理)
- [聚合查询](#聚合查询)
- [事务管理](#事务管理)
- [用户权限管理](#用户权限管理)
- [系统命令](#系统命令)
- [数据类型](#数据类型)

---

## 基础概念

MQL (Miku Query Language) 是 MikuDB 的查询语言，采用类似 SQL 的语法，但针对文档数据库进行了优化。

### 核心术语
- **Database（数据库）**：存储集合的容器
- **Collection（集合）**：存储文档的容器，类似关系数据库中的表
- **Document（文档）**：JSON格式的数据记录，类似关系数据库中的行
- **Field（字段）**：文档中的键值对，类似关系数据库中的列

### 连接数据库

```bash
# 使用 mikudb-cli 连接
mikudb-cli --host localhost --port 3939 --user miku --password mikumiku3939

# 简写形式（使用默认值）
mikudb-cli
```

---

## 数据库操作

### 1. 切换数据库

```sql
USE database_name
```

**示例：**
```sql
USE myapp
```

**说明：**
- 切换到指定数据库
- 如果数据库不存在，会在第一次创建集合时自动创建
- 默认数据库为 `default`

---

### 2. 查看所有数据库

```sql
SHOW DATABASE
```

**示例输出：**
```
┌──────────┐
│ name     │
├──────────┤
│ default  │
│ myapp    │
│ testdb   │
└──────────┘
```

**说明：**
- 列出当前服务器上的所有数据库
- 注意：使用单数形式 `DATABASE`，而非 `DATABASES`

---

### 3. 创建数据库

```sql
CREATE DATABASE database_name
```

**示例：**
```sql
CREATE DATABASE production
```

**说明：**
- 显式创建数据库（可选操作）
- 通常不需要手动创建，使用 `USE` 切换后自动创建

---

### 4. 删除数据库

```sql
DROP DATABASE database_name
```

**示例：**
```sql
DROP DATABASE testdb
```

**警告：**
- 此操作会删除数据库及其所有集合和文档
- 操作不可逆，请谨慎使用

---

## 集合操作

### 1. 创建集合

```sql
CREATE COLLECTION collection_name
```

**示例：**
```sql
CREATE COLLECTION users
CREATE COLLECTION orders
CREATE COLLECTION products
```

**说明：**
- 创建一个新的集合
- 集合名称必须唯一（在同一数据库内）
- 通常不需要显式创建，插入文档时自动创建

---

### 2. 查看所有集合

```sql
SHOW COLLECTION
```

**示例输出：**
```
┌────────────┐
│ name       │
├────────────┤
│ users      │
│ orders     │
│ products   │
└────────────┘
```

**说明：**
- 列出当前数据库中的所有集合
- 注意：使用单数形式 `COLLECTION`

---

### 3. 删除集合

```sql
DROP COLLECTION collection_name
```

**示例：**
```sql
DROP COLLECTION temp_data
```

**警告：**
- 删除集合及其所有文档
- 操作不可逆

---

## 文档CRUD操作

### 1. 插入文档 (INSERT)

#### 插入单个文档

```sql
INSERT INTO collection_name {field1: value1, field2: value2, ...}
```

**示例：**
```sql
INSERT INTO users {"name": "Alice", "age": 25, "email": "alice@example.com"}

INSERT INTO products {
    "name": "Laptop",
    "price": 1299.99,
    "stock": 50,
    "tags": ["electronics", "computers"]
}
```

#### 插入多个文档

```sql
INSERT INTO collection_name [
    {field1: value1, ...},
    {field1: value1, ...},
    ...
]
```

**示例：**
```sql
INSERT INTO users [
    {"name": "Bob", "age": 30, "city": "Beijing"},
    {"name": "Charlie", "age": 35, "city": "Shanghai"},
    {"name": "David", "age": 28, "city": "Shenzhen"}
]
```

**返回值：**
```
Inserted 3 document(s)
3 document(s) affected
```

**注意事项：**
- 每个文档会自动生成唯一的 `_id` 字段
- 字段名必须用引号包裹（支持双引号或单引号）
- 支持嵌套文档和数组

---

### 2. 查询文档 (FIND)

#### 基本查询

```sql
FIND collection_name
```

**示例：**
```sql
FIND users
```

**输出：**
```
┌──────────────────┬────────┬─────┬──────────────────────┐
│ _id              │ name   │ age │ email                │
├──────────────────┼────────┼─────┼──────────────────────┤
│ 6586a78014520... │ Alice  │ 25  │ alice@example.com    │
│ 6586a780166a4... │ Bob    │ 30  │ bob@example.com      │
└──────────────────┴────────┴─────┴──────────────────────┘
```

---

#### 条件查询 (WHERE)

```sql
FIND collection_name WHERE condition
```

**比较运算符：**
- `=` - 等于
- `!=` 或 `<>` - 不等于
- `>` - 大于
- `>=` - 大于等于
- `<` - 小于
- `<=` - 小于等于

**示例：**
```sql
-- 查询年龄大于25的用户
FIND users WHERE age > 25

-- 查询名字为Alice的用户
FIND users WHERE name = "Alice"

-- 查询价格在100到1000之间的产品
FIND products WHERE price >= 100 AND price <= 1000
```

---

#### 逻辑运算符

```sql
-- AND 运算
FIND users WHERE age > 25 AND city = "Beijing"

-- OR 运算
FIND users WHERE age < 20 OR age > 60

-- NOT 运算
FIND users WHERE NOT active = false

-- 组合使用
FIND users WHERE (age > 25 AND city = "Beijing") OR (age < 20 AND city = "Shanghai")
```

---

#### IN 运算符

```sql
FIND collection_name WHERE field IN [value1, value2, ...]
```

**示例：**
```sql
FIND users WHERE age IN [25, 30, 35]
FIND products WHERE category IN ["electronics", "books", "toys"]
```

---

#### LIKE 运算符（模糊查询）

```sql
FIND collection_name WHERE field LIKE "pattern"
```

**通配符：**
- `%` - 匹配任意字符（0个或多个）
- `_` - 匹配单个字符

**示例：**
```sql
-- 查找名字以A开头的用户
FIND users WHERE name LIKE "A%"

-- 查找邮箱包含gmail的用户
FIND users WHERE email LIKE "%gmail%"

-- 查找名字为4个字符的用户
FIND users WHERE name LIKE "____"
```

---

#### BETWEEN 运算符

```sql
FIND collection_name WHERE field BETWEEN low AND high
```

**示例：**
```sql
FIND users WHERE age BETWEEN 20 AND 30
FIND orders WHERE total BETWEEN 100.0 AND 500.0
```

---

#### IS NULL / IS NOT NULL

```sql
FIND collection_name WHERE field IS NULL
FIND collection_name WHERE field IS NOT NULL
```

**示例：**
```sql
FIND users WHERE email IS NULL
FIND products WHERE description IS NOT NULL
```

---

#### 嵌套字段查询

使用点号 `.` 访问嵌套字段：

```sql
FIND collection_name WHERE nested.field = value
```

**示例：**
```sql
-- 假设文档结构：{"name": "Alice", "address": {"city": "Beijing", "zip": "100000"}}
FIND users WHERE address.city = "Beijing"
FIND users WHERE address.zip = "100000"
```

---

#### 字段投影 (SELECT)

```sql
FIND collection_name SELECT field1, field2, ...
```

**示例：**
```sql
-- 只返回name和age字段
FIND users SELECT name, age

-- 带条件的投影
FIND users WHERE age > 25 SELECT name, email
```

---

#### 排序 (ORDER BY)

```sql
FIND collection_name ORDER BY field1 ASC|DESC, field2 ASC|DESC, ...
```

**示例：**
```sql
-- 按年龄升序排序
FIND users ORDER BY age ASC

-- 按年龄降序排序
FIND users ORDER BY age DESC

-- 多字段排序：先按city升序，再按age降序
FIND users ORDER BY city ASC, age DESC
```

---

#### 分页 (LIMIT / SKIP)

```sql
FIND collection_name LIMIT n
FIND collection_name SKIP n
FIND collection_name LIMIT n SKIP m
```

**示例：**
```sql
-- 只返回前10条
FIND users LIMIT 10

-- 跳过前20条
FIND users SKIP 20

-- 分页：第3页，每页10条（跳过20条，返回10条）
FIND users LIMIT 10 SKIP 20

-- 完整查询示例
FIND users WHERE age > 25 ORDER BY age DESC LIMIT 10 SKIP 0
```

---

#### 组合查询示例

```sql
-- 查询年龄在25-35之间的北京用户，按年龄降序排序，返回前10条，只显示姓名和年龄
FIND users
WHERE age BETWEEN 25 AND 35 AND city = "Beijing"
SELECT name, age
ORDER BY age DESC
LIMIT 10
```

---

### 3. 更新文档 (UPDATE)

#### 基本语法

```sql
UPDATE collection_name SET field1 = value1, field2 = value2 WHERE condition
```

**示例：**
```sql
-- 更新单个字段
UPDATE users SET age = 26 WHERE name = "Alice"

-- 更新多个字段
UPDATE users SET age = 31, city = "Guangzhou" WHERE name = "Bob"

-- 更新所有文档（无WHERE子句）
UPDATE users SET active = true
```

---

#### 增量更新 (+=)

```sql
UPDATE collection_name SET field += value WHERE condition
```

**示例：**
```sql
-- 年龄增加1
UPDATE users SET age += 1 WHERE name = "Alice"

-- 库存减少5
UPDATE products SET stock += -5 WHERE id = "P001"

-- 价格增加10%
UPDATE products SET price += 129.99 WHERE name = "Laptop"
```

---

#### 删除字段 (UNSET)

```sql
UPDATE collection_name UNSET field1, field2 WHERE condition
```

**示例：**
```sql
-- 删除email字段
UPDATE users UNSET email WHERE name = "Alice"

-- 删除多个字段
UPDATE users UNSET phone, address WHERE age < 18
```

---

#### 数组操作 (PUSH)

```sql
UPDATE collection_name PUSH array_field = value WHERE condition
```

**示例：**
```sql
-- 向tags数组添加元素
UPDATE products PUSH tags = "sale" WHERE name = "Laptop"

-- 添加多个元素（需要分别执行）
UPDATE products PUSH tags = "featured" WHERE name = "Laptop"
```

---

#### 注意事项

- 默认更新所有匹配的文档（`multi: true`）
- UPDATE 操作不可逆，建议先用 FIND 验证条件
- 使用 `+=` 时，字段必须是数值类型

---

### 4. 删除文档 (DELETE)

#### 基本语法

```sql
DELETE FROM collection_name WHERE condition
```

**示例：**
```sql
-- 删除特定用户
DELETE FROM users WHERE name = "Alice"

-- 删除年龄小于18的用户
DELETE FROM users WHERE age < 18

-- 删除inactive用户
DELETE FROM users WHERE active = false

-- 删除所有文档（危险操作！）
DELETE FROM users
```

---

#### 注意事项

- 默认删除所有匹配的文档（`multi: true`）
- 无WHERE子句会删除集合中的所有文档
- 删除操作不可逆，建议先用 FIND 验证条件

---

## 索引管理

### 1. 创建索引

#### 普通索引

```sql
CREATE INDEX index_name ON collection_name (field1 ASC|DESC, field2 ASC|DESC, ...)
```

**示例：**
```sql
-- 单字段索引
CREATE INDEX idx_age ON users (age ASC)

-- 复合索引（多字段）
CREATE INDEX idx_city_age ON users (city ASC, age DESC)
```

---

#### 唯一索引

```sql
CREATE UNIQUE INDEX index_name ON collection_name (field ASC|DESC)
```

**示例：**
```sql
-- 邮箱唯一索引
CREATE UNIQUE INDEX idx_email ON users (email ASC)

-- 用户名唯一索引
CREATE UNIQUE INDEX idx_username ON users (username ASC)
```

**说明：**
- 唯一索引确保字段值在集合中唯一
- 插入重复值会导致错误

---

#### 全文索引

```sql
CREATE TEXT INDEX index_name ON collection_name (field)
```

**示例：**
```sql
-- 为文章标题创建全文索引
CREATE TEXT INDEX idx_title ON articles (title)

-- 为产品描述创建全文索引
CREATE TEXT INDEX idx_description ON products (description)
```

---

### 2. 删除索引

```sql
DROP INDEX index_name ON collection_name
```

**示例：**
```sql
DROP INDEX idx_age ON users
DROP INDEX idx_email ON users
```

---

### 3. 查看索引

```sql
SHOW INDEX ON collection_name
```

**示例：**
```sql
SHOW INDEX ON users
```

**输出示例：**
```
┌──────────────┬────────────────────┐
│ name         │ fields             │
├──────────────┼────────────────────┤
│ idx_email    │ ["email"]          │
│ idx_city_age │ ["city", "age"]    │
└──────────────┴────────────────────┘
```

---

### 索引最佳实践

1. **为常用查询字段创建索引**：WHERE、ORDER BY 中频繁使用的字段
2. **唯一约束字段使用唯一索引**：如 email、username
3. **复合索引顺序很重要**：高选择性字段在前
4. **避免过多索引**：索引会影响写入性能
5. **定期分析查询性能**：使用 EXPLAIN（计划支持）

---

## 聚合查询

### 基本语法

```sql
AGGREGATE collection_name
  | stage1
  | stage2
  | ...
```

### 聚合阶段

#### 1. MATCH（过滤）

```sql
AGGREGATE collection_name
  | MATCH condition
```

**示例：**
```sql
-- 过滤年龄大于25的用户
AGGREGATE users
  | MATCH age > 25

-- 过滤北京的用户
AGGREGATE users
  | MATCH city = "Beijing"
```

---

#### 2. GROUP BY（分组聚合）

```sql
AGGREGATE collection_name
  | GROUP BY field1, field2 AS {
      result_field1: FUNCTION(field),
      result_field2: FUNCTION(field),
      ...
    }
```

**聚合函数：**
- `COUNT()` - 计数
- `SUM(field)` - 求和
- `AVG(field)` - 平均值
- `MIN(field)` - 最小值
- `MAX(field)` - 最大值
- `FIRST(field)` - 第一个值
- `LAST(field)` - 最后一个值

**示例：**
```sql
-- 按城市分组统计用户数
AGGREGATE users
  | GROUP BY city AS {count: COUNT()}

-- 按城市分组，计算平均年龄
AGGREGATE users
  | GROUP BY city AS {avg_age: AVG(age), total: COUNT()}

-- 按类别分组，计算总销售额和平均价格
AGGREGATE products
  | GROUP BY category AS {
      total_sales: SUM(price),
      avg_price: AVG(price),
      count: COUNT()
    }
```

---

#### 3. SORT（排序）

```sql
AGGREGATE collection_name
  | SORT field1 ASC|DESC, field2 ASC|DESC
```

**示例：**
```sql
AGGREGATE users
  | GROUP BY city AS {count: COUNT()}
  | SORT count DESC
```

---

#### 4. LIMIT（限制结果数）

```sql
AGGREGATE collection_name
  | LIMIT n
```

**示例：**
```sql
-- 取前10条
AGGREGATE users
  | LIMIT 10
```

---

#### 5. SKIP（跳过记录）

```sql
AGGREGATE collection_name
  | SKIP n
```

**示例：**
```sql
-- 跳过前20条
AGGREGATE users
  | SKIP 20
```

---

#### 6. PROJECT（投影字段）

```sql
AGGREGATE collection_name
  | PROJECT field1, field2, ...
```

**示例：**
```sql
AGGREGATE users
  | PROJECT name, age
```

---

#### 7. UNWIND（展开数组）

```sql
AGGREGATE collection_name
  | UNWIND array_field
```

**示例：**
```sql
-- 假设文档：{"name": "Alice", "tags": ["python", "rust", "go"]}
AGGREGATE users
  | UNWIND tags
```

**结果：**
```
{"name": "Alice", "tags": "python"}
{"name": "Alice", "tags": "rust"}
{"name": "Alice", "tags": "go"}
```

---

### 完整聚合示例

#### 示例1：统计各城市用户数量，按数量降序排序

```sql
AGGREGATE users
  | GROUP BY city AS {user_count: COUNT()}
  | SORT user_count DESC
```

---

#### 示例2：计算各类别产品的平均价格，只显示前5个

```sql
AGGREGATE products
  | GROUP BY category AS {
      avg_price: AVG(price),
      total_products: COUNT()
    }
  | SORT avg_price DESC
  | LIMIT 5
```

---

#### 示例3：过滤后分组统计

```sql
AGGREGATE orders
  | MATCH status = "completed"
  | GROUP BY customer_id AS {
      total_amount: SUM(amount),
      order_count: COUNT()
    }
  | SORT total_amount DESC
  | LIMIT 10
```

---

#### 示例4：多阶段复杂聚合

```sql
AGGREGATE sales
  | MATCH date >= "2024-01-01"
  | GROUP BY product_id AS {
      total_revenue: SUM(amount),
      total_quantity: SUM(quantity),
      avg_price: AVG(price)
    }
  | SORT total_revenue DESC
  | LIMIT 20
```

---

## 事务管理

### 开始事务

```sql
BEGIN TRANSACTION
```

---

### 提交事务

```sql
COMMIT
```

---

### 回滚事务

```sql
ROLLBACK
```

---

### 事务示例

```sql
-- 开始事务
BEGIN TRANSACTION

-- 执行多个操作
INSERT INTO accounts {"user": "Alice", "balance": 1000}
UPDATE accounts SET balance += -100 WHERE user = "Alice"
UPDATE accounts SET balance += 100 WHERE user = "Bob"

-- 提交事务
COMMIT
```

**失败示例（回滚）：**
```sql
BEGIN TRANSACTION

UPDATE accounts SET balance += -1000 WHERE user = "Alice"
-- 发现余额不足，回滚事务
ROLLBACK
```

---

### 事务特性（ACID）

- **Atomicity（原子性）**：事务中的所有操作要么全部成功，要么全部失败
- **Consistency（一致性）**：事务执行前后，数据库保持一致状态
- **Isolation（隔离性）**：并发事务之间互不干扰
- **Durability（持久性）**：事务提交后，数据永久保存

---

### 隔离级别

MikuDB 支持以下隔离级别：
- **Read Committed（读已提交）** - 默认级别
- **Repeatable Read（可重复读）**
- **Snapshot Isolation（快照隔离）**

---

## 用户权限管理

### 1. 创建用户

```sql
CREATE USER username WITH PASSWORD 'password' ROLE role_name
```

**示例：**
```sql
CREATE USER alice WITH PASSWORD 'secret123' ROLE admin
CREATE USER bob WITH PASSWORD 'pass456' ROLE reader
```

---

### 2. 删除用户

```sql
DROP USER username
```

**示例：**
```sql
DROP USER alice
```

---

### 3. 授予权限

```sql
GRANT privilege ON resource TO username
```

**示例：**
```sql
GRANT read ON mydb.users TO bob
GRANT write ON mydb.* TO alice
GRANT admin ON *.* TO root
```

---

### 4. 撤销权限

```sql
REVOKE privilege ON resource FROM username
```

**示例：**
```sql
REVOKE write ON mydb.users FROM bob
```

---

### 5. 查看用户

```sql
SHOW USERS
```

---

## 系统命令

### 1. 查看服务器状态

```sql
SHOW STATUS
```

**输出示例：**
```json
{
  "version": "0.1.1",
  "uptime": "3 days, 2 hours",
  "connections": 42,
  "storage_size": "2.3 GB",
  "cache_hit_ratio": 0.95
}
```

---

### 2. 内置CLI命令

这些命令只在 `mikudb-cli` 中可用：

```bash
help           # 显示帮助信息
exit / quit    # 退出CLI
clear          # 清屏
status         # 显示连接状态
use <db>       # 切换数据库（同 USE 命令）
```

**示例：**
```
mikudb> help
mikudb> status
mikudb> clear
mikudb> exit
```

---

## 数据类型

MikuDB 支持以下BOML数据类型：

### 1. 基本类型

| 类型 | 示例 | 说明 |
|------|------|------|
| **Null** | `null` | 空值 |
| **Boolean** | `true`, `false` | 布尔值 |
| **Integer** | `42`, `-100` | 整数（i64） |
| **Float** | `3.14`, `-0.5` | 浮点数（f64） |
| **String** | `"hello"`, `'world'` | 字符串（UTF-8） |

---

### 2. 复杂类型

| 类型 | 示例 | 说明 |
|------|------|------|
| **Array** | `[1, 2, 3]`, `["a", "b"]` | 数组 |
| **Document** | `{"name": "Alice", "age": 25}` | 嵌套文档（对象） |

---

### 3. 特殊类型

| 类型 | 示例 | 说明 |
|------|------|------|
| **ObjectId** | 自动生成 | 12字节唯一标识符 |
| **Binary** | 二进制数据 | 字节数组 |
| **DateTime** | `"2024-01-15T10:30:00Z"` | 日期时间 |

---

### 数据类型示例

```sql
INSERT INTO examples {
    "null_field": null,
    "bool_field": true,
    "int_field": 42,
    "float_field": 3.14,
    "string_field": "Hello MikuDB",
    "array_field": [1, 2, 3, "mixed"],
    "object_field": {
        "nested": "value",
        "count": 10
    }
}
```

---

## 查询优化建议

### 1. 使用索引

```sql
-- 为常用查询字段创建索引
CREATE INDEX idx_age ON users (age ASC)

-- 查询会自动使用索引
FIND users WHERE age > 25
```

---

### 2. 字段投影

```sql
-- 只查询需要的字段，减少数据传输
FIND users WHERE age > 25 SELECT name, email
```

---

### 3. 分页查询

```sql
-- 避免一次查询大量数据
FIND users LIMIT 100 SKIP 0
```

---

### 4. 使用聚合代替客户端计算

```sql
-- 服务器端聚合更高效
AGGREGATE users
  | GROUP BY city AS {count: COUNT(), avg_age: AVG(age)}
```

---

## 常见错误处理

### 1. 语法错误

```
Error: Parse error: Expected identifier, got ...
```

**解决方法：**
- 检查关键字拼写
- 确保字段名用引号包裹
- 检查括号、逗号是否匹配

---

### 2. 集合不存在

```
Error: Storage error: Collection not found: users
```

**解决方法：**
- 先创建集合：`CREATE COLLECTION users`
- 或直接插入数据（自动创建集合）

---

### 3. 类型不匹配

```
Error: Type mismatch: expected number, got string
```

**解决方法：**
- 检查字段值类型是否正确
- 数值不要加引号：`age: 25` 而非 `age: "25"`

---

### 4. 认证失败

```
Error: Authentication failed
```

**解决方法：**
- 检查用户名和密码是否正确
- 确保服务器启用了认证（默认用户：`miku`，密码：`mikumiku3939`）

---

## 完整示例：电商系统

### 1. 创建集合和插入数据

```sql
-- 切换数据库
USE ecommerce

-- 创建用户集合
CREATE COLLECTION users

-- 插入用户
INSERT INTO users [
    {"username": "alice", "email": "alice@shop.com", "age": 28, "city": "Beijing"},
    {"username": "bob", "email": "bob@shop.com", "age": 35, "city": "Shanghai"},
    {"username": "charlie", "email": "charlie@shop.com", "age": 42, "city": "Guangzhou"}
]

-- 创建产品集合
CREATE COLLECTION products

-- 插入产品
INSERT INTO products [
    {"name": "Laptop", "price": 999.99, "category": "electronics", "stock": 50},
    {"name": "Mouse", "price": 29.99, "category": "electronics", "stock": 200},
    {"name": "Keyboard", "price": 79.99, "category": "electronics", "stock": 100},
    {"name": "Book", "price": 19.99, "category": "books", "stock": 500}
]

-- 创建订单集合
CREATE COLLECTION orders

-- 插入订单
INSERT INTO orders [
    {"user": "alice", "product": "Laptop", "quantity": 1, "total": 999.99, "status": "completed"},
    {"user": "bob", "product": "Mouse", "quantity": 2, "total": 59.98, "status": "pending"},
    {"user": "alice", "product": "Keyboard", "quantity": 1, "total": 79.99, "status": "completed"}
]
```

---

### 2. 创建索引

```sql
-- 用户名唯一索引
CREATE UNIQUE INDEX idx_username ON users (username ASC)

-- 邮箱唯一索引
CREATE UNIQUE INDEX idx_email ON users (email ASC)

-- 产品类别索引
CREATE INDEX idx_category ON products (category ASC)

-- 订单状态索引
CREATE INDEX idx_status ON orders (status ASC)
```

---

### 3. 常见查询

```sql
-- 查询所有电子产品
FIND products WHERE category = "electronics"

-- 查询库存少于100的产品
FIND products WHERE stock < 100 SELECT name, stock

-- 查询已完成的订单
FIND orders WHERE status = "completed"

-- 查询Alice的所有订单
FIND orders WHERE user = "alice"

-- 查询价格在50-100之间的产品
FIND products WHERE price BETWEEN 50 AND 100
```

---

### 4. 聚合统计

```sql
-- 统计各类别产品数量
AGGREGATE products
  | GROUP BY category AS {count: COUNT(), avg_price: AVG(price)}

-- 统计每个用户的订单总额
AGGREGATE orders
  | GROUP BY user AS {total_spent: SUM(total), order_count: COUNT()}
  | SORT total_spent DESC

-- 统计已完成订单的总销售额
AGGREGATE orders
  | MATCH status = "completed"
  | GROUP BY product AS {revenue: SUM(total), quantity: SUM(quantity)}
  | SORT revenue DESC
```

---

### 5. 更新操作

```sql
-- 产品降价10%
UPDATE products SET price += -10 WHERE category = "electronics"

-- 订单状态更新
UPDATE orders SET status = "shipped" WHERE status = "pending"

-- 库存减少
UPDATE products SET stock += -1 WHERE name = "Laptop"
```

---

### 6. 事务示例（下单流程）

```sql
BEGIN TRANSACTION

-- 1. 检查库存（假设通过应用层逻辑验证）
-- 2. 减少库存
UPDATE products SET stock += -1 WHERE name = "Laptop"

-- 3. 创建订单
INSERT INTO orders {"user": "charlie", "product": "Laptop", "quantity": 1, "total": 999.99, "status": "pending"}

-- 4. 提交事务
COMMIT
```

---

## 性能调优建议

### 1. 索引策略
- 为WHERE、ORDER BY、JOIN字段创建索引
- 复合索引：高选择性字段在前
- 避免过多索引（影响写入性能）

### 2. 查询优化
- 使用字段投影减少数据传输
- 分页查询避免全表扫描
- 使用聚合代替客户端计算

### 3. 数据模型设计
- 合理使用嵌套文档
- 避免过深的嵌套层级（建议≤3层）
- 大数组字段考虑拆分到独立集合

### 4. 批量操作
- 使用批量插入代替单条插入
- 批量更新减少网络往返

---

## 附录：MQL语法速查表

### 数据库操作
```sql
USE database_name
SHOW DATABASE
CREATE DATABASE database_name
DROP DATABASE database_name
```

### 集合操作
```sql
SHOW COLLECTION
CREATE COLLECTION collection_name
DROP COLLECTION collection_name
```

### CRUD操作
```sql
INSERT INTO collection {field: value}
FIND collection WHERE condition
UPDATE collection SET field = value WHERE condition
DELETE FROM collection WHERE condition
```

### 索引操作
```sql
CREATE INDEX idx_name ON collection (field ASC|DESC)
CREATE UNIQUE INDEX idx_name ON collection (field)
DROP INDEX idx_name ON collection
SHOW INDEX ON collection
```

### 聚合操作
```sql
AGGREGATE collection
  | MATCH condition
  | GROUP BY field AS {result: FUNCTION(field)}
  | SORT field ASC|DESC
  | LIMIT n
```

### 事务操作
```sql
BEGIN TRANSACTION
COMMIT
ROLLBACK
```

---

## 获取帮助

- **官方文档**: https://mikudb.org/docs
- **GitHub**: https://github.com/mikudb/mikudb
- **CLI帮助**: 在 `mikudb-cli` 中输入 `help`

---

**版本**: MikuDB v0.1.1
**最后更新**: 2026-01-02
