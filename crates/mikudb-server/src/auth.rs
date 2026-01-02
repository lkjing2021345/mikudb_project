//! 用户认证模块
//!
//! 本模块实现用户身份验证和权限管理:
//! - SHA-256 密码哈希和 Base64 编码
//! - 用户身份验证
//! - 基于角色的访问控制 (RBAC)
//! - 数据库级别权限检查

use crate::config::AuthConfig;
use crate::{ServerError, ServerResult};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use mikudb_storage::StorageEngine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use chrono::{DateTime, Utc};

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
/// 定义数据库操作的细粒度权限。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Privilege {
    Find,
    Count,
    Aggregate,
    ListCollections,
    ListIndexes,
    Insert,
    Update,
    Delete,
    CreateCollection,
    DropCollection,
    RenameCollection,
    CreateIndex,
    DropIndex,
    DropDatabase,
    CompactDatabase,
    CreateUser,
    UpdateUser,
    DropUser,
    GrantRole,
    RevokeRole,
    AddNode,
    RemoveNode,
    Shutdown,
    ServerStatus,
}

/// 权限类型枚举 (兼容旧代码)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    Read,
    Write,
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
    match permission {
        Permission::Read => user.has_role("read") || user.has_role("readWrite") || user.has_role("root"),
        Permission::Write => user.has_role("readWrite") || user.has_role("root"),
        Permission::Admin => user.has_role("root"),
    }
}

/// 角色分配
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoleAssignment {
    pub role: String,
    pub db: String,
}

/// 用户凭证 (SCRAM-SHA-256)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCredentials {
    pub salt: String,
    pub stored_key: String,
    pub server_key: String,
    pub iterations: u32,
}

/// 持久化用户对象
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredUser {
    #[serde(rename = "_id")]
    pub id: String,
    pub username: String,
    pub credentials: UserCredentials,
    pub roles: Vec<RoleAssignment>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 用户管理器
pub struct UserManager {
    storage: Arc<StorageEngine>,
}

impl UserManager {
    pub fn new(storage: Arc<StorageEngine>) -> Self {
        Self { storage }
    }

    pub async fn initialize(&self) -> ServerResult<()> {
        use mikudb_boml::Document;
        use tracing::warn;

        let admin_db = "admin";
        let users_collection = "users";

        let collections = self.storage.list_collections(admin_db)?;
        if !collections.contains(&users_collection.to_string()) {
            warn!("Initializing authentication system...");
            warn!("Creating admin.users collection");

            let initial_password = "mikudb_initial_password";
            let root_user = self.create_user_internal(
                "root",
                initial_password,
                vec![RoleAssignment {
                    role: "root".to_string(),
                    db: "*".to_string(),
                }],
            )?;

            let collection = self.storage.get_or_create_collection(&format!("{}:{}", admin_db, users_collection))?;

            let mut user_doc = Document::new();
            user_doc.insert("_id".to_string(), root_user.id.clone().into());
            user_doc.insert("username".to_string(), root_user.username.into());

            let mut cred_doc = Document::new();
            cred_doc.insert("salt".to_string(), root_user.credentials.salt.into());
            cred_doc.insert("storedKey".to_string(), root_user.credentials.stored_key.into());
            cred_doc.insert("serverKey".to_string(), root_user.credentials.server_key.into());
            cred_doc.insert("iterations".to_string(), (root_user.credentials.iterations as i64).into());
            user_doc.insert("credentials".to_string(), cred_doc.into());

            let roles_vec: Vec<mikudb_boml::BomlValue> = root_user.roles.iter().map(|r| {
                let mut role_doc = Document::new();
                role_doc.insert("role".to_string(), r.role.clone().into());
                role_doc.insert("db".to_string(), r.db.clone().into());
                role_doc.into()
            }).collect();
            user_doc.insert("roles".to_string(), roles_vec.into());

            collection.insert(user_doc)?;

            warn!("⚠️  Initial root user created");
            warn!("⚠️  Username: root");
            warn!("⚠️  Password: {}", initial_password);
            warn!("⚠️  Please change the password immediately using:");
            warn!("   ALTER USER \"root\" PASSWORD \"your_secure_password\";");
        }

        Ok(())
    }

    fn create_user_internal(
        &self,
        username: &str,
        password: &str,
        roles: Vec<RoleAssignment>,
    ) -> ServerResult<StoredUser> {
        let credentials = create_scram_credentials(password, 10000)?;
        Ok(StoredUser {
            id: username.to_string(),
            username: username.to_string(),
            credentials,
            roles,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    pub async fn authenticate(&self, username: &str, password: &str) -> ServerResult<User> {
        use mikudb_boml::{Document, BomlValue};

        let admin_db = "admin";
        let users_collection = "users";

        let collection = self.storage.get_or_create_collection(&format!("{}:{}", admin_db, users_collection))?;

        let mut filter = Document::new();
        filter.insert("username".to_string(), username.into());
        let docs = collection.find(Some(filter), None)?;

        if docs.is_empty() {
            return Err(ServerError::AuthFailed("User not found".to_string()));
        }

        let user_doc = &docs[0];
        let stored_username = user_doc.get("username")
            .and_then(|v| if let BomlValue::String(s) = v { Some(s.as_str()) } else { None })
            .ok_or_else(|| ServerError::Internal("Missing username field".to_string()))?;

        let cred_doc = user_doc.get("credentials")
            .and_then(|v| if let BomlValue::Document(d) = v { Some(d) } else { None })
            .ok_or_else(|| ServerError::Internal("Missing credentials field".to_string()))?;

        let credentials = UserCredentials {
            salt: cred_doc.get("salt")
                .and_then(|v| if let BomlValue::String(s) = v { Some(s.clone()) } else { None })
                .ok_or_else(|| ServerError::Internal("Missing salt".to_string()))?,
            stored_key: cred_doc.get("storedKey")
                .and_then(|v| if let BomlValue::String(s) = v { Some(s.clone()) } else { None })
                .ok_or_else(|| ServerError::Internal("Missing storedKey".to_string()))?,
            server_key: cred_doc.get("serverKey")
                .and_then(|v| if let BomlValue::String(s) = v { Some(s.clone()) } else { None })
                .ok_or_else(|| ServerError::Internal("Missing serverKey".to_string()))?,
            iterations: cred_doc.get("iterations")
                .and_then(|v| if let BomlValue::Int64(i) = v { Some(*i as u32) } else { None })
                .unwrap_or(10000),
        };

        if !verify_scram_password(password, &credentials)? {
            return Err(ServerError::AuthFailed("Invalid password".to_string()));
        }

        let roles_vec = user_doc.get("roles")
            .and_then(|v| if let BomlValue::Array(arr) = v { Some(arr) } else { None })
            .ok_or_else(|| ServerError::Internal("Missing roles".to_string()))?;

        let mut roles = Vec::new();
        let mut databases = Vec::new();
        for role_val in roles_vec {
            if let BomlValue::Document(role_doc) = role_val {
                if let (Some(BomlValue::String(role)), Some(BomlValue::String(db))) =
                    (role_doc.get("role"), role_doc.get("db")) {
                    roles.push(role.clone());
                    databases.push(db.clone());
                }
            }
        }

        Ok(User {
            username: stored_username.to_string(),
            password_hash: String::new(),
            roles,
            databases,
        })
    }
}

fn create_scram_credentials(password: &str, iterations: u32) -> ServerResult<UserCredentials> {
    use rand::Rng;

    let mut rng = rand::thread_rng();
    let salt: Vec<u8> = (0..16).map(|_| rng.gen()).collect();

    let salted_password = pbkdf2_sha256(password.as_bytes(), &salt, iterations);

    let client_key = hmac_sha256(&salted_password, b"Client Key");
    let stored_key = sha256(&client_key);
    let server_key = hmac_sha256(&salted_password, b"Server Key");

    Ok(UserCredentials {
        salt: BASE64.encode(&salt),
        stored_key: BASE64.encode(&stored_key),
        server_key: BASE64.encode(&server_key),
        iterations,
    })
}

fn verify_scram_password(password: &str, credentials: &UserCredentials) -> ServerResult<bool> {
    let salt = BASE64
        .decode(&credentials.salt)
        .map_err(|e| ServerError::Internal(format!("Invalid salt: {}", e)))?;

    let salted_password = pbkdf2_sha256(password.as_bytes(), &salt, credentials.iterations);
    let client_key = hmac_sha256(&salted_password, b"Client Key");
    let stored_key = sha256(&client_key);

    Ok(BASE64.encode(&stored_key) == credentials.stored_key)
}

fn pbkdf2_sha256(password: &[u8], salt: &[u8], iterations: u32) -> Vec<u8> {
    use sha2::Sha256;
    let mut result = vec![0u8; 32];
    pbkdf2::pbkdf2::<hmac::Hmac<Sha256>>(password, salt, iterations, &mut result)
        .expect("PBKDF2 derivation failed");
    result
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    use hmac::{Hmac, Mac};
    use sha2::Sha256;
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC initialization failed");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

fn sha256(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}
