use crate::config::AuthConfig;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use sha2::{Digest, Sha256};

pub struct Authenticator {
    default_user: String,
    default_password_hash: String,
}

impl Authenticator {
    pub fn new(config: &AuthConfig) -> Self {
        Self {
            default_user: config.default_user.clone(),
            default_password_hash: hash_password(&config.default_password),
        }
    }

    pub fn verify(&self, username: &str, password: &str) -> bool {
        if username == self.default_user {
            let password_hash = hash_password(password);
            return password_hash == self.default_password_hash;
        }
        false
    }

    pub fn hash_password(password: &str) -> String {
        hash_password(password)
    }
}

fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let result = hasher.finalize();
    BASE64.encode(result)
}

#[derive(Debug, Clone)]
pub struct User {
    pub username: String,
    pub password_hash: String,
    pub roles: Vec<String>,
    pub databases: Vec<String>,
}

impl User {
    pub fn new(username: String, password: &str) -> Self {
        Self {
            username,
            password_hash: hash_password(password),
            roles: vec!["readWrite".to_string()],
            databases: vec![],
        }
    }

    pub fn verify_password(&self, password: &str) -> bool {
        hash_password(password) == self.password_hash
    }

    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role || r == "root")
    }

    pub fn can_access_database(&self, database: &str) -> bool {
        self.has_role("root") || self.databases.is_empty() || self.databases.contains(&database.to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    Read,
    Write,
    Admin,
}

pub fn check_permission(user: &User, _database: &str, _collection: &str, permission: Permission) -> bool {
    match permission {
        Permission::Read => user.has_role("read") || user.has_role("readWrite") || user.has_role("root"),
        Permission::Write => user.has_role("readWrite") || user.has_role("root"),
        Permission::Admin => user.has_role("root"),
    }
}
