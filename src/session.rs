// Copyright (c) 2025 Chetan Conikee <conikee@gmail.com>
// Licensed under the MIT License

//! Session processing and tree building integration.
//!
//! This module provides high-level interfaces for processing Claude Code
//! session data, building Merkle trees, and storing results in the database.

use crate::error::{SniffError, Result};
use crate::hash::Blake3Hash;
use crate::jsonl::JsonlParser;
use crate::operations::{Operation, OperationExtractor};
use crate::storage::TreeStorage;
use crate::tree::{MerkleNode, TreeBuilder};
use crate::types::{ClaudeMessage, SessionId};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Configuration for session processing.
#[derive(Debug, Clone)]
pub struct SessionProcessorConfig {
    /// Whether to include message content in tree nodes.
    pub include_content: bool,
    /// Whether to extract and analyze operations.
    pub extract_operations: bool,
    /// Whether to compute operation dependencies.
    pub compute_dependencies: bool,
    /// Maximum number of messages to process per session.
    pub max_messages: Option<usize>,
    /// Whether to validate tree integrity after building.
    pub validate_tree: bool,
}

impl Default for SessionProcessorConfig {
    fn default() -> Self {
        Self {
            include_content: true,
            extract_operations: true,
            compute_dependencies: true,
            max_messages: None,
            validate_tree: true,
        }
    }
}

/// Statistics about processed session data.
#[derive(Debug, Clone)]
pub struct ProcessingStats {
    /// Number of sessions processed.
    pub sessions_processed: usize,
    /// Total number of messages processed.
    pub total_messages: usize,
    /// Total number of operations extracted.
    pub total_operations: usize,
    /// Number of tree nodes created.
    pub tree_nodes_created: usize,
    /// Total processing time in milliseconds.
    pub processing_time_ms: u64,
    /// Number of errors encountered.
    pub error_count: usize,
}

impl Default for ProcessingStats {
    fn default() -> Self {
        Self {
            sessions_processed: 0,
            total_messages: 0,
            total_operations: 0,
            tree_nodes_created: 0,
            processing_time_ms: 0,
            error_count: 0,
        }
    }
}

/// High-level processor for Claude Code sessions.
pub struct SessionProcessor {
    /// Configuration for processing.
    config: SessionProcessorConfig,
    /// JSONL parser for reading session files.
    jsonl_parser: JsonlParser,
    /// Operation extractor for analyzing tool usage.
    operation_extractor: OperationExtractor,
    /// Tree builder for creating Merkle trees.
    tree_builder: TreeBuilder,
    /// Storage interface for persisting data.
    storage: TreeStorage,
    /// Processing statistics.
    stats: ProcessingStats,
}

impl SessionProcessor {
    /// Creates a new session processor with the given storage and configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the processor cannot be initialized.
    pub fn new(storage: TreeStorage, config: SessionProcessorConfig) -> Result<Self> {
        let jsonl_parser = JsonlParser::new();
        let operation_extractor = OperationExtractor::new();
        let tree_builder = TreeBuilder::with_config(crate::tree::TreeBuilderConfig {
            include_content: config.include_content,
            compute_dependencies: config.compute_dependencies,
            max_depth: 0,
            validate_hashes: config.validate_tree,
        });

        Ok(Self {
            config,
            jsonl_parser,
            operation_extractor,
            tree_builder,
            storage,
            stats: ProcessingStats::default(),
        })
    }

    /// Processes a single session file and builds its Merkle tree.
    ///
    /// # Errors
    ///
    /// Returns an error if the session cannot be processed.
    pub fn process_session_file(&mut self, file_path: &Path) -> Result<Blake3Hash> {
        let start_time = std::time::Instant::now();
        info!("Processing session file: {}", file_path.display());

        // Extract session ID from filename
        let session_id = self.extract_session_id(file_path)?;
        
        // Parse JSONL file to get messages
        let messages = self.parse_session_file(file_path)?;
        
        if messages.is_empty() {
            warn!("No messages found in session file: {}", file_path.display());
            return Err(SniffError::invalid_session(
                "Session file contains no messages",
            ));
        }

        // Extract operations if enabled
        let operations = if self.config.extract_operations {
            self.extract_operations_from_messages(&messages)?
        } else {
            Vec::new()
        };

        // Build session tree
        let session_tree = self.build_session_tree(&session_id, &messages, &operations)?;
        let root_hash = session_tree.hash;

        // Store tree and index session
        self.store_session_tree(session_tree, &session_id)?;

        // Update statistics
        let processing_time = start_time.elapsed();
        self.stats.sessions_processed += 1;
        self.stats.total_messages += messages.len();
        self.stats.total_operations += operations.len();
        self.stats.processing_time_ms += processing_time.as_millis() as u64;

        info!(
            "Processed session {} with {} messages, {} operations in {}ms",
            session_id,
            messages.len(),
            operations.len(),
            processing_time.as_millis()
        );

        Ok(root_hash)
    }

    /// Processes multiple session files in a project directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the project cannot be processed.
    pub fn process_project_directory(&mut self, project_path: &Path) -> Result<Blake3Hash> {
        info!("Processing project directory: {}", project_path.display());

        // Discover session files
        let session_files = self.discover_session_files(project_path)?;
        
        if session_files.is_empty() {
            warn!("No session files found in project: {}", project_path.display());
            return Err(SniffError::project_discovery(
                project_path,
                "No session files found",
            ));
        }

        // Process each session file and collect root hashes
        let mut session_root_hashes = Vec::new();

        for session_file in &session_files {
            match self.process_session_file(session_file) {
                Ok(root_hash) => {
                    session_root_hashes.push(root_hash);
                }
                Err(e) => {
                    warn!("Failed to process session file {}: {}", session_file.display(), e);
                    self.stats.error_count += 1;
                }
            }
        }

        if session_root_hashes.is_empty() {
            return Err(SniffError::project_discovery(
                project_path,
                "No valid sessions were processed",
            ));
        }

        // Retrieve session nodes for project tree building
        let mut session_nodes = Vec::new();
        for root_hash in &session_root_hashes {
            if let Ok(Some(session_node)) = self.storage.get_node(root_hash) {
                session_nodes.push(session_node);
            }
        }

        // Build project tree
        let project_name = self.extract_project_name(project_path)?;
        let project_tree = self.tree_builder.build_project_tree(
            project_name,
            project_path.to_path_buf(),
            session_nodes,
        )?;
        let project_hash = project_tree.hash;

        // Store project tree
        self.storage.store_node(&project_tree)?;

        info!(
            "Built project tree for {} with {} sessions",
            project_path.display(),
            session_files.len()
        );

        Ok(project_hash)
    }

    /// Returns current processing statistics.
    #[must_use]
    pub fn stats(&self) -> &ProcessingStats {
        &self.stats
    }

    /// Resets processing statistics.
    pub fn reset_stats(&mut self) {
        self.stats = ProcessingStats::default();
    }

    /// Extracts the session ID from a file path.
    fn extract_session_id(&self, file_path: &Path) -> Result<SessionId> {
        file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .map(String::from)
            .ok_or_else(|| {
                SniffError::invalid_session(format!(
                    "Could not extract session ID from path: {}",
                    file_path.display()
                ))
            })
    }

    /// Parses a session file to extract messages.
    fn parse_session_file(&mut self, file_path: &Path) -> Result<Vec<ClaudeMessage>> {
        let parse_result = self.jsonl_parser.parse_file(file_path)?;
        let mut messages = parse_result.messages;
        
        // Apply message limit if configured
        if let Some(max_messages) = self.config.max_messages {
            if messages.len() > max_messages {
                debug!("Limiting messages from {} to {}", messages.len(), max_messages);
                messages.truncate(max_messages);
            }
        }

        Ok(messages)
    }

    /// Extracts operations from a list of messages.
    fn extract_operations_from_messages(&mut self, messages: &[ClaudeMessage]) -> Result<Vec<Operation>> {
        self.operation_extractor.extract_operations(messages)
    }

    /// Builds a Merkle tree for a session.
    fn build_session_tree(
        &mut self,
        session_id: &SessionId,
        messages: &[ClaudeMessage],
        operations: &[Operation],
    ) -> Result<MerkleNode> {
        let tree = self.tree_builder.build_session_tree(
            session_id.clone(),
            messages,
            operations,
        )?;

        self.stats.tree_nodes_created += 1 + tree.child_count();
        Ok(tree)
    }

    /// Stores a session tree and indexes it.
    fn store_session_tree(&mut self, tree: MerkleNode, session_id: &SessionId) -> Result<()> {
        let root_hash = tree.hash;
        
        // Store the tree node
        self.storage.store_node(&tree)?;
        
        // Store all cached nodes from the tree builder
        for (hash, node) in self.tree_builder.cached_nodes() {
            if *hash != root_hash {
                self.storage.store_node(node)?;
            }
        }
        
        // Index the session
        self.storage.index_session(session_id, &root_hash)?;
        
        debug!("Stored session tree for session: {}", session_id);
        Ok(())
    }

    /// Discovers session files in a project directory.
    fn discover_session_files(&self, project_path: &Path) -> Result<Vec<PathBuf>> {
        use walkdir::WalkDir;

        let mut session_files = Vec::new();
        
        for entry in WalkDir::new(project_path)
            .follow_links(false)
            .max_depth(2) // Limit search depth
        {
            let entry = entry.map_err(|e| {
                SniffError::project_discovery(
                    project_path,
                    format!("Error walking directory: {e}"),
                )
            })?;

            if entry.file_type().is_file() {
                let path = entry.path();
                if self.is_session_file(path) {
                    session_files.push(path.to_path_buf());
                }
            }
        }

        // Sort by filename for consistent processing order
        session_files.sort();
        Ok(session_files)
    }

    /// Checks if a file path represents a session file.
    fn is_session_file(&self, path: &Path) -> bool {
        path.extension().map_or(false, |ext| ext == "jsonl") &&
        path.file_stem()
            .and_then(|name| name.to_str())
            .map_or(false, |name| {
                // Session files typically have UUID-like names
                name.len() > 10 && name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
            })
    }

    /// Extracts the project name from a directory path.
    fn extract_project_name(&self, project_path: &Path) -> Result<String> {
        project_path
            .file_name()
            .and_then(|s| s.to_str())
            .map(String::from)
            .ok_or_else(|| {
                SniffError::project_discovery(
                    project_path,
                    "Could not extract project name from path",
                )
            })
    }
}

/// Utilities for session processing.
pub mod utils {
    use super::*;

    /// Validates a session file format.
    ///
    /// # Errors
    ///
    /// Returns an error if the file is not a valid session file.
    pub fn validate_session_file(file_path: &Path) -> Result<()> {
        if !file_path.exists() {
            return Err(SniffError::invalid_session(
                format!("Session file does not exist: {}", file_path.display()),
            ));
        }

        if !file_path.is_file() {
            return Err(SniffError::invalid_session(
                format!("Path is not a file: {}", file_path.display()),
            ));
        }

        if file_path.extension().map_or(true, |ext| ext != "jsonl") {
            return Err(SniffError::invalid_session(
                "Session file must have .jsonl extension",
            ));
        }

        Ok(())
    }

    /// Estimates the processing time for a project based on file sizes.
    ///
    /// # Errors
    ///
    /// Returns an error if the estimation cannot be computed.
    pub fn estimate_processing_time(project_path: &Path) -> Result<std::time::Duration> {
        use walkdir::WalkDir;

        let mut total_size = 0u64;
        let mut file_count = 0usize;

        for entry in WalkDir::new(project_path).follow_links(false) {
            let entry = entry.map_err(|e| {
                SniffError::project_discovery(
                    project_path,
                    format!("Error walking directory: {e}"),
                )
            })?;

            if entry.file_type().is_file() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "jsonl") {
                    if let Ok(metadata) = std::fs::metadata(path) {
                        total_size += metadata.len();
                        file_count += 1;
                    }
                }
            }
        }

        // Rough estimation: ~1MB per second processing speed
        let estimated_seconds = total_size / 1_000_000 + file_count as u64;
        Ok(std::time::Duration::from_secs(estimated_seconds.max(1)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::StorageConfig;
    use tempfile::TempDir;

    fn create_test_processor() -> (SessionProcessor, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.redb");
        
        let storage_config = StorageConfig {
            db_path,
            ..Default::default()
        };
        
        let storage = TreeStorage::open(storage_config).unwrap();
        let config = SessionProcessorConfig::default();
        let processor = SessionProcessor::new(storage, config).unwrap();
        
        (processor, temp_dir)
    }

    fn create_test_session_file(dir: &Path, content: &str) -> PathBuf {
        let session_file = dir.join("test-session.jsonl");
        std::fs::write(&session_file, content).unwrap();
        session_file
    }

    #[test]
    fn test_processor_creation() {
        let (_processor, _temp_dir) = create_test_processor();
        // Processor creation should succeed
    }

    #[test]
    fn test_extract_session_id() {
        let (processor, _temp_dir) = create_test_processor();
        let path = Path::new("/path/to/session-123.jsonl");
        let session_id = processor.extract_session_id(path).unwrap();
        assert_eq!(session_id, "session-123");
    }

    #[test]
    fn test_is_session_file() {
        let (processor, _temp_dir) = create_test_processor();
        
        assert!(processor.is_session_file(Path::new("session-123.jsonl")));
        assert!(!processor.is_session_file(Path::new("not-session.txt")));
        assert!(!processor.is_session_file(Path::new("too-short.jsonl")));
    }

    #[test]
    fn test_extract_project_name() {
        let (processor, _temp_dir) = create_test_processor();
        let path = Path::new("/path/to/my-project");
        let name = processor.extract_project_name(path).unwrap();
        assert_eq!(name, "my-project");
    }

    #[test]
    fn test_validate_session_file() {
        let temp_dir = TempDir::new().unwrap();
        let valid_file = create_test_session_file(temp_dir.path(), "{}");
        
        assert!(utils::validate_session_file(&valid_file).is_ok());
        assert!(utils::validate_session_file(Path::new("/nonexistent.jsonl")).is_err());
    }

    #[test]
    fn test_processing_stats() {
        let (mut processor, _temp_dir) = create_test_processor();
        
        let initial_stats = processor.stats();
        assert_eq!(initial_stats.sessions_processed, 0);
        
        processor.reset_stats();
        assert_eq!(processor.stats().sessions_processed, 0);
    }
}