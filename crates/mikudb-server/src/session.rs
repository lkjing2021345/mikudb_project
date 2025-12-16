use dashmap::DashMap;
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

static SESSION_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub struct Session {
    id: u64,
    username: String,
    database: RwLock<Option<String>>,
    created_at: Instant,
    last_activity: RwLock<Instant>,
    transaction_id: RwLock<Option<u64>>,
}

impl Session {
    pub fn new(username: String) -> Self {
        Self {
            id: SESSION_ID_COUNTER.fetch_add(1, Ordering::SeqCst),
            username,
            database: RwLock::new(None),
            created_at: Instant::now(),
            last_activity: RwLock::new(Instant::now()),
            transaction_id: RwLock::new(None),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn database(&self) -> Option<String> {
        self.database.read().clone()
    }

    pub fn set_database(&self, db: String) {
        *self.database.write() = Some(db);
    }

    pub fn touch(&self) {
        *self.last_activity.write() = Instant::now();
    }

    pub fn idle_duration(&self) -> Duration {
        self.last_activity.read().elapsed()
    }

    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    pub fn transaction_id(&self) -> Option<u64> {
        *self.transaction_id.read()
    }

    pub fn set_transaction(&self, txn_id: Option<u64>) {
        *self.transaction_id.write() = txn_id;
    }

    pub fn in_transaction(&self) -> bool {
        self.transaction_id.read().is_some()
    }
}

pub struct SessionManager {
    sessions: DashMap<u64, Arc<Session>>,
    timeout: Duration,
}

impl SessionManager {
    pub fn new(timeout: Duration) -> Self {
        Self {
            sessions: DashMap::new(),
            timeout,
        }
    }

    pub fn create_session(&self, username: String) -> Arc<Session> {
        let session = Arc::new(Session::new(username));
        self.sessions.insert(session.id(), session.clone());
        session
    }

    pub fn get_session(&self, id: u64) -> Option<Arc<Session>> {
        self.sessions.get(&id).map(|s| {
            s.touch();
            s.clone()
        })
    }

    pub fn remove_session(&self, id: u64) -> Option<Arc<Session>> {
        self.sessions.remove(&id).map(|(_, s)| s)
    }

    pub fn active_count(&self) -> usize {
        self.sessions.len()
    }

    pub fn cleanup_expired(&self) -> usize {
        let expired: Vec<u64> = self.sessions
            .iter()
            .filter(|s| s.idle_duration() > self.timeout)
            .map(|s| s.id())
            .collect();

        let count = expired.len();
        for id in expired {
            self.sessions.remove(&id);
        }
        count
    }

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

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub id: u64,
    pub username: String,
    pub database: Option<String>,
    pub age_secs: u64,
    pub idle_secs: u64,
    pub in_transaction: bool,
}
