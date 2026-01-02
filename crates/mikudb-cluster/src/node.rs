//! 节点管理模块

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::SystemTime;

/// 集群节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    /// 节点 ID
    pub id: String,
    /// 节点地址
    pub addr: SocketAddr,
    /// 节点角色
    pub role: NodeRole,
    /// 健康状态
    pub health: HealthStatus,
    /// 最后心跳时间
    pub last_heartbeat: SystemTime,
}

impl Node {
    /// 创建新节点
    pub fn new(id: String, addr: SocketAddr) -> Self {
        Self {
            id,
            addr,
            role: NodeRole::Follower,
            health: HealthStatus::Healthy,
            last_heartbeat: SystemTime::now(),
        }
    }

    /// 更新心跳时间
    pub fn update_heartbeat(&mut self) {
        self.last_heartbeat = SystemTime::now();
    }

    /// 检查是否健康
    pub fn is_healthy(&self, timeout_secs: u64) -> bool {
        if let Ok(elapsed) = self.last_heartbeat.elapsed() {
            elapsed.as_secs() < timeout_secs
        } else {
            false
        }
    }
}

/// 节点角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeRole {
    /// 领导者
    Leader,
    /// 跟随者
    Follower,
    /// 候选者
    Candidate,
}

/// 健康状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// 健康
    Healthy,
    /// 疑似故障
    Suspected,
    /// 故障
    Failed,
}

/// 节点状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeState {
    /// 启动中
    Starting,
    /// 运行中
    Running,
    /// 停止中
    Stopping,
    /// 已停止
    Stopped,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;
    use std::time::Duration;

    #[test]
    fn test_node_creation() {
        let node = Node::new("node1".to_string(), "127.0.0.1:3940".parse().unwrap());
        assert_eq!(node.role, NodeRole::Follower);
        assert_eq!(node.health, HealthStatus::Healthy);
    }

    #[test]
    fn test_node_heartbeat() {
        let mut node = Node::new("node1".to_string(), "127.0.0.1:3940".parse().unwrap());
        assert!(node.is_healthy(10));

        // 模拟心跳超时
        sleep(Duration::from_secs(2));
        assert!(node.is_healthy(10)); // 2秒内仍健康

        assert!(!node.is_healthy(1)); // 超过1秒视为不健康
    }
}
