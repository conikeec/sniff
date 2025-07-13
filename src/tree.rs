// Copyright (c) 2025 Chetan Conikee <conikee@gmail.com>
// Licensed under the MIT License

//! Merkle tree implementation for Claude Code session data.
//!
//! This module provides a hierarchical Merkle tree structure for organizing
//! and verifying Claude Code session data with cryptographic integrity.

use crate::error::{SniffError, Result};
use crate::hash::Blake3Hash;
use crate::operations::Operation;
use crate::types::{ClaudeMessage, MessageUuid, SessionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use tracing::{debug, info};

/// Represents the type of a node in the Merkle tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    /// Root node containing all projects.
    Root,
    /// Project node containing sessions.
    Project {
        /// Project name extracted from directory.
        name: String,
        /// Current working directory path.
        cwd: PathBuf,
    },
    /// Session node containing messages and operations.
    Session {
        /// Session identifier.
        session_id: SessionId,
        /// Session start time.
        start_time: DateTime<Utc>,
        /// Session end time (latest message timestamp).
        end_time: Option<DateTime<Utc>>,
    },
    /// Message node representing a single conversation entry.
    Message {
        /// Message UUID.
        message_uuid: MessageUuid,
        /// Message timestamp.
        timestamp: DateTime<Utc>,
        /// Role of the message sender.
        role: String,
    },
    /// Operation node representing a tool use operation.
    Operation {
        /// Tool use identifier.
        tool_use_id: String,
        /// Name of the tool used.
        tool_name: String,
        /// Operation timestamp.
        timestamp: DateTime<Utc>,
    },
}

/// Metadata associated with tree nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    /// Creation timestamp of the node.
    pub created_at: DateTime<Utc>,
    /// Last modification timestamp.
    pub updated_at: DateTime<Utc>,
    /// Total number of messages in this subtree.
    pub message_count: usize,
    /// Total number of operations in this subtree.
    pub operation_count: usize,
    /// Total size in bytes of content in this subtree.
    pub content_size: u64,
    /// Custom metadata fields.
    pub custom_fields: HashMap<String, serde_json::Value>,
}

impl Default for NodeMetadata {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            updated_at: now,
            message_count: 0,
            operation_count: 0,
            content_size: 0,
            custom_fields: HashMap::new(),
        }
    }
}

/// A node in the Merkle tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleNode {
    /// Unique hash identifying this node.
    pub hash: Blake3Hash,
    /// Type and type-specific data of this node.
    pub node_type: NodeType,
    /// Metadata associated with this node.
    pub metadata: NodeMetadata,
    /// Hashes of child nodes, ordered by key.
    pub children: BTreeMap<String, Blake3Hash>,
    /// Hash of the parent node, if any.
    pub parent: Option<Blake3Hash>,
    /// Raw content data for leaf nodes.
    pub content: Option<Vec<u8>>,
}

impl MerkleNode {
    /// Creates a new Merkle node.
    ///
    /// # Errors
    ///
    /// Returns an error if the node hash cannot be computed.
    pub fn new(
        node_type: NodeType,
        metadata: NodeMetadata,
        children: BTreeMap<String, Blake3Hash>,
        parent: Option<Blake3Hash>,
        content: Option<Vec<u8>>,
    ) -> Result<Self> {
        let mut node = Self {
            hash: Blake3Hash::null(), // Temporary hash
            node_type,
            metadata,
            children,
            parent,
            content,
        };
        
        // Compute the actual hash
        node.hash = node.compute_hash()?;
        Ok(node)
    }

    /// Computes the hash for this node based on its content and children.
    ///
    /// # Errors
    ///
    /// Returns an error if the hash cannot be computed.
    pub fn compute_hash(&self) -> Result<Blake3Hash> {
        let mut hasher = blake3::Hasher::new();
        
        // Add domain separator
        hasher.update(b"MERKLE_NODE:");
        
        // Hash node type
        let node_type_bytes = serde_json::to_vec(&self.node_type)
            .map_err(|e| SniffError::hash_computation(format!("Failed to serialize node type: {e}")))?;
        hasher.update(&node_type_bytes);
        
        // Hash children in sorted order
        hasher.update(b"CHILDREN:");
        hasher.update(&(self.children.len() as u64).to_le_bytes());
        for (key, child_hash) in &self.children {
            hasher.update(key.as_bytes());
            hasher.update(child_hash.as_bytes());
        }
        
        // Hash content if present
        if let Some(ref content) = self.content {
            hasher.update(b"CONTENT:");
            hasher.update(&(content.len() as u64).to_le_bytes());
            hasher.update(content);
        }
        
        // Hash metadata counts (for structural integrity)
        hasher.update(b"METADATA:");
        hasher.update(&self.metadata.message_count.to_le_bytes());
        hasher.update(&self.metadata.operation_count.to_le_bytes());
        hasher.update(&self.metadata.content_size.to_le_bytes());
        
        Ok(hasher.finalize().into())
    }

    /// Adds a child node to this node.
    ///
    /// # Errors
    ///
    /// Returns an error if the hash cannot be recomputed.
    pub fn add_child(&mut self, key: String, child_hash: Blake3Hash) -> Result<()> {
        self.children.insert(key, child_hash);
        self.metadata.updated_at = Utc::now();
        self.hash = self.compute_hash()?;
        Ok(())
    }

    /// Removes a child node from this node.
    ///
    /// # Errors
    ///
    /// Returns an error if the hash cannot be recomputed.
    pub fn remove_child(&mut self, key: &str) -> Result<Option<Blake3Hash>> {
        let removed = self.children.remove(key);
        if removed.is_some() {
            self.metadata.updated_at = Utc::now();
            self.hash = self.compute_hash()?;
        }
        Ok(removed)
    }

    /// Updates the metadata and recomputes the hash.
    ///
    /// # Errors
    ///
    /// Returns an error if the hash cannot be recomputed.
    pub fn update_metadata(&mut self, metadata: NodeMetadata) -> Result<()> {
        self.metadata = metadata;
        self.metadata.updated_at = Utc::now();
        self.hash = self.compute_hash()?;
        Ok(())
    }

    /// Returns true if this is a leaf node (no children).
    #[must_use]
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Returns the number of direct children.
    #[must_use]
    pub fn child_count(&self) -> usize {
        self.children.len()
    }

    /// Returns an iterator over child hashes.
    pub fn child_hashes(&self) -> impl Iterator<Item = &Blake3Hash> {
        self.children.values()
    }

    /// Returns an iterator over child keys and hashes.
    pub fn child_entries(&self) -> impl Iterator<Item = (&String, &Blake3Hash)> {
        self.children.iter()
    }
}

/// Builder for constructing Merkle trees from Claude Code data.
#[derive(Debug)]
pub struct TreeBuilder {
    /// Configuration for tree building.
    config: TreeBuilderConfig,
    /// Cache of created nodes to avoid duplication.
    node_cache: HashMap<Blake3Hash, MerkleNode>,
}

/// Configuration for tree building operations.
#[derive(Debug, Clone)]
pub struct TreeBuilderConfig {
    /// Whether to include message content in nodes.
    pub include_content: bool,
    /// Whether to compute operation dependencies.
    pub compute_dependencies: bool,
    /// Maximum depth of the tree (0 = unlimited).
    pub max_depth: usize,
    /// Whether to validate hashes during construction.
    pub validate_hashes: bool,
}

impl Default for TreeBuilderConfig {
    fn default() -> Self {
        Self {
            include_content: true,
            compute_dependencies: true,
            max_depth: 0,
            validate_hashes: true,
        }
    }
}

impl TreeBuilder {
    /// Creates a new tree builder with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self::with_config(TreeBuilderConfig::default())
    }

    /// Creates a new tree builder with custom configuration.
    #[must_use]
    pub fn with_config(config: TreeBuilderConfig) -> Self {
        Self {
            config,
            node_cache: HashMap::new(),
        }
    }

    /// Builds a session node from messages and operations.
    ///
    /// # Errors
    ///
    /// Returns an error if the tree cannot be constructed.
    pub fn build_session_tree(
        &mut self,
        session_id: SessionId,
        messages: &[ClaudeMessage],
        operations: &[Operation],
    ) -> Result<MerkleNode> {
        info!("Building session tree for session: {}", session_id);
        
        if messages.is_empty() {
            return Err(SniffError::invalid_session(
                "Cannot build tree from empty message list",
            ));
        }

        // Determine session time bounds
        let start_time = messages.first().and_then(|m| m.timestamp()).unwrap_or(Utc::now());
        let end_time = messages.last().and_then(|m| m.timestamp());

        // Build message nodes
        let mut message_children = BTreeMap::new();
        let mut total_content_size = 0u64;

        for message in messages {
            let message_node = self.build_message_node(message)?;
            if let Some(ref content) = message_node.content {
                total_content_size += content.len() as u64;
            }
            if let Some(uuid) = message.uuid() {
                message_children.insert(
                    uuid.clone(),
                    message_node.hash,
                );
            }
            self.node_cache.insert(message_node.hash, message_node);
        }

        // Build operation nodes
        let mut operation_children = BTreeMap::new();
        for operation in operations {
            let operation_node = self.build_operation_node(operation)?;
            operation_children.insert(
                operation.tool_use_id.clone(),
                operation_node.hash,
            );
            self.node_cache.insert(operation_node.hash, operation_node);
        }

        // Combine all children
        let mut all_children = BTreeMap::new();
        
        // Add message children with prefix
        for (key, hash) in message_children {
            all_children.insert(format!("msg:{key}"), hash);
        }
        
        // Add operation children with prefix
        for (key, hash) in operation_children {
            all_children.insert(format!("op:{key}"), hash);
        }

        // Create session metadata
        let metadata = NodeMetadata {
            message_count: messages.len(),
            operation_count: operations.len(),
            content_size: total_content_size,
            ..Default::default()
        };

        // Create session node
        let session_node = MerkleNode::new(
            NodeType::Session {
                session_id: session_id.clone(),
                start_time,
                end_time,
            },
            metadata,
            all_children,
            None, // Parent will be set when added to project
            None, // Sessions don't have direct content
        )?;

        debug!(
            "Built session tree with {} messages, {} operations, hash: {}",
            messages.len(),
            operations.len(),
            session_node.hash
        );

        Ok(session_node)
    }

    /// Builds a message node from a Claude message.
    fn build_message_node(&self, message: &ClaudeMessage) -> Result<MerkleNode> {
        let content = if self.config.include_content {
            // Use JSON serialization which is compatible with untagged enums
            let content_bytes = serde_json::to_vec(message)
                .map_err(|e| SniffError::hash_computation(format!("Failed to serialize message: {e}")))?;
            Some(content_bytes)
        } else {
            None
        };

        let metadata = NodeMetadata {
            message_count: 1,
            operation_count: 0,
            content_size: content.as_ref().map_or(0, |c| c.len()) as u64,
            ..Default::default()
        };

        let node_type = NodeType::Message {
            message_uuid: message.uuid().unwrap_or(&"<summary>".to_string()).clone(),
            timestamp: message.timestamp().unwrap_or(Utc::now()),
            role: match message {
                crate::types::ClaudeMessage::User(_) => "user".to_string(),
                crate::types::ClaudeMessage::Assistant(_) => "assistant".to_string(),
                crate::types::ClaudeMessage::Summary(_) => "summary".to_string(),
            },
        };

        MerkleNode::new(
            node_type,
            metadata,
            BTreeMap::new(), // Messages are leaf nodes
            None, // Parent will be set when added to session
            content,
        )
    }

    /// Builds an operation node from an operation.
    fn build_operation_node(&self, operation: &Operation) -> Result<MerkleNode> {
        let content = if self.config.include_content {
            let content_bytes = serde_json::to_vec(operation)
                .map_err(|e| SniffError::hash_computation(format!("Failed to serialize operation: {e}")))?;
            Some(content_bytes)
        } else {
            None
        };

        let metadata = NodeMetadata {
            message_count: 0,
            operation_count: 1,
            content_size: content.as_ref().map_or(0, |c| c.len()) as u64,
            ..Default::default()
        };

        let node_type = NodeType::Operation {
            tool_use_id: operation.tool_use_id.clone(),
            tool_name: operation.tool_name.clone(),
            timestamp: operation.timestamp,
        };

        MerkleNode::new(
            node_type,
            metadata,
            BTreeMap::new(), // Operations are leaf nodes
            None, // Parent will be set when added to session
            content,
        )
    }

    /// Builds a project node from session nodes.
    ///
    /// # Errors
    ///
    /// Returns an error if the tree cannot be constructed.
    pub fn build_project_tree(
        &mut self,
        project_name: String,
        project_path: PathBuf,
        session_nodes: Vec<MerkleNode>,
    ) -> Result<MerkleNode> {
        info!("Building project tree for project: {}", project_name);

        let mut children = BTreeMap::new();
        let mut total_messages = 0;
        let mut total_operations = 0;
        let mut total_size = 0;

        for session_node in session_nodes {
            // Extract session ID from node type
            let session_id = match &session_node.node_type {
                NodeType::Session { session_id, .. } => session_id.clone(),
                _ => return Err(SniffError::invalid_session(
                    "Expected session node type",
                )),
            };

            total_messages += session_node.metadata.message_count;
            total_operations += session_node.metadata.operation_count;
            total_size += session_node.metadata.content_size;

            children.insert(session_id.clone(), session_node.hash);
            self.node_cache.insert(session_node.hash, session_node);
        }

        let metadata = NodeMetadata {
            message_count: total_messages,
            operation_count: total_operations,
            content_size: total_size,
            ..Default::default()
        };

        let project_node = MerkleNode::new(
            NodeType::Project {
                name: project_name.clone(),
                cwd: project_path,
            },
            metadata,
            children,
            None, // Parent will be set when added to root
            None, // Projects don't have direct content
        )?;

        debug!(
            "Built project tree '{}' with {} sessions, hash: {}",
            project_name,
            project_node.children.len(),
            project_node.hash
        );

        Ok(project_node)
    }

    /// Retrieves a cached node by hash.
    #[must_use]
    pub fn get_cached_node(&self, hash: &Blake3Hash) -> Option<&MerkleNode> {
        self.node_cache.get(hash)
    }

    /// Returns the number of cached nodes.
    #[must_use]
    pub fn cache_size(&self) -> usize {
        self.node_cache.len()
    }

    /// Clears the node cache.
    pub fn clear_cache(&mut self) {
        self.node_cache.clear();
    }

    /// Returns an iterator over cached nodes.
    pub fn cached_nodes(&self) -> impl Iterator<Item = (&Blake3Hash, &MerkleNode)> {
        self.node_cache.iter()
    }
}

impl Default for TreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Utilities for working with Merkle trees.
pub mod utils {
    use super::*;

    /// Validates the integrity of a Merkle tree node.
    ///
    /// # Errors
    ///
    /// Returns an error if the node's hash is invalid.
    pub fn validate_node(node: &MerkleNode) -> Result<()> {
        let computed_hash = node.compute_hash()?;
        if computed_hash != node.hash {
            return Err(SniffError::hash_computation(
                format!("Node hash mismatch: expected {}, got {}", node.hash, computed_hash),
            ));
        }
        Ok(())
    }

    /// Computes statistics for a tree node and its subtree.
    #[must_use]
    pub fn compute_tree_stats(node: &MerkleNode) -> TreeStats {
        TreeStats {
            total_nodes: 1 + node.children.len(),
            total_messages: node.metadata.message_count,
            total_operations: node.metadata.operation_count,
            total_size: node.metadata.content_size,
            tree_depth: 1, // Would need recursive calculation for actual depth
            leaf_nodes: if node.is_leaf() { 1 } else { 0 },
        }
    }

    /// Finds the path from root to a target hash.
    pub fn find_path_to_hash(
        root: &MerkleNode,
        target_hash: &Blake3Hash,
        cache: &HashMap<Blake3Hash, MerkleNode>,
    ) -> Option<Vec<String>> {
        if root.hash == *target_hash {
            return Some(vec![]);
        }

        for (key, child_hash) in &root.children {
            if let Some(child_node) = cache.get(child_hash) {
                if let Some(mut path) = find_path_to_hash(child_node, target_hash, cache) {
                    path.insert(0, key.clone());
                    return Some(path);
                }
            }
        }

        None
    }
}

/// Statistics about a Merkle tree.
#[derive(Debug, Clone, PartialEq)]
pub struct TreeStats {
    /// Total number of nodes in the tree.
    pub total_nodes: usize,
    /// Total number of messages across all nodes.
    pub total_messages: usize,
    /// Total number of operations across all nodes.
    pub total_operations: usize,
    /// Total content size in bytes.
    pub total_size: u64,
    /// Maximum depth of the tree.
    pub tree_depth: usize,
    /// Number of leaf nodes.
    pub leaf_nodes: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use chrono::Utc;
    use std::collections::HashMap;

    fn create_test_message(uuid: &str, parent_uuid: Option<&str>) -> ClaudeMessage {
        let base = MessageBase {
            uuid: uuid.to_string(),
            parent_uuid: parent_uuid.map(|s| s.to_string()),
            is_sidechain: false,
            is_meta: None,
            user_type: "external".to_string(),
            cwd: PathBuf::from("/test"),
            session_id: "test-session".to_string(),
            version: "1.0.0".to_string(),
            timestamp: Utc::now(),
        };

        ClaudeMessage::User(UserMessage {
            base,
            message: UserMessageContent {
                role: "user".to_string(),
                content: UserContentType::Text("Hello".to_string()),
            },
            tool_use_result: None,
        })
    }

    fn create_test_operation(id: &str) -> Operation {
        crate::operations::Operation {
            tool_use_id: id.to_string(),
            operation_type: crate::operations::OperationType::FileRead,
            tool_name: "Read".to_string(),
            status: crate::operations::OperationStatus::Success,
            timestamp: Utc::now(),
            working_directory: PathBuf::from("/test"),
            message_uuid: "msg1".to_string(),
            file_paths: vec![PathBuf::from("/test/file.txt")],
            command: None,
            input_parameters: HashMap::new(),
            output_data: None,
            duration_ms: None,
            modified_files: false,
            dependencies: Vec::new(),
        }
    }

    #[test]
    fn test_node_creation() {
        let metadata = NodeMetadata::default();
        let children = BTreeMap::new();
        
        let node = MerkleNode::new(
            NodeType::Root,
            metadata,
            children,
            None,
            None,
        ).unwrap();

        assert!(!node.hash.is_null());
        assert!(node.is_leaf());
        assert_eq!(node.child_count(), 0);
    }

    #[test]
    fn test_node_hash_consistency() {
        let metadata = NodeMetadata::default();
        let mut children = BTreeMap::new();
        children.insert("child1".to_string(), Blake3Hash::new([1u8; 32]));
        
        let node1 = MerkleNode::new(
            NodeType::Root,
            metadata.clone(),
            children.clone(),
            None,
            None,
        ).unwrap();

        let node2 = MerkleNode::new(
            NodeType::Root,
            metadata,
            children,
            None,
            None,
        ).unwrap();

        assert_eq!(node1.hash, node2.hash);
    }

    #[test]
    fn test_add_remove_child() {
        let metadata = NodeMetadata::default();
        let children = BTreeMap::new();
        
        let mut node = MerkleNode::new(
            NodeType::Root,
            metadata,
            children,
            None,
            None,
        ).unwrap();

        let original_hash = node.hash;
        let child_hash = Blake3Hash::new([1u8; 32]);

        node.add_child("child1".to_string(), child_hash).unwrap();
        assert_ne!(node.hash, original_hash);
        assert_eq!(node.child_count(), 1);

        let removed = node.remove_child("child1").unwrap();
        assert_eq!(removed, Some(child_hash));
        assert_eq!(node.child_count(), 0);
    }

    #[test]
    fn test_tree_builder_session() {
        let mut builder = TreeBuilder::new();
        
        let messages = vec![
            create_test_message("msg1", None),
            create_test_message("msg2", Some("msg1")),
        ];
        
        let operations = vec![
            create_test_operation("op1"),
            create_test_operation("op2"),
        ];

        let session_node = builder.build_session_tree(
            "test-session".to_string(),
            &messages,
            &operations,
        ).unwrap();

        assert!(!session_node.hash.is_null());
        assert_eq!(session_node.metadata.message_count, 2);
        assert_eq!(session_node.metadata.operation_count, 2);
        assert_eq!(session_node.child_count(), 4); // 2 messages + 2 operations
    }

    #[test]
    fn test_tree_builder_project() {
        let mut builder = TreeBuilder::new();
        
        let messages = vec![create_test_message("msg1", None)];
        let operations = vec![create_test_operation("op1")];

        let session_node = builder.build_session_tree(
            "session1".to_string(),
            &messages,
            &operations,
        ).unwrap();

        let project_node = builder.build_project_tree(
            "test-project".to_string(),
            PathBuf::from("/test/project"),
            vec![session_node],
        ).unwrap();

        assert!(!project_node.hash.is_null());
        assert_eq!(project_node.metadata.message_count, 1);
        assert_eq!(project_node.metadata.operation_count, 1);
        assert_eq!(project_node.child_count(), 1);
    }

    #[test]
    fn test_utils_validate_node() {
        let metadata = NodeMetadata::default();
        let children = BTreeMap::new();
        
        let node = MerkleNode::new(
            NodeType::Root,
            metadata,
            children,
            None,
            None,
        ).unwrap();

        assert!(utils::validate_node(&node).is_ok());
    }

    #[test]
    fn test_tree_stats() {
        let metadata = NodeMetadata {
            message_count: 5,
            operation_count: 3,
            content_size: 1024,
            ..Default::default()
        };
        
        let node = MerkleNode::new(
            NodeType::Root,
            metadata,
            BTreeMap::new(),
            None,
            None,
        ).unwrap();

        let stats = utils::compute_tree_stats(&node);
        assert_eq!(stats.total_messages, 5);
        assert_eq!(stats.total_operations, 3);
        assert_eq!(stats.total_size, 1024);
        assert_eq!(stats.leaf_nodes, 1);
    }
}