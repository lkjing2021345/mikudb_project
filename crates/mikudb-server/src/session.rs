//! 会话管理模块
//!
//! 本模块实现用户会话生命周期管理:
//! - 会话创建和销毁
//! - 会话超时检测和清理
//! - 事务状态跟踪
//! - 并发安全的会话访问(使用 DashMap)

use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// 全局会话 ID 计数器,为每个新会话生成唯一 ID
static SESSION_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// 用户会话
///
/// 表示一个已认证用户的会话,跟踪会话状态、活动时间和事务信息。
/// 所有可变字段使用 RwLock 保护以支持并发访问。
#[derive(Debug)]
pub struct Session {
    /// 会话唯一标识符
    id: u64,
    /// 用户名
    username: String,
    /// 当前数据库(可变)
    database: RwLock<Option<String>>,
    /// 会话创建时间
    created_at: Instant,
    /// 最后活动时间(可变)
    last_activity: RwLock<Instant>,
    /// 当前事务 ID(可变)
    transaction_id: RwLock<Option<u64>>,
}

impl Session {
    /// # Brief
    /// 创建新会话
    ///
    /// 分配全局唯一的会话 ID,初始化时间戳。
    ///
    /// # Arguments
    /// * `username` - 用户名
    ///
    /// # Returns
    /// 新的会话实例
    pub fn new(username: String) -> Self {
        Self {
            // 原子递增获取唯一 ID
            id: SESSION_ID_COUNTER.fetch_add(1, Ordering::SeqCst),
            username,
            database: RwLock::new(None),
            created_at: Instant::now(),
            last_activity: RwLock::new(Instant::now()),
            transaction_id: RwLock::new(None),
        }
    }

    /// # Brief
    /// 获取会话 ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// # Brief
    /// 获取用户名
    pub fn username(&self) -> &str {
        &self.username
    }

    /// # Brief
    /// 获取当前数据库
    ///
    /// # Returns
    /// 当前数据库名称,如果未设置则为 None
    pub fn database(&self) -> Option<String> {
        self.database.read().clone()
    }

    /// # Brief
    /// 设置当前数据库
    ///
    /// # Arguments
    /// * `db` - 数据库名称
    pub fn set_database(&self, db: String) {
        *self.database.write() = Some(db);
    }

    /// # Brief
    /// 更新最后活动时间
    ///
    /// 每次会话操作时应调用此方法以防止超时。
    pub fn touch(&self) {
        *self.last_activity.write() = Instant::now();
    }

    /// # Brief
    /// 获取空闲时间
    ///
    /// # Returns
    /// 自最后活动以来的时间长度
    pub fn idle_duration(&self) -> Duration {
        self.last_activity.read().elapsed()
    }

    /// # Brief
    /// 获取会话年龄
    ///
    /// # Returns
    /// 自会话创建以来的时间长度
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    /// # Brief
    /// 获取当前事务 ID
    pub fn transaction_id(&self) -> Option<u64> {
        *self.transaction_id.read()
    }

    /// # Brief
    /// 设置事务 ID
    ///
    /// # Arguments
    /// * `txn_id` - 事务 ID,None 表示结束事务
    pub fn set_transaction(&self, txn_id: Option<u64>) {
        *self.transaction_id.write() = txn_id;
    }

    /// # Brief
    /// 检查会话是否在事务中
    ///
    /// # Returns
    /// true 表示会话有活跃事务
    pub fn in_transaction(&self) -> bool {
        self.transaction_id.read().is_some()
    }
}

/// 会话管理器
///
/// 管理所有活跃会话,提供会话创建、查找、超时清理等功能。
/// 使用 DashMap 实现无锁并发访问。
pub struct SessionManager {
    /// 会话映射表 (session_id -> Session)
    sessions: DashMap<u64, Arc<Session>>,
    /// 会话超时时间
    timeout: Duration,
}

impl SessionManager {
    /// # Brief
    /// 创建新的会话管理器
    ///
    /// # Arguments
    /// * `timeout` - 会话超时时间
    ///
    /// # Returns
    /// 会话管理器实例
    pub fn new(timeout: Duration) -> Self {
        Self {
            sessions: DashMap::new(),
            timeout,
        }
    }

    /// # Brief
    /// 创建新会话
    ///
    /// 为用户创建新会话并添加到管理器中。
    ///
    /// # Arguments
    /// * `username` - 用户名
    ///
    /// # Returns
    /// 新创建的会话(Arc 包装)
    pub fn create_session(&self, username: String) -> Arc<Session> {
        let session = Arc::new(Session::new(username));
        // 插入到并发映射表
        self.sessions.insert(session.id(), session.clone());
        session
    }

    /// # Brief
    /// 获取会话
    ///
    /// 获取会话时自动更新最后活动时间。
    ///
    /// # Arguments
    /// * `id` - 会话 ID
    ///
    /// # Returns
    /// 会话实例(如果存在)
    pub fn get_session(&self, id: u64) -> Option<Arc<Session>> {
        self.sessions.get(&id).map(|s| {
            // 更新活动时间
            s.touch();
            s.clone()
        })
    }

    /// # Brief
    /// 移除会话
    ///
    /// # Arguments
    /// * `id` - 会话 ID
    ///
    /// # Returns
    /// 被移除的会话(如果存在)
    pub fn remove_session(&self, id: u64) -> Option<Arc<Session>> {
        self.sessions.remove(&id).map(|(_, s)| s)
    }

    /// # Brief
    /// 获取活跃会话数量
    pub fn active_count(&self) -> usize {
        self.sessions.len()
    }

    /// # Brief
    /// 清理过期会话
    ///
    /// 遍历所有会话,移除超过超时时间的空闲会话。
    /// 应定期调用以释放资源。
    ///
    /// # Returns
    /// 清理的会话数量
    pub fn cleanup_expired(&self) -> usize {
        // 收集过期会话 ID
        let expired: Vec<u64> = self.sessions
            .iter()
            .filter(|s| s.idle_duration() > self.timeout)
            .map(|s| s.id())
            .collect();

        let count = expired.len();
        // 批量移除
        for id in expired {
            self.sessions.remove(&id);
        }
        count
    }

    /// # Brief
    /// 列出所有会话信息
    ///
    /// # Returns
    /// 所有会话的快照信息列表
    pub fn list_sessions(&self) -> Vec<SessionInfo> {
        self.sessions
            .iter()
            .map(|s| SessionInfo {
                id: s.id(),
                username: s.username().to_string(),
                database: s.database(),
                age_secs: s.age().as_secs(),
                idle_secs: s.idle_duration().as_secs(),
                in_transaction: s.in_transaction(),
            })
            .collect()
    }
}

/// 会话信息快照
///
/// 用于展示会话状态的只读结构。
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: u64,
    pub username: String,
    pub database: Option<String>,
    pub age_secs: u64,
    pub idle_secs: u64,
    pub in_transaction: bool,
}
