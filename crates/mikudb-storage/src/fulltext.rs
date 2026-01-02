//! 全文索引模块
//!
//! 实现基础的全文搜索功能:
//! - **中文分词**: 基于字符 N-gram 分词
//! - **倒排索引**: 词项 -> 文档 ID 列表
//! - **布尔查询**: AND、OR、NOT 操作
//! - **短语匹配**: 精确短语搜索
//! - **相关性评分**: TF-IDF 算法
//!
//! # 分词策略
//!
//! - **中文**: Bi-gram (2-gram) 分词,例如 "数据库" -> ["数据", "据库"]
//! - **英文**: 空格分词 + 小写化,例如 "Hello World" -> ["hello", "world"]
//! - **混合**: 识别中英文并分别处理
//!
//! # 倒排索引结构
//!
//! ```text
//! Term -> PostingList {
//!     doc_ids: Vec<ObjectId>,
//!     positions: Vec<Vec<usize>>,  // 每个文档中的位置
//!     frequencies: Vec<u32>,        // 每个文档中的词频
//! }
//! ```

use crate::{StorageError, StorageResult};
use mikudb_boml::BomlValue;
use mikudb_common::ObjectId;
use parking_lot::RwLock;
use rocksdb::{BoundColumnFamily, IteratorMode, WriteBatch, DB};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;
use tracing::{debug, info};

/// 全文索引定义
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullTextIndexDefinition {
    /// 索引名称
    pub name: String,
    /// 所属集合
    pub collection: String,
    /// 索引字段(支持多字段)
    pub fields: Vec<String>,
    /// 分词器类型
    pub tokenizer: TokenizerType,
    /// 最小词长度
    pub min_token_length: usize,
    /// 最大词长度
    pub max_token_length: usize,
    /// 是否存储位置信息(用于短语查询)
    pub store_positions: bool,
}

/// 分词器类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TokenizerType {
    /// 简单分词器(空格分词)
    Simple,
    /// 中文 N-gram 分词器
    ChineseNGram,
    /// 混合分词器(自动识别中英文)
    Mixed,
}

/// 全文索引引擎
pub struct FullTextIndex {
    /// 索引定义
    definition: FullTextIndexDefinition,
    /// RocksDB 实例
    db: Arc<DB>,
    /// 倒排索引缓存(词项 -> 倒排列表)
    inverted_index: RwLock<BTreeMap<String, PostingList>>,
    /// 文档统计(用于 TF-IDF)
    doc_stats: RwLock<DocumentStats>,
}

/// 倒排列表
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PostingList {
    /// 文档 ID 列表
    doc_ids: Vec<ObjectId>,
    /// 每个文档中的位置列表
    positions: Vec<Vec<usize>>,
    /// 每个文档中的词频
    frequencies: Vec<u32>,
}

impl PostingList {
    fn new() -> Self {
        Self {
            doc_ids: Vec::new(),
            positions: Vec::new(),
            frequencies: Vec::new(),
        }
    }

    /// 添加文档
    fn add_document(&mut self, doc_id: ObjectId, positions: Vec<usize>) {
        let frequency = positions.len() as u32;

        // 检查是否已存在
        if let Some(pos) = self.doc_ids.iter().position(|id| *id == doc_id) {
            // 更新现有文档
            self.positions[pos] = positions;
            self.frequencies[pos] = frequency;
        } else {
            // 添加新文档
            self.doc_ids.push(doc_id);
            self.positions.push(positions);
            self.frequencies.push(frequency);
        }
    }

    /// 删除文档
    fn remove_document(&mut self, doc_id: &ObjectId) -> bool {
        if let Some(pos) = self.doc_ids.iter().position(|id| id == doc_id) {
            self.doc_ids.remove(pos);
            self.positions.remove(pos);
            self.frequencies.remove(pos);
            true
        } else {
            false
        }
    }
}

/// 文档统计信息
#[derive(Debug, Default)]
struct DocumentStats {
    /// 总文档数
    total_docs: u64,
    /// 每个文档的词数
    doc_lengths: HashMap<ObjectId, usize>,
}

impl FullTextIndex {
    /// 创建全文索引
    pub fn new(definition: FullTextIndexDefinition, db: Arc<DB>) -> Self {
        Self {
            definition,
            db,
            inverted_index: RwLock::new(BTreeMap::new()),
            doc_stats: RwLock::new(DocumentStats::default()),
        }
    }

    /// 索引文档
    ///
    /// # Arguments
    /// * `doc_id` - 文档 ID
    /// * `text` - 要索引的文本
    pub fn index_document(&self, doc_id: ObjectId, text: &str) -> StorageResult<()> {
        // 分词
        let tokens = self.tokenize(text);

        // 统计词频和位置
        let mut term_positions: HashMap<String, Vec<usize>> = HashMap::new();
        for (pos, token) in tokens.iter().enumerate() {
            term_positions.entry(token.clone()).or_default().push(pos);
        }

        let term_count = term_positions.len();

        // 更新倒排索引
        let mut inverted_index = self.inverted_index.write();
        for (term, positions) in term_positions {
            inverted_index
                .entry(term)
                .or_insert_with(PostingList::new)
                .add_document(doc_id, positions);
        }

        // 更新文档统计
        let mut doc_stats = self.doc_stats.write();
        doc_stats.total_docs += 1;
        doc_stats.doc_lengths.insert(doc_id, tokens.len());

        debug!(
            "Indexed document {} with {} unique terms",
            doc_id,
            term_count
        );

        Ok(())
    }

    /// 删除文档
    pub fn delete_document(&self, doc_id: &ObjectId) -> StorageResult<()> {
        let mut inverted_index = self.inverted_index.write();

        // 从所有倒排列表中删除文档
        inverted_index.retain(|_, posting_list| {
            posting_list.remove_document(doc_id);
            !posting_list.doc_ids.is_empty() // 删除空的倒排列表
        });

        // 更新文档统计
        let mut doc_stats = self.doc_stats.write();
        if doc_stats.doc_lengths.remove(doc_id).is_some() {
            doc_stats.total_docs = doc_stats.total_docs.saturating_sub(1);
        }

        Ok(())
    }

    /// 搜索文档
    ///
    /// # Arguments
    /// * `query` - 查询字符串
    ///
    /// # Returns
    /// 匹配的文档 ID 列表(按相关性排序)
    pub fn search(&self, query: &str) -> StorageResult<Vec<(ObjectId, f64)>> {
        let tokens = self.tokenize(query);

        if tokens.is_empty() {
            return Ok(Vec::new());
        }

        // 查找所有匹配的文档
        let mut doc_scores: HashMap<ObjectId, f64> = HashMap::new();
        let inverted_index = self.inverted_index.read();
        let doc_stats = self.doc_stats.read();

        for token in tokens {
            if let Some(posting_list) = inverted_index.get(&token) {
                let idf = self.calculate_idf(posting_list.doc_ids.len(), doc_stats.total_docs);

                for (i, doc_id) in posting_list.doc_ids.iter().enumerate() {
                    let tf = posting_list.frequencies[i] as f64;
                    let doc_length = doc_stats.doc_lengths.get(doc_id).copied().unwrap_or(1);

                    // TF-IDF 评分
                    let score = (tf / doc_length as f64) * idf;
                    *doc_scores.entry(*doc_id).or_default() += score;
                }
            }
        }

        // 按评分排序
        let mut results: Vec<_> = doc_scores.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        Ok(results)
    }

    /// 短语搜索
    ///
    /// # Arguments
    /// * `phrase` - 短语查询
    ///
    /// # Returns
    /// 包含完整短语的文档 ID 列表
    pub fn search_phrase(&self, phrase: &str) -> StorageResult<Vec<ObjectId>> {
        let tokens = self.tokenize(phrase);

        if tokens.is_empty() {
            return Ok(Vec::new());
        }

        if tokens.len() == 1 {
            // 单词查询
            let inverted_index = self.inverted_index.read();
            if let Some(posting_list) = inverted_index.get(&tokens[0]) {
                return Ok(posting_list.doc_ids.clone());
            } else {
                return Ok(Vec::new());
            }
        }

        // 短语查询: 查找连续的词序列
        let inverted_index = self.inverted_index.read();

        // 获取第一个词的倒排列表作为候选
        let first_posting = match inverted_index.get(&tokens[0]) {
            Some(p) => p,
            None => return Ok(Vec::new()),
        };

        let mut results = Vec::new();

        // 遍历候选文档
        for (doc_idx, doc_id) in first_posting.doc_ids.iter().enumerate() {
            let first_positions = &first_posting.positions[doc_idx];

            // 检查每个起始位置
            for &start_pos in first_positions {
                let mut is_phrase_match = true;

                // 检查后续词是否连续
                for (i, token) in tokens.iter().enumerate().skip(1) {
                    let expected_pos = start_pos + i;

                    if let Some(posting_list) = inverted_index.get(token) {
                        if let Some(doc_pos) = posting_list.doc_ids.iter().position(|id| id == doc_id) {
                            if !posting_list.positions[doc_pos].contains(&expected_pos) {
                                is_phrase_match = false;
                                break;
                            }
                        } else {
                            is_phrase_match = false;
                            break;
                        }
                    } else {
                        is_phrase_match = false;
                        break;
                    }
                }

                if is_phrase_match {
                    results.push(*doc_id);
                    break; // 找到匹配,跳过此文档的其他位置
                }
            }
        }

        Ok(results)
    }

    /// 获取索引统计信息
    pub fn stats(&self) -> IndexStats {
        let inverted_index = self.inverted_index.read();
        let doc_stats = self.doc_stats.read();

        IndexStats {
            total_terms: inverted_index.len(),
            total_docs: doc_stats.total_docs,
            avg_doc_length: if doc_stats.total_docs > 0 {
                doc_stats.doc_lengths.values().sum::<usize>() as f64 / doc_stats.total_docs as f64
            } else {
                0.0
            },
        }
    }

    // ========== 内部辅助方法 ==========

    /// 分词
    fn tokenize(&self, text: &str) -> Vec<String> {
        match self.definition.tokenizer {
            TokenizerType::Simple => self.tokenize_simple(text),
            TokenizerType::ChineseNGram => self.tokenize_chinese_ngram(text),
            TokenizerType::Mixed => self.tokenize_mixed(text),
        }
    }

    /// 简单分词器(空格分词)
    fn tokenize_simple(&self, text: &str) -> Vec<String> {
        text.split_whitespace()
            .map(|s| s.to_lowercase())
            .filter(|s| {
                s.len() >= self.definition.min_token_length
                    && s.len() <= self.definition.max_token_length
            })
            .collect()
    }

    /// 中文 N-gram 分词器
    fn tokenize_chinese_ngram(&self, text: &str) -> Vec<String> {
        let chars: Vec<char> = text.chars().collect();
        let mut tokens = Vec::new();

        // Bi-gram (2-gram)
        for i in 0..chars.len().saturating_sub(1) {
            let token: String = chars[i..i + 2].iter().collect();
            if token.chars().all(|c| c.is_alphanumeric() || c > '\u{4E00}') {
                tokens.push(token.to_lowercase());
            }
        }

        // 单字符
        for ch in chars {
            if ch.is_alphanumeric() || ch > '\u{4E00}' {
                tokens.push(ch.to_lowercase().to_string());
            }
        }

        tokens
    }

    /// 混合分词器(中英文混合)
    fn tokenize_mixed(&self, text: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current_word = String::new();
        let mut is_chinese_mode = false;

        for ch in text.chars() {
            let is_chinese = ch > '\u{4E00}' && ch < '\u{9FFF}';

            if is_chinese {
                // 保存当前英文词
                if !current_word.is_empty() && !is_chinese_mode {
                    tokens.push(current_word.to_lowercase());
                    current_word.clear();
                }

                // 中文字符
                if !current_word.is_empty() {
                    // Bi-gram
                    tokens.push(format!("{}{}", current_word, ch).to_lowercase());
                }
                tokens.push(ch.to_lowercase().to_string());
                current_word = ch.to_string();
                is_chinese_mode = true;
            } else if ch.is_alphanumeric() {
                if is_chinese_mode {
                    current_word.clear();
                    is_chinese_mode = false;
                }
                current_word.push(ch);
            } else {
                // 分隔符
                if !current_word.is_empty() {
                    tokens.push(current_word.to_lowercase());
                    current_word.clear();
                }
                is_chinese_mode = false;
            }
        }

        if !current_word.is_empty() {
            tokens.push(current_word.to_lowercase());
        }

        tokens
            .into_iter()
            .filter(|s| {
                s.len() >= self.definition.min_token_length
                    && s.len() <= self.definition.max_token_length
            })
            .collect()
    }

    /// 计算 IDF (Inverse Document Frequency)
    fn calculate_idf(&self, doc_freq: usize, total_docs: u64) -> f64 {
        if doc_freq == 0 || total_docs == 0 {
            return 0.0;
        }
        ((total_docs as f64) / (doc_freq as f64)).ln() + 1.0
    }
}

/// 索引统计信息
#[derive(Debug, Clone)]
pub struct IndexStats {
    /// 总词项数
    pub total_terms: usize,
    /// 总文档数
    pub total_docs: u64,
    /// 平均文档长度
    pub avg_doc_length: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_tokenizer() {
        let definition = FullTextIndexDefinition {
            name: "test".to_string(),
            collection: "docs".to_string(),
            fields: vec!["content".to_string()],
            tokenizer: TokenizerType::Simple,
            min_token_length: 1,
            max_token_length: 100,
            store_positions: true,
        };

        let db = Arc::new(rocksdb::DB::open_default(tempfile::tempdir().unwrap().path()).unwrap());
        let index = FullTextIndex::new(definition, db);

        let tokens = index.tokenize("Hello World Rust Database");
        assert_eq!(tokens, vec!["hello", "world", "rust", "database"]);
    }

    #[test]
    fn test_chinese_ngram_tokenizer() {
        let definition = FullTextIndexDefinition {
            name: "test".to_string(),
            collection: "docs".to_string(),
            fields: vec!["content".to_string()],
            tokenizer: TokenizerType::ChineseNGram,
            min_token_length: 1,
            max_token_length: 100,
            store_positions: true,
        };

        let db = Arc::new(rocksdb::DB::open_default(tempfile::tempdir().unwrap().path()).unwrap());
        let index = FullTextIndex::new(definition, db);

        let tokens = index.tokenize("数据库");
        assert!(tokens.contains(&"数据".to_string()));
        assert!(tokens.contains(&"据库".to_string()));
    }

    #[test]
    fn test_index_and_search() {
        let definition = FullTextIndexDefinition {
            name: "test".to_string(),
            collection: "docs".to_string(),
            fields: vec!["content".to_string()],
            tokenizer: TokenizerType::Simple,
            min_token_length: 1,
            max_token_length: 100,
            store_positions: true,
        };

        let db = Arc::new(rocksdb::DB::open_default(tempfile::tempdir().unwrap().path()).unwrap());
        let index = FullTextIndex::new(definition, db);

        let doc1 = ObjectId::new();
        let doc2 = ObjectId::new();

        index.index_document(doc1, "Rust is a systems programming language").unwrap();
        index.index_document(doc2, "Database systems are complex").unwrap();

        // 搜索 "systems"
        let results = index.search("systems").unwrap();
        assert_eq!(results.len(), 2);

        // 搜索 "rust"
        let results = index.search("rust").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, doc1);
    }

    #[test]
    fn test_phrase_search() {
        let definition = FullTextIndexDefinition {
            name: "test".to_string(),
            collection: "docs".to_string(),
            fields: vec!["content".to_string()],
            tokenizer: TokenizerType::Simple,
            min_token_length: 1,
            max_token_length: 100,
            store_positions: true,
        };

        let db = Arc::new(rocksdb::DB::open_default(tempfile::tempdir().unwrap().path()).unwrap());
        let index = FullTextIndex::new(definition, db);

        let doc1 = ObjectId::new();
        let doc2 = ObjectId::new();

        index.index_document(doc1, "rust programming language").unwrap();
        index.index_document(doc2, "programming with rust").unwrap();

        // 短语搜索 "rust programming"
        let results = index.search_phrase("rust programming").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], doc1);

        // 短语搜索 "programming language"
        let results = index.search_phrase("programming language").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], doc1);
    }
}
