//! Raft 共识算法实现

use crate::{ClusterConfig, ClusterError, ClusterResult};
use mikudb_boml::Document;
use mikudb_common::ObjectId;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::info;

/// Raft 节点
pub struct RaftNode {
    config: ClusterConfig,
}

impl RaftNode {
    /// 创建 Raft 节点
    pub async fn new(config: ClusterConfig) -> ClusterResult<Self> {
        info!("Creating Raft node: {}", config.node_id);
        Ok(Self { config })
    }

    /// 启动 Raft 节点
    pub async fn start(&self) -> ClusterResult<()> {
        info!("Starting Raft node: {}", self.config.node_id);
        // TODO: 实现 Raft 启动逻辑
        Ok(())
    }

    /// 提交命令
    pub async fn propose(&self, command: Command) -> ClusterResult<()> {
        // TODO: 实现命令提交到 Raft
        Ok(())
    }
}

/// Raft 日志条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// 日志索引
    pub index: u64,
    /// 任期号
    pub term: u64,
    /// 命令
    pub command: Command,
}

/// Raft 命令
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    /// 写入文档
    Write {
        collection: String,
        doc: Document,
    },
    /// 删除文档
    Delete {
        collection: String,
        doc_id: ObjectId,
    },
    /// 配置变更
    ConfigChange {
        node_id: String,
        addr: String,
        action: ConfigAction,
    },
}

/// 配置变更操作
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigAction {
    /// 添加节点
    Add,
    /// 移除节点
    Remove,
}
