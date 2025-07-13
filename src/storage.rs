// Copyright (c) 2025 Chetan Conikee <conikee@gmail.com>
// Licensed under the MIT License

//! Persistent storage layer using redb for Merkle tree nodes and metadata.
//!
//! This module provides a high-performance embedded database for storing
//! Merkle tree nodes, session metadata, and search indices.

use crate::error::{SniffError, Result};
use crate::hash::Blake3Hash;
use crate::tree::MerkleNode;
use crate::types::SessionId;
use redb::{Database, ReadableTable, ReadableTableMetadata, TableDefinition};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tracing::{debug, info};

/// Table definitions for the redb database.

/// Main table storing Merkle tree nodes keyed by their hash.
const NODES_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("nodes");

/// Index mapping session IDs to their root node hashes.
const SESSION_INDEX: TableDefinition<&str, &[u8]> = TableDefinition::new("session_index");

/// Index mapping project names to their root node hashes.
const PROJECT_INDEX: TableDefinition<&str, &[u8]> = TableDefinition::new("project_index");

/// Metadata table storing database statistics and configuration.
const METADATA_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("metadata");

/// Search index for full-text search capabilities.
const SEARCH_INDEX: TableDefinition<&str, &[u8]> = TableDefinition::new("search_index");

/// Parent-child relationship index for tree traversal.
const PARENT_CHILD_INDEX: TableDefinition<&[u8], &[u8]> = TableDefinition::new("parent_child");

/// Storage configuration options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Path to the database file.
    pub db_path: PathBuf,
    /// Enable compression for stored data.
    pub enable_compression: bool,
    /// Maximum cache size in bytes.
    pub cache_size_bytes: usize,
    /// Enable full-text search indexing.
    pub enable_search_index: bool,
    /// Compact database on startup.
    pub compact_on_startup: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from("sniff.redb"),
            enable_compression: true,
            cache_size_bytes: 64 * 1024 * 1024, // 64MB
            enable_search_index: true,
            compact_on_startup: false,
        }
    }
}

/// Database statistics and metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    /// Total number of nodes in the database.
    pub total_nodes: usize,
    /// Total number of sessions indexed.
    pub total_sessions: usize,
    /// Total number of projects indexed.
    pub total_projects: usize,
    /// Database file size in bytes.
    pub file_size_bytes: u64,
    /// Last compaction timestamp.
    pub last_compaction: Option<chrono::DateTime<chrono::Utc>>,
    /// Database schema version.
    pub schema_version: u32,
}

/// High-level storage interface for Merkle tree operations.
pub struct TreeStorage {
    /// The redb database instance.
    db: Database,
    /// Storage configuration.
    config: StorageConfig,
    /// In-memory cache for frequently accessed nodes.
    node_cache: HashMap<Blake3Hash, MerkleNode>,
    /// Cache hit/miss statistics.
    cache_stats: CacheStats,
}

/// Cache performance statistics.
#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    /// Number of cache hits.
    pub hits: u64,
    /// Number of cache misses.
    pub misses: u64,
    /// Number of evictions.
    pub evictions: u64,
}

impl CacheStats {
    /// Returns the cache hit ratio.
    #[must_use]
    pub fn hit_ratio(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }
}

/// Extracts a snippet around a search match for display.
fn extract_snippet(text: &str, query: &str, max_length: usize) -> String {
    let text_lower = text.to_lowercase();
    let query_lower = query.to_lowercase();
    
    if let Some(match_pos) = text_lower.find(&query_lower) {
        let start = match_pos.saturating_sub(max_length / 2);
        let end = (match_pos + query.len() + max_length / 2).min(text.len());
        
        // Find character boundaries to avoid UTF-8 issues
        let char_start = text.char_indices()
            .find(|(idx, _)| *idx >= start)
            .map(|(idx, _)| idx)
            .unwrap_or(0);
        let char_end = text.char_indices()
            .find(|(idx, _)| *idx >= end)
            .map(|(idx, _)| idx)
            .unwrap_or(text.len());
        
        let mut snippet = text[char_start..char_end].to_string();
        
        // Add ellipsis if we truncated
        if char_start > 0 {
            snippet = format!("...{}", snippet);
        }
        if char_end < text.len() {
            snippet = format!("{}...", snippet);
        }
        
        snippet
    } else {
        // Fallback: just take the beginning of the text
        if text.chars().count() > max_length {
            let truncated: String = text.chars().take(max_length).collect();
            format!("{}...", truncated)
        } else {
            text.to_string()
        }
    }
}

impl TreeStorage {
    /// Opens or creates a new database with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the database cannot be opened or created.
    pub fn open(config: StorageConfig) -> Result<Self> {
        info!("Opening TreeStorage at: {}", config.db_path.display());

        // Ensure parent directory exists
        if let Some(parent) = config.db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                SniffError::storage_error(format!(
                    "Failed to create database directory: {e}"
                ))
            })?;
        }

        let db = Database::create(&config.db_path).map_err(|e| {
            SniffError::storage_error(format!("Failed to open database: {e}"))
        })?;

        let mut storage = Self {
            db,
            config,
            node_cache: HashMap::new(),
            cache_stats: CacheStats::default(),
        };

        // Initialize database schema
        storage.initialize_schema()?;

        // Compact database if requested
        if storage.config.compact_on_startup {
            storage.compact()?;
        }

        debug!("TreeStorage opened successfully");
        Ok(storage)
    }

    /// Initializes the database schema and tables.
    fn initialize_schema(&self) -> Result<()> {
        let write_txn = self.db.begin_write().map_err(|e| {
            SniffError::storage_error(format!("Failed to begin write transaction: {e}"))
        })?;

        // Create all tables
        write_txn.open_table(NODES_TABLE).map_err(|e| {
            SniffError::storage_error(format!("Failed to create nodes table: {e}"))
        })?;

        write_txn.open_table(SESSION_INDEX).map_err(|e| {
            SniffError::storage_error(format!("Failed to create session index: {e}"))
        })?;

        write_txn.open_table(PROJECT_INDEX).map_err(|e| {
            SniffError::storage_error(format!("Failed to create project index: {e}"))
        })?;

        write_txn.open_table(METADATA_TABLE).map_err(|e| {
            SniffError::storage_error(format!("Failed to create metadata table: {e}"))
        })?;

        write_txn.open_table(SEARCH_INDEX).map_err(|e| {
            SniffError::storage_error(format!("Failed to create search index: {e}"))
        })?;

        write_txn.open_table(PARENT_CHILD_INDEX).map_err(|e| {
            SniffError::storage_error(format!("Failed to create parent-child index: {e}"))
        })?;

        write_txn.commit().map_err(|e| {
            SniffError::storage_error(format!("Failed to commit schema initialization: {e}"))
        })?;

        // Store schema version
        self.store_metadata("schema_version", &1u32)?;

        debug!("Database schema initialized successfully");
        Ok(())
    }

    /// Stores a Merkle tree node in the database.
    ///
    /// # Errors
    ///
    /// Returns an error if the node cannot be stored.
    pub fn store_node(&mut self, node: &MerkleNode) -> Result<()> {
        let node_bytes = if self.config.enable_compression {
            self.compress_data(&bincode::serialize(node).map_err(|e| {
                SniffError::storage_error(format!("Failed to serialize node: {e}"))
            })?)?
        } else {
            bincode::serialize(node).map_err(|e| {
                SniffError::storage_error(format!("Failed to serialize node: {e}"))
            })?
        };

        let write_txn = self.db.begin_write().map_err(|e| {
            SniffError::storage_error(format!("Failed to begin write transaction: {e}"))
        })?;

        {
            let mut table = write_txn.open_table(NODES_TABLE).map_err(|e| {
                SniffError::storage_error(format!("Failed to open nodes table: {e}"))
            })?;

            table
                .insert(node.hash.as_slice(), node_bytes.as_slice())
                .map_err(|e| {
                    SniffError::storage_error(format!("Failed to insert node: {e}"))
                })?;
        }

        // Update parent-child relationships
        self.update_parent_child_index(&write_txn, node)?;

        write_txn.commit().map_err(|e| {
            SniffError::storage_error(format!("Failed to commit node storage: {e}"))
        })?;

        // Update cache
        self.node_cache.insert(node.hash, node.clone());
        self.evict_cache_if_needed();

        debug!("Stored node with hash: {}", node.hash);
        Ok(())
    }

    /// Retrieves a Merkle tree node by its hash.
    ///
    /// # Errors
    ///
    /// Returns an error if the node cannot be retrieved.
    pub fn get_node(&mut self, hash: &Blake3Hash) -> Result<Option<MerkleNode>> {
        // Check cache first
        if let Some(node) = self.node_cache.get(hash) {
            self.cache_stats.hits += 1;
            return Ok(Some(node.clone()));
        }

        self.cache_stats.misses += 1;

        let read_txn = self.db.begin_read().map_err(|e| {
            SniffError::storage_error(format!("Failed to begin read transaction: {e}"))
        })?;

        let table = read_txn.open_table(NODES_TABLE).map_err(|e| {
            SniffError::storage_error(format!("Failed to open nodes table: {e}"))
        })?;

        let node_bytes = match table.get(hash.as_slice()).map_err(|e| {
            SniffError::storage_error(format!("Failed to get node: {e}"))
        })? {
            Some(bytes) => bytes.value().to_vec(),
            None => return Ok(None),
        };

        let decompressed = if self.config.enable_compression {
            self.decompress_data(&node_bytes)?
        } else {
            node_bytes
        };

        let node: MerkleNode = bincode::deserialize(&decompressed).map_err(|e| {
            SniffError::storage_error(format!("Failed to deserialize node: {e}"))
        })?;

        // Update cache
        self.node_cache.insert(*hash, node.clone());
        self.evict_cache_if_needed();

        debug!("Retrieved node with hash: {}", hash);
        Ok(Some(node))
    }

    /// Stores an index mapping a session ID to its root node hash.
    ///
    /// # Errors
    ///
    /// Returns an error if the index cannot be stored.
    pub fn index_session(&self, session_id: &SessionId, root_hash: &Blake3Hash) -> Result<()> {
        let write_txn = self.db.begin_write().map_err(|e| {
            SniffError::storage_error(format!("Failed to begin write transaction: {e}"))
        })?;

        {
            let mut table = write_txn.open_table(SESSION_INDEX).map_err(|e| {
                SniffError::storage_error(format!("Failed to open session index: {e}"))
            })?;

            table
                .insert(session_id.as_str(), root_hash.as_slice())
                .map_err(|e| {
                    SniffError::storage_error(format!("Failed to index session: {e}"))
                })?;
        }

        write_txn.commit().map_err(|e| {
            SniffError::storage_error(format!("Failed to commit session index: {e}"))
        })?;

        debug!("Indexed session: {} -> {}", session_id, root_hash);
        Ok(())
    }

    /// Retrieves the root node hash for a session.
    ///
    /// # Errors
    ///
    /// Returns an error if the lookup fails.
    pub fn get_session_root(&self, session_id: &SessionId) -> Result<Option<Blake3Hash>> {
        let read_txn = self.db.begin_read().map_err(|e| {
            SniffError::storage_error(format!("Failed to begin read transaction: {e}"))
        })?;

        let table = read_txn.open_table(SESSION_INDEX).map_err(|e| {
            SniffError::storage_error(format!("Failed to open session index: {e}"))
        })?;

        match table.get(session_id.as_str()).map_err(|e| {
            SniffError::storage_error(format!("Failed to get session root: {e}"))
        })? {
            Some(hash_bytes) => {
                let mut hash_array = [0u8; 32];
                hash_array.copy_from_slice(hash_bytes.value());
                Ok(Some(Blake3Hash::new(hash_array)))
            }
            None => Ok(None),
        }
    }

    /// Lists all available session IDs.
    ///
    /// # Errors
    ///
    /// Returns an error if the listing fails.
    pub fn list_sessions(&self) -> Result<Vec<SessionId>> {
        let read_txn = self.db.begin_read().map_err(|e| {
            SniffError::storage_error(format!("Failed to begin read transaction: {e}"))
        })?;

        let table = read_txn.open_table(SESSION_INDEX).map_err(|e| {
            SniffError::storage_error(format!("Failed to open session index: {e}"))
        })?;

        let mut sessions = Vec::new();
        let mut iter = table.iter().map_err(|e| {
            SniffError::storage_error(format!("Failed to iterate sessions: {e}"))
        })?;

        while let Some(Ok((key, _))) = iter.next() {
            sessions.push(key.value().to_string());
        }

        Ok(sessions)
    }

    /// Gets database statistics.
    ///
    /// # Errors
    ///
    /// Returns an error if statistics cannot be computed.
    pub fn get_stats(&self) -> Result<DatabaseStats> {
        let read_txn = self.db.begin_read().map_err(|e| {
            SniffError::storage_error(format!("Failed to begin read transaction: {e}"))
        })?;

        // Count nodes
        let nodes_table = read_txn.open_table(NODES_TABLE).map_err(|e| {
            SniffError::storage_error(format!("Failed to open nodes table: {e}"))
        })?;
        let total_nodes = nodes_table.len().map_err(|e| {
            SniffError::storage_error(format!("Failed to count nodes: {e}"))
        })? as usize;

        // Count sessions
        let session_table = read_txn.open_table(SESSION_INDEX).map_err(|e| {
            SniffError::storage_error(format!("Failed to open session index: {e}"))
        })?;
        let total_sessions = session_table.len().map_err(|e| {
            SniffError::storage_error(format!("Failed to count sessions: {e}"))
        })? as usize;

        // Count projects
        let project_table = read_txn.open_table(PROJECT_INDEX).map_err(|e| {
            SniffError::storage_error(format!("Failed to open project index: {e}"))
        })?;
        let total_projects = project_table.len().map_err(|e| {
            SniffError::storage_error(format!("Failed to count projects: {e}"))
        })? as usize;

        // Get file size
        let file_size_bytes = std::fs::metadata(&self.config.db_path)
            .map(|m| m.len())
            .unwrap_or(0);

        Ok(DatabaseStats {
            total_nodes,
            total_sessions,
            total_projects,
            file_size_bytes,
            last_compaction: None, // TODO: Track compaction times
            schema_version: 1,
        })
    }

    /// Compacts the database to reclaim space.
    ///
    /// # Errors
    ///
    /// Returns an error if compaction fails.
    pub fn compact(&mut self) -> Result<()> {
        info!("Starting database compaction");
        
        self.db.compact().map_err(|e| {
            SniffError::storage_error(format!("Failed to compact database: {e}"))
        })?;

        // Update compaction timestamp
        let now = chrono::Utc::now();
        self.store_metadata("last_compaction", &now)?;

        info!("Database compaction completed");
        Ok(())
    }

    /// Returns cache statistics.
    #[must_use]
    pub fn cache_stats(&self) -> &CacheStats {
        &self.cache_stats
    }

    /// Searches through all indexed content for the given query.
    ///
    /// This is a basic implementation that searches through all sessions and
    /// their message content. Returns session IDs that contain matching content.
    pub fn search_content(&self, query: &str, limit: usize) -> Result<Vec<(String, Vec<String>)>> {
        if query.is_empty() {
            return Ok(Vec::new());
        }
        
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();
        
        let read_txn = self.db.begin_read().map_err(|e| {
            SniffError::storage_error(format!("Failed to begin read transaction: {e}"))
        })?;
        
        // Get all sessions
        let session_table = read_txn.open_table(SESSION_INDEX).map_err(|e| {
            SniffError::storage_error(format!("Failed to open session index: {e}"))
        })?;
        
        let node_table = read_txn.open_table(NODES_TABLE).map_err(|e| {
            SniffError::storage_error(format!("Failed to open node table: {e}"))
        })?;
        
        // Search through each session
        let mut _sessions_checked = 0;
        let mut _nodes_checked = 0;
        let mut _messages_with_content = 0;
        let mut _content_chunks_checked = 0;
        
        for item in session_table.iter().map_err(|e| SniffError::storage_error(format!("Failed to iterate sessions: {e}")))? {
            let (k, v) = item.map_err(|e| SniffError::storage_error(format!("Failed to read session entry: {e}")))?;
            let session_id = k.value().to_string();
            let hash_bytes = v.value();
            _sessions_checked += 1;
            
            if hash_bytes.len() != 32 {
                continue; // Skip invalid hashes
            }
            
            let mut hash_array = [0u8; 32];
            hash_array.copy_from_slice(hash_bytes);
            let _session_hash = crate::hash::Blake3Hash::new(hash_array);
            
            let mut matching_snippets = Vec::new();
            
            // Get the session node data directly from the table
            if let Some(session_data) = node_table.get(hash_bytes).map_err(|e| {
                SniffError::storage_error(format!("Failed to read session node: {e}"))
            })? {
                // Deserialize the session node
                if let Ok(session_node) = bincode::deserialize::<crate::tree::MerkleNode>(session_data.value()) {
                    // Search through all child nodes (messages)
                    for (_child_key, child_hash) in &session_node.children {
                        _nodes_checked += 1;
                        if let Some(message_data) = node_table.get(child_hash.as_bytes() as &[u8]).map_err(|e| {
                            SniffError::storage_error(format!("Failed to read message node: {e}"))
                        })? {
                            if let Ok(message_node) = bincode::deserialize::<crate::tree::MerkleNode>(message_data.value()) {
                                // Check if this is a message node and has content
                                if matches!(message_node.node_type, crate::tree::NodeType::Message { .. }) {
                                    if let Some(ref content_data) = message_node.content {
                                        _messages_with_content += 1;
                                        // Parse the content as a ClaudeMessage and extract text
                                        match serde_json::from_slice::<crate::types::ClaudeMessage>(content_data) {
                                            Ok(message) => {
                                                let text_content = message.extract_all_text_content();
                                                
                                                for text in text_content {
                                                    _content_chunks_checked += 1;
                                                    if text.to_lowercase().contains(&query_lower) {
                                                        // Extract a snippet around the match
                                                        let snippet = extract_snippet(&text, query, 100);
                                                        matching_snippets.push(snippet);
                                                    }
                                                }
                                            }
                                            Err(_e) => {
                                                // Skip messages that can't be deserialized
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            if !matching_snippets.is_empty() {
                results.push((session_id, matching_snippets));
                
                if results.len() >= limit {
                    break;
                }
            }
        }
        
        Ok(results)
    }

    /// Stores metadata in the database.
    fn store_metadata<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let value_bytes = bincode::serialize(value).map_err(|e| {
            SniffError::storage_error(format!("Failed to serialize metadata: {e}"))
        })?;

        let write_txn = self.db.begin_write().map_err(|e| {
            SniffError::storage_error(format!("Failed to begin write transaction: {e}"))
        })?;

        {
            let mut table = write_txn.open_table(METADATA_TABLE).map_err(|e| {
                SniffError::storage_error(format!("Failed to open metadata table: {e}"))
            })?;

            table.insert(key, value_bytes.as_slice()).map_err(|e| {
                SniffError::storage_error(format!("Failed to store metadata: {e}"))
            })?;
        }

        write_txn.commit().map_err(|e| {
            SniffError::storage_error(format!("Failed to commit metadata: {e}"))
        })?;

        Ok(())
    }

    /// Updates the parent-child relationship index.
    fn update_parent_child_index(
        &self,
        write_txn: &redb::WriteTransaction,
        node: &MerkleNode,
    ) -> Result<()> {
        let mut table = write_txn.open_table(PARENT_CHILD_INDEX).map_err(|e| {
            SniffError::storage_error(format!("Failed to open parent-child index: {e}"))
        })?;

        // Store children for this node
        let children_data = bincode::serialize(&node.children).map_err(|e| {
            SniffError::storage_error(format!("Failed to serialize children: {e}"))
        })?;

        table
            .insert(node.hash.as_slice(), children_data.as_slice())
            .map_err(|e| {
                SniffError::storage_error(format!("Failed to update parent-child index: {e}"))
            })?;

        Ok(())
    }

    /// Compresses data if compression is enabled.
    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        // For now, return data as-is. Real compression would use zstd/lz4
        Ok(data.to_vec())
    }

    /// Decompresses data if compression is enabled.
    fn decompress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        // For now, return data as-is. Real decompression would use zstd/lz4
        Ok(data.to_vec())
    }

    /// Evicts cache entries if the cache is too large.
    fn evict_cache_if_needed(&mut self) {
        const MAX_CACHE_ENTRIES: usize = 1000; // Simple LRU would be better
        
        if self.node_cache.len() > MAX_CACHE_ENTRIES {
            // Simple eviction: remove random entries
            let keys_to_remove: Vec<_> = self
                .node_cache
                .keys()
                .take(self.node_cache.len() - MAX_CACHE_ENTRIES / 2)
                .cloned()
                .collect();

            for key in keys_to_remove {
                self.node_cache.remove(&key);
                self.cache_stats.evictions += 1;
            }
        }
    }
}

/// Batch operations for efficient bulk storage.
pub struct BatchWriter<'a> {
    storage: &'a mut TreeStorage,
    nodes: Vec<MerkleNode>,
    session_indices: Vec<(SessionId, Blake3Hash)>,
    project_indices: Vec<(String, Blake3Hash)>,
}

impl<'a> BatchWriter<'a> {
    /// Creates a new batch writer.
    #[must_use]
    pub fn new(storage: &'a mut TreeStorage) -> Self {
        Self {
            storage,
            nodes: Vec::new(),
            session_indices: Vec::new(),
            project_indices: Vec::new(),
        }
    }

    /// Adds a node to the batch.
    pub fn add_node(&mut self, node: MerkleNode) {
        self.nodes.push(node);
    }

    /// Adds a session index to the batch.
    pub fn add_session_index(&mut self, session_id: SessionId, root_hash: Blake3Hash) {
        self.session_indices.push((session_id, root_hash));
    }

    /// Commits all batched operations.
    ///
    /// # Errors
    ///
    /// Returns an error if the batch cannot be committed.
    pub fn commit(self) -> Result<()> {
        let write_txn = self.storage.db.begin_write().map_err(|e| {
            SniffError::storage_error(format!("Failed to begin batch write: {e}"))
        })?;

        // Store all nodes
        {
            let mut nodes_table = write_txn.open_table(NODES_TABLE).map_err(|e| {
                SniffError::storage_error(format!("Failed to open nodes table: {e}"))
            })?;

            for node in &self.nodes {
                let node_bytes = bincode::serialize(node).map_err(|e| {
                    SniffError::storage_error(format!("Failed to serialize node: {e}"))
                })?;

                nodes_table
                    .insert(node.hash.as_slice(), node_bytes.as_slice())
                    .map_err(|e| {
                        SniffError::storage_error(format!("Failed to insert node: {e}"))
                    })?;
            }
        }

        // Store session indices
        {
            let mut session_table = write_txn.open_table(SESSION_INDEX).map_err(|e| {
                SniffError::storage_error(format!("Failed to open session index: {e}"))
            })?;

            for (session_id, root_hash) in &self.session_indices {
                session_table
                    .insert(session_id.as_str(), root_hash.as_slice())
                    .map_err(|e| {
                        SniffError::storage_error(format!("Failed to index session: {e}"))
                    })?;
            }
        }

        write_txn.commit().map_err(|e| {
            SniffError::storage_error(format!("Failed to commit batch: {e}"))
        })?;

        // Update cache with new nodes
        for node in self.nodes {
            self.storage.node_cache.insert(node.hash, node);
        }
        self.storage.evict_cache_if_needed();

        info!("Committed batch with {} operations", 
              self.session_indices.len() + self.project_indices.len());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tree::NodeType;
    use tempfile::TempDir;

    fn create_test_storage() -> (TreeStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");
        
        let config = StorageConfig {
            db_path,
            ..Default::default()
        };
        
        let storage = TreeStorage::open(config).unwrap();
        (storage, temp_dir)
    }

    fn create_test_node() -> MerkleNode {
        let metadata = crate::tree::NodeMetadata::default();
        MerkleNode::new(
            NodeType::Root,
            metadata,
            std::collections::BTreeMap::new(),
            None,
            None,
        ).unwrap()
    }

    fn create_test_node_with_content(content: &[u8]) -> MerkleNode {
        let metadata = crate::tree::NodeMetadata::default();
        MerkleNode::new(
            NodeType::Root,
            metadata,
            std::collections::BTreeMap::new(),
            None,
            Some(content.to_vec()),
        ).unwrap()
    }

    #[test]
    fn test_storage_creation() {
        let (_storage, _temp_dir) = create_test_storage();
        // Storage creation in create_test_storage should succeed
    }

    #[test]
    fn test_node_storage_retrieval() {
        let (mut storage, _temp_dir) = create_test_storage();
        let node = create_test_node();
        let hash = node.hash;

        storage.store_node(&node).unwrap();
        let retrieved = storage.get_node(&hash).unwrap();
        
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().hash, hash);
    }

    #[test]
    fn test_session_indexing() {
        let (mut storage, _temp_dir) = create_test_storage();
        let node = create_test_node();
        let session_id = "test-session".to_string();
        
        storage.store_node(&node).unwrap();
        storage.index_session(&session_id, &node.hash).unwrap();
        
        let root_hash = storage.get_session_root(&session_id).unwrap();
        assert_eq!(root_hash, Some(node.hash));
    }

    #[test]
    fn test_cache_functionality() {
        let (mut storage, _temp_dir) = create_test_storage();
        let node = create_test_node();
        let hash = node.hash;

        // First retrieval should miss cache
        storage.store_node(&node).unwrap();
        let stats_before = storage.cache_stats().clone();
        
        // Second retrieval should hit cache
        storage.get_node(&hash).unwrap();
        storage.get_node(&hash).unwrap();
        
        let stats_after = storage.cache_stats();
        assert!(stats_after.hits > stats_before.hits);
    }

    #[test]
    fn test_database_stats() {
        let (mut storage, _temp_dir) = create_test_storage();
        let node = create_test_node();
        
        storage.store_node(&node).unwrap();
        storage.index_session(&"test-session".to_string(), &node.hash).unwrap();
        
        let stats = storage.get_stats().unwrap();
        assert_eq!(stats.total_nodes, 1);
        assert_eq!(stats.total_sessions, 1);
    }

    #[test]
    fn test_batch_operations() {
        let (mut storage, _temp_dir) = create_test_storage();
        let node1 = create_test_node_with_content(b"content1");
        let node2 = create_test_node_with_content(b"content2");
        
        let mut batch = BatchWriter::new(&mut storage);
        batch.add_node(node1.clone());
        batch.add_node(node2.clone());
        batch.add_session_index("session1".to_string(), node1.hash);
        batch.add_session_index("session2".to_string(), node2.hash);
        
        batch.commit().unwrap();
        
        let stats = storage.get_stats().unwrap();
        assert_eq!(stats.total_nodes, 2);
        assert_eq!(stats.total_sessions, 2);
    }
}