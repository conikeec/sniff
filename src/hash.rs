// Copyright (c) 2025 Chetan Conikee <conikee@gmail.com>
// Licensed under the MIT License

//! Hash utilities and Blake3 integration for Merkle tree operations.
//!
//! This module provides cryptographic hashing functionality using Blake3,
//! including utilities for computing hashes of various data types and
//! creating deterministic content hashes.

use crate::error::{Result, SniffError};
use blake3::{Hash, Hasher};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A Blake3 hash value used in Merkle tree operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Blake3Hash([u8; 32]);

impl Blake3Hash {
    /// Creates a new `Blake3Hash` from a 32-byte array.
    #[must_use]
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns the hash as a byte array.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Returns the hash as a byte slice.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Returns the hash as a hexadecimal string.
    #[must_use]
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Parses a hexadecimal string into a `Blake3Hash`.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is not valid hexadecimal or not 64 characters long.
    pub fn from_hex(hex_str: &str) -> Result<Self> {
        if hex_str.len() != 64 {
            return Err(SniffError::hash_computation(
                "Hash hex string must be exactly 64 characters",
            ));
        }

        let bytes = hex::decode(hex_str)
            .map_err(|e| SniffError::hash_computation(format!("Invalid hex string: {e}")))?;

        let mut hash_bytes = [0u8; 32];
        hash_bytes.copy_from_slice(&bytes);
        Ok(Self::new(hash_bytes))
    }

    /// Creates a null hash (all zeros).
    #[must_use]
    pub fn null() -> Self {
        Self([0u8; 32])
    }

    /// Returns true if this is a null hash.
    #[must_use]
    pub fn is_null(&self) -> bool {
        self.0 == [0u8; 32]
    }
}

impl fmt::Display for Blake3Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl From<Hash> for Blake3Hash {
    fn from(hash: Hash) -> Self {
        Self(*hash.as_bytes())
    }
}

impl From<Blake3Hash> for Hash {
    fn from(hash: Blake3Hash) -> Self {
        Hash::from(hash.0)
    }
}

/// Utilities for computing Blake3 hashes of various data types.
pub struct HashUtils;

impl HashUtils {
    /// Computes the Blake3 hash of arbitrary bytes.
    #[must_use]
    pub fn hash_bytes(data: &[u8]) -> Blake3Hash {
        blake3::hash(data).into()
    }

    /// Computes the Blake3 hash of a string.
    #[must_use]
    pub fn hash_string(data: &str) -> Blake3Hash {
        Self::hash_bytes(data.as_bytes())
    }

    /// Computes the Blake3 hash of a serializable object.
    ///
    /// # Errors
    ///
    /// Returns an error if the object cannot be serialized.
    pub fn hash_json<T: Serialize>(data: &T) -> Result<Blake3Hash> {
        let json_bytes = serde_json::to_vec(data).map_err(|e| {
            SniffError::hash_computation(format!("Failed to serialize for hashing: {e}"))
        })?;
        Ok(Self::hash_bytes(&json_bytes))
    }

    /// Computes a hash from multiple input hashes (for Merkle tree internal nodes).
    #[must_use]
    pub fn hash_combine(hash_list: &[Blake3Hash]) -> Blake3Hash {
        let mut hasher = Hasher::new();

        // Add a domain separator to prevent length extension attacks
        hasher.update(b"MERKLE_COMBINE:");
        hasher.update(&(hash_list.len() as u64).to_le_bytes());

        for hash in hash_list {
            hasher.update(hash.as_bytes());
        }

        hasher.finalize().into()
    }

    /// Computes a content hash for Claude Code messages.
    ///
    /// This creates a deterministic hash based on message content,
    /// excluding volatile fields like timestamps.
    ///
    /// # Errors
    ///
    /// Returns an error if the message cannot be processed for hashing.
    pub fn hash_message_content(message: &crate::types::ClaudeMessage) -> Result<Blake3Hash> {
        use crate::types::ClaudeMessage;

        let mut hasher = Hasher::new();

        // Add domain separator
        hasher.update(b"CLAUDE_MESSAGE:");

        // Hash stable content based on message type
        match message {
            ClaudeMessage::User(user_msg) => {
                hasher.update(b"USER:");
                hasher.update(user_msg.base.uuid.as_bytes());

                if let Some(ref parent) = user_msg.base.parent_uuid {
                    hasher.update(b"PARENT:");
                    hasher.update(parent.as_bytes());
                }

                hasher.update(b"CONTENT:");
                let content_bytes = serde_json::to_vec(&user_msg.message.content).map_err(|e| {
                    SniffError::hash_computation(format!(
                        "Failed to serialize user message content: {e}"
                    ))
                })?;
                hasher.update(&content_bytes);
            }
            ClaudeMessage::Assistant(assistant_msg) => {
                hasher.update(b"ASSISTANT:");
                hasher.update(assistant_msg.base.uuid.as_bytes());

                if let Some(ref parent) = assistant_msg.base.parent_uuid {
                    hasher.update(b"PARENT:");
                    hasher.update(parent.as_bytes());
                }

                hasher.update(b"CONTENT:");
                let content_bytes =
                    serde_json::to_vec(&assistant_msg.message.content).map_err(|e| {
                        SniffError::hash_computation(format!(
                            "Failed to serialize assistant message content: {e}"
                        ))
                    })?;
                hasher.update(&content_bytes);
            }
            ClaudeMessage::Summary(summary_msg) => {
                hasher.update(b"SUMMARY:");
                hasher.update(summary_msg.leaf_uuid.as_bytes());
                hasher.update(b"CONTENT:");
                hasher.update(summary_msg.summary.as_bytes());
            }
        }

        Ok(hasher.finalize().into())
    }

    /// Computes a hash for operation data.
    ///
    /// # Errors
    ///
    /// Returns an error if the operation cannot be processed for hashing.
    pub fn hash_operation(operation: &crate::operations::Operation) -> Result<Blake3Hash> {
        let mut hasher = Hasher::new();

        // Add domain separator
        hasher.update(b"OPERATION:");

        // Hash stable operation fields
        hasher.update(operation.tool_use_id.as_bytes());
        hasher.update(operation.tool_name.as_bytes());
        hasher.update(operation.message_uuid.as_bytes());

        // Hash operation type
        let op_type_bytes = serde_json::to_vec(&operation.operation_type).map_err(|e| {
            SniffError::hash_computation(format!("Failed to serialize operation type: {e}"))
        })?;
        hasher.update(&op_type_bytes);

        // Hash file paths
        for path in &operation.file_paths {
            hasher.update(path.to_string_lossy().as_bytes());
        }

        // Hash command if present
        if let Some(ref command) = operation.command {
            hasher.update(b"COMMAND:");
            hasher.update(command.as_bytes());
        }

        Ok(hasher.finalize().into())
    }

    /// Validates a hash string format.
    #[must_use]
    pub fn is_valid_hash_string(hash_str: &str) -> bool {
        hash_str.len() == 64 && hash_str.chars().all(|c| c.is_ascii_hexdigit())
    }
}

/// A key-value pair that can be hashed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashableEntry<K, V> {
    /// The key.
    pub key: K,
    /// The value.
    pub value: V,
}

impl<K: Serialize, V: Serialize> HashableEntry<K, V> {
    /// Creates a new hashable entry.
    #[must_use]
    pub fn new(key: K, value: V) -> Self {
        Self { key, value }
    }

    /// Computes the hash of this entry.
    ///
    /// # Errors
    ///
    /// Returns an error if the entry cannot be serialized.
    pub fn compute_hash(&self) -> Result<Blake3Hash> {
        HashUtils::hash_json(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blake3_hash_creation() {
        let bytes = [1u8; 32];
        let hash = Blake3Hash::new(bytes);
        assert_eq!(hash.as_bytes(), &bytes);
    }

    #[test]
    fn test_blake3_hash_hex() {
        let hash = Blake3Hash::new([0u8; 32]);
        let hex = hash.to_hex();
        assert_eq!(hex.len(), 64);
        assert!(hex.chars().all(|c| c == '0'));

        let parsed = Blake3Hash::from_hex(&hex).unwrap();
        assert_eq!(hash, parsed);
    }

    #[test]
    fn test_blake3_hash_invalid_hex() {
        assert!(Blake3Hash::from_hex("invalid").is_err());
        assert!(Blake3Hash::from_hex("too_short").is_err());
        assert!(Blake3Hash::from_hex("x".repeat(64).as_str()).is_err());
    }

    #[test]
    fn test_blake3_hash_null() {
        let null_hash = Blake3Hash::null();
        assert!(null_hash.is_null());

        let non_null = Blake3Hash::new([1u8; 32]);
        assert!(!non_null.is_null());
    }

    #[test]
    fn test_hash_utils_basic() {
        let data = b"hello world";
        let hash1 = HashUtils::hash_bytes(data);
        let hash2 = HashUtils::hash_bytes(data);
        assert_eq!(hash1, hash2);

        let hash3 = HashUtils::hash_string("hello world");
        assert_eq!(hash1, hash3);
    }

    #[test]
    fn test_hash_utils_json() {
        let data = serde_json::json!({
            "key": "value",
            "number": 42
        });

        let hash1 = HashUtils::hash_json(&data).unwrap();
        let hash2 = HashUtils::hash_json(&data).unwrap();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hash_combine() {
        let hash1 = Blake3Hash::new([1u8; 32]);
        let hash2 = Blake3Hash::new([2u8; 32]);
        let hash3 = Blake3Hash::new([3u8; 32]);

        let combined1 = HashUtils::hash_combine(&[hash1, hash2]);
        let combined2 = HashUtils::hash_combine(&[hash1, hash2]);
        assert_eq!(combined1, combined2);

        let combined3 = HashUtils::hash_combine(&[hash2, hash1]);
        assert_ne!(combined1, combined3); // Order matters

        let combined4 = HashUtils::hash_combine(&[hash1, hash2, hash3]);
        assert_ne!(combined1, combined4);
    }

    #[test]
    fn test_hash_validation() {
        assert!(HashUtils::is_valid_hash_string("a".repeat(64).as_str()));
        assert!(HashUtils::is_valid_hash_string(
            "0123456789abcdef".repeat(4).as_str()
        ));
        assert!(!HashUtils::is_valid_hash_string("x".repeat(64).as_str()));
        assert!(!HashUtils::is_valid_hash_string("a".repeat(63).as_str()));
        assert!(!HashUtils::is_valid_hash_string("a".repeat(65).as_str()));
    }

    #[test]
    fn test_hashable_entry() {
        let entry = HashableEntry::new("key", "value");
        let hash1 = entry.compute_hash().unwrap();
        let hash2 = entry.compute_hash().unwrap();
        assert_eq!(hash1, hash2);

        let entry2 = HashableEntry::new("key", "different_value");
        let hash3 = entry2.compute_hash().unwrap();
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_display_format() {
        let hash = Blake3Hash::new([0u8; 32]);
        let display = format!("{hash}");
        assert_eq!(display, "0".repeat(64));
    }
}
