//! 集群配置管理

use crate::error::{ClusterError, ClusterResult};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::Duration;

/// 集群配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// 集群名称
    pub cluster_name: String,
    /// 本节点 ID
    pub node_id: String,
    /// 监听地址
    pub bind_addr: SocketAddr,
    /// 种子节点列表
    pub seeds: Vec<String>,
    /// Raft 配置
    pub raft: RaftConfig,
    /// 复制配置
    pub replication: ReplicationConfig,
}

impl ClusterConfig {
    /// 从连接字符串解析配置
    ///
    /// # Arguments
    /// * `connection_string` - 格式: "mikudb://node1:port,node2:port,node3:port"
    pub fn from_connection_string(connection_string: &str) -> ClusterResult<Self> {
        // 简单实现
        if !connection_string.starts_with("mikudb://") {
            return Err(ClusterError::Config(
                "Connection string must start with 'mikudb://'".into(),
            ));
        }

        let nodes_str = connection_string.trim_start_matches("mikudb://");
        let seeds: Vec<String> = nodes_str.split(',').map(|s| s.to_string()).collect();

        if seeds.is_empty() {
            return Err(ClusterError::Config("No seeds provided".into()));
        }

        // 使用第一个节点作为默认配置
        let bind_addr: SocketAddr = seeds[0]
            .parse()
            .map_err(|_| ClusterError::Config("Invalid seed address".into()))?;

        Ok(Self {
            cluster_name: "mikudb".to_string(),
            node_id: format!("node_{}", bind_addr.port()),
            bind_addr,
            seeds,
            raft: RaftConfig::default(),
            replication: ReplicationConfig::default(),
        })
    }

    /// 从文件加载配置
    pub fn load(_path: &str) -> ClusterResult<Self> {
        // TODO: 实现从文件加载
        Ok(Self::default())
    }
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            cluster_name: "mikudb".to_string(),
            node_id: "node1".to_string(),
            bind_addr: "127.0.0.1:3940".parse().unwrap(),
            seeds: vec![],
            raft: RaftConfig::default(),
            replication: ReplicationConfig::default(),
        }
    }
}

/// Raft 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftConfig {
    /// 心跳间隔 (毫秒)
    pub heartbeat_interval_ms: u64,
    /// 选举超时最小值 (毫秒)
    pub election_timeout_min_ms: u64,
    /// 选举超时最大值 (毫秒)
    pub election_timeout_max_ms: u64,
    /// 快照触发阈值 (日志条目数)
    pub snapshot_threshold: u64,
}

impl Default for RaftConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval_ms: 100,
            election_timeout_min_ms: 150,
            election_timeout_max_ms: 300,
            snapshot_threshold: 10000,
        }
    }
}

/// 复制配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConfig {
    /// 复制模式
    pub mode: ReplicationMode,
    /// 写确认级别
    pub write_concern: WriteConcern,
    /// 最大复制延迟 (秒)
    pub max_lag_seconds: u64,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            mode: ReplicationMode::SemiSync,
            write_concern: WriteConcern::Majority(2),
            max_lag_seconds: 10,
        }
    }
}

/// 复制模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplicationMode {
    /// 异步复制 (高性能,可能丢数据)
    Async,
    /// 半同步复制 (至少1个从节点确认)
    SemiSync,
    /// 同步复制 (所有节点确认,强一致性)
    Sync,
}

/// 写确认级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WriteConcern {
    /// 主节点写入成功即返回
    One,
    /// 至少 N 个节点写入成功
    Majority(usize),
    /// 所有节点写入成功
    All,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_string_parse() {
        let config = ClusterConfig::from_connection_string("mikudb://localhost:3940").unwrap();
        assert_eq!(config.bind_addr.port(), 3940);
        assert_eq!(config.seeds.len(), 1);
    }

    #[test]
    fn test_invalid_connection_string() {
        let result = ClusterConfig::from_connection_string("invalid://localhost:3940");
        assert!(result.is_err());
    }
}
