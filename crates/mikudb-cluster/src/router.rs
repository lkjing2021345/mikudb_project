//! 查询路由器

use crate::{ClusterError, ClusterResult, HealthStatus, Node, NodeRole};
use crate::replication::ReadPreference;
use dashmap::DashMap;
use std::sync::Arc;
use tracing::debug;

/// 查询路由器
pub struct QueryRouter {
    nodes: Arc<DashMap<String, Node>>,
}

impl QueryRouter {
    /// 创建查询路由器
    pub async fn new(nodes: Arc<DashMap<String, Node>>) -> ClusterResult<Self> {
        Ok(Self { nodes })
    }

    /// 路由读请求
    pub async fn route_read(&self, preference: ReadPreference) -> ClusterResult<String> {
        match preference {
            ReadPreference::Primary => self.get_leader(),
            ReadPreference::Secondary => self.get_follower(),
            ReadPreference::SecondaryPreferred => {
                self.get_follower().or_else(|_| self.get_leader())
            }
            ReadPreference::Nearest => self.get_nearest(),
        }
    }

    /// 路由写请求
    pub async fn route_write(&self) -> ClusterResult<String> {
        self.get_leader()
    }

    /// 获取 Leader 节点
    fn get_leader(&self) -> ClusterResult<String> {
        self.nodes
            .iter()
            .find(|n| n.role == NodeRole::Leader)
            .map(|n| n.id.clone())
            .ok_or(ClusterError::NodeNotFound("No leader found".into()))
    }

    /// 获取 Follower 节点
    fn get_follower(&self) -> ClusterResult<String> {
        self.nodes
            .iter()
            .find(|n| n.role == NodeRole::Follower && n.health == HealthStatus::Healthy)
            .map(|n| n.id.clone())
            .ok_or(ClusterError::NodeNotFound(
                "No healthy follower found".into(),
            ))
    }

    /// 获取最近节点
    fn get_nearest(&self) -> ClusterResult<String> {
        // 简单实现: 优先 Follower,否则 Leader
        self.get_follower().or_else(|_| self.get_leader())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    #[tokio::test]
    async fn test_router_creation() {
        let nodes = Arc::new(DashMap::new());
        let router = QueryRouter::new(nodes).await.unwrap();
        // 基本创建测试
    }
}
