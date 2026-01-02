//! 用户认证模块
//!
//! 本模块实现用户身份验证和权限管理:
//! - SHA-256 密码哈希和 Base64 编码
//! - 用户身份验证
//! - 基于角色的访问控制 (RBAC)
//! - 数据库级别权限检查

use crate::config::AuthConfig;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use sha2::{Digest, Sha256};

/// 认证器
///
/// 处理用户认证,当前支持单一默认用户。
pub struct Authenticator {
    /// 默认用户名
    default_user: String,
    /// 默认密码的 SHA-256 哈希值 (Base64 编码)
    default_password_hash: String,
}

impl Authenticator {
    /// # Brief
    /// 创建新的认证器
    ///
    /// 从配置文件加载默认用户名和密码,密码存储为 SHA-256 哈希值。
    ///
    /// # Arguments
    /// * `config` - 认证配置
    ///
    /// # Returns
    /// 初始化的认证器实例
    pub fn new(config: &AuthConfig) -> Self {
        Self {
            default_user: config.default_user.clone(),
            default_password_hash: hash_password(&config.default_password),
        }
    }

    /// # Brief
    /// 验证用户名和密码
    ///
    /// 将输入密码进行 SHA-256 哈希后与存储的哈希值比较。
    ///
    /// # Arguments
    /// * `username` - 用户名
    /// * `password` - 明文密码
    ///
    /// # Returns
    /// true 表示认证成功,false 表示认证失败
    pub fn verify(&self, username: &str, password: &str) -> bool {
        // 检查用户名是否匹配
        if username == self.default_user {
            // 计算密码哈希并比较
            let password_hash = hash_password(password);
            return password_hash == self.default_password_hash;
        }
        false
    }

    /// # Brief
    /// 对密码进行哈希处理 (公共接口)
    ///
    /// # Arguments
    /// * `password` - 明文密码
    ///
    /// # Returns
    /// Base64 编码的 SHA-256 哈希值
    pub fn hash_password(password: &str) -> String {
        hash_password(password)
    }
}

/// # Brief
/// 内部密码哈希函数
///
/// 使用 SHA-256 算法对密码进行哈希,结果用 Base64 编码。
///
/// # Arguments
/// * `password` - 明文密码
///
/// # Returns
/// Base64 编码的哈希值
fn hash_password(password: &str) -> String {
    // 创建 SHA-256 哈希器
    let mut hasher = Sha256::new();
    // 更新哈希器输入
    hasher.update(password.as_bytes());
    // 计算哈希值
    let result = hasher.finalize();
    // Base64 编码
    BASE64.encode(result)
}

/// 用户实体
///
/// 表示一个数据库用户,包含身份信息和权限。
#[derive(Debug, Clone)]
pub struct User {
    /// 用户名
    pub username: String,
    /// 密码哈希值 (SHA-256 + Base64)
    pub password_hash: String,
    /// 角色列表(如 "readWrite", "root")
    pub roles: Vec<String>,
    /// 可访问的数据库列表(空表示全部)
    pub databases: Vec<String>,
}

impl User {
    /// # Brief
    /// 创建新用户
    ///
    /// 默认赋予 readWrite 角色,可访问所有数据库。
    ///
    /// # Arguments
    /// * `username` - 用户名
    /// * `password` - 明文密码
    ///
    /// # Returns
    /// 新用户实例
    pub fn new(username: String, password: &str) -> Self {
        Self {
            username,
            password_hash: hash_password(password),
            roles: vec!["readWrite".to_string()],
            databases: vec![],
        }
    }

    /// # Brief
    /// 验证密码
    ///
    /// # Arguments
    /// * `password` - 明文密码
    ///
    /// # Returns
    /// true 表示密码正确,false 表示密码错误
    pub fn verify_password(&self, password: &str) -> bool {
        hash_password(password) == self.password_hash
    }

    /// # Brief
    /// 检查用户是否具有指定角色
    ///
    /// root 角色具有所有权限。
    ///
    /// # Arguments
    /// * `role` - 角色名称
    ///
    /// # Returns
    /// true 表示用户具有该角色
    pub fn has_role(&self, role: &str) -> bool {
        // 检查用户是否具有指定角色或 root 角色
        self.roles.iter().any(|r| r == role || r == "root")
    }

    /// # Brief
    /// 检查用户是否可访问指定数据库
    ///
    /// root 用户可访问所有数据库。
    /// 如果 databases 列表为空,表示可访问所有数据库。
    ///
    /// # Arguments
    /// * `database` - 数据库名称
    ///
    /// # Returns
    /// true 表示有访问权限
    pub fn can_access_database(&self, database: &str) -> bool {
        // root 角色或空数据库列表或数据库在列表中
        self.has_role("root") || self.databases.is_empty() || self.databases.contains(&database.to_string())
    }
}

/// 权限类型枚举
///
/// 定义三种基本权限类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    /// 读权限
    Read,
    /// 写权限
    Write,
    /// 管理员权限
    Admin,
}

/// # Brief
/// 检查用户对指定资源是否具有指定权限
///
/// 权限层级:
/// - Read: 需要 read, readWrite 或 root 角色
/// - Write: 需要 readWrite 或 root 角色
/// - Admin: 需要 root 角色
///
/// # Arguments
/// * `user` - 用户实例
/// * `_database` - 数据库名(预留,当前未使用)
/// * `_collection` - 集合名(预留,当前未使用)
/// * `permission` - 需要检查的权限类型
///
/// # Returns
/// true 表示具有权限,false 表示无权限
pub fn check_permission(user: &User, _database: &str, _collection: &str, permission: Permission) -> bool {
    // 根据权限类型检查角色
    match permission {
        Permission::Read => user.has_role("read") || user.has_role("readWrite") || user.has_role("root"),
        Permission::Write => user.has_role("readWrite") || user.has_role("root"),
        Permission::Admin => user.has_role("root"),
    }
}
