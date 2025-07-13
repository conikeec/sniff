// Copyright (c) 2025 Chetan Conikee <conikee@gmail.com>
// Licensed under the MIT License

//! Error handling for sniff.
//!
//! This module provides comprehensive error types for all operations
//! within the sniff application, from JSONL parsing to file
//! system operations and data processing.

use std::path::PathBuf;
use thiserror::Error;

/// Result type alias for sniff operations.
pub type Result<T> = std::result::Result<T, SniffError>;

/// Comprehensive error type for all sniff operations.
#[derive(Error, Debug)]
pub enum SniffError {
    /// Error occurred during JSONL parsing operations.
    #[error("JSONL parsing error at line {line}: {source}")]
    JsonlParse {
        /// Line number where the error occurred.
        line: usize,
        /// The underlying JSON parsing error.
        #[source]
        source: serde_json::Error,
    },

    /// Error occurred during file system operations.
    #[error("File system error for path '{path}': {source}")]
    FileSystem {
        /// The file path that caused the error.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Error occurred during file watching operations.
    #[error("File watcher error: {source}")]
    FileWatcher {
        /// The underlying notify error.
        #[source]
        source: notify::Error,
    },

    /// Error occurred during session validation.
    #[error("Invalid session structure: {reason}")]
    InvalidSession {
        /// The reason for the validation failure.
        reason: String,
    },

    /// Error occurred during message validation.
    #[error("Invalid message in session {session_id}, message {message_uuid}: {reason}")]
    InvalidMessage {
        /// The session ID containing the invalid message.
        session_id: String,
        /// The UUID of the invalid message.
        message_uuid: String,
        /// The reason for the validation failure.
        reason: String,
    },

    /// Error occurred during operation extraction.
    #[error("Operation extraction error: {reason}")]
    OperationExtraction {
        /// The reason for the extraction failure.
        reason: String,
    },

    /// Error occurred during hash computation.
    #[error("Hash computation error: {reason}")]
    HashComputation {
        /// The reason for the hash computation failure.
        reason: String,
    },

    /// Error occurred during project discovery.
    #[error("Project discovery error for path '{path}': {reason}")]
    ProjectDiscovery {
        /// The project path that caused the error.
        path: PathBuf,
        /// The reason for the discovery failure.
        reason: String,
    },

    /// A required field was missing from the data structure.
    #[error("Missing required field '{field}' in {context}")]
    MissingField {
        /// The name of the missing field.
        field: String,
        /// The context where the field was expected.
        context: String,
    },

    /// An invalid data format was encountered.
    #[error("Invalid data format in {context}: {reason}")]
    InvalidFormat {
        /// The context where the invalid format was encountered.
        context: String,
        /// The reason for the format invalidity.
        reason: String,
    },

    /// Error occurred during storage operations.
    #[error("Storage error: {reason}")]
    StorageError {
        /// The reason for the storage failure.
        reason: String,
    },

    /// Error occurred during language detection or TreeSitter parsing.
    #[error("Language detection error: {reason}")]
    LanguageDetection {
        /// The reason for the language detection failure.
        reason: String,
    },

    /// Error occurred during code analysis.
    #[error("Code analysis error: {reason}")]
    AnalysisError {
        /// The reason for the analysis failure.
        reason: String,
    },
}

impl SniffError {
    /// Creates a new JSONL parsing error.
    pub fn jsonl_parse(line: usize, source: serde_json::Error) -> Self {
        Self::JsonlParse { line, source }
    }

    /// Creates a new file system error.
    pub fn file_system(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::FileSystem {
            path: path.into(),
            source,
        }
    }

    /// Creates a new file watcher error.
    pub fn file_watcher(source: notify::Error) -> Self {
        Self::FileWatcher { source }
    }

    /// Creates a new invalid session error.
    pub fn invalid_session(reason: impl Into<String>) -> Self {
        Self::InvalidSession {
            reason: reason.into(),
        }
    }

    /// Creates a new invalid message error.
    pub fn invalid_message(
        session_id: impl Into<String>,
        message_uuid: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::InvalidMessage {
            session_id: session_id.into(),
            message_uuid: message_uuid.into(),
            reason: reason.into(),
        }
    }

    /// Creates a new operation extraction error.
    pub fn operation_extraction(reason: impl Into<String>) -> Self {
        Self::OperationExtraction {
            reason: reason.into(),
        }
    }

    /// Creates a new hash computation error.
    pub fn hash_computation(reason: impl Into<String>) -> Self {
        Self::HashComputation {
            reason: reason.into(),
        }
    }

    /// Creates a new project discovery error.
    pub fn project_discovery(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        Self::ProjectDiscovery {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Creates a new missing field error.
    pub fn missing_field(field: impl Into<String>, context: impl Into<String>) -> Self {
        Self::MissingField {
            field: field.into(),
            context: context.into(),
        }
    }

    /// Creates a new invalid format error.
    pub fn invalid_format(context: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidFormat {
            context: context.into(),
            reason: reason.into(),
        }
    }

    /// Creates a new storage error.
    pub fn storage_error(reason: impl Into<String>) -> Self {
        Self::StorageError {
            reason: reason.into(),
        }
    }

    /// Creates a new language detection error.
    pub fn language_detection(reason: impl Into<String>) -> Self {
        Self::LanguageDetection {
            reason: reason.into(),
        }
    }

    /// Creates a new code analysis error.
    pub fn analysis_error(reason: impl Into<String>) -> Self {
        Self::AnalysisError {
            reason: reason.into(),
        }
    }
}

// Automatic conversions from common error types
impl From<serde_json::Error> for SniffError {
    fn from(error: serde_json::Error) -> Self {
        Self::InvalidFormat {
            context: "JSON serialization/deserialization".to_string(),
            reason: error.to_string(),
        }
    }
}

impl From<std::io::Error> for SniffError {
    fn from(error: std::io::Error) -> Self {
        Self::StorageError {
            reason: format!("I/O operation failed: {error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_error_creation() {
        // Create a real JSON parsing error by parsing invalid JSON
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let jsonl_error = SniffError::jsonl_parse(42, json_error);
        assert!(matches!(
            jsonl_error,
            SniffError::JsonlParse { line: 42, .. }
        ));

        let fs_error = SniffError::file_system(
            "/tmp/test",
            io::Error::new(io::ErrorKind::NotFound, "File not found"),
        );
        assert!(matches!(fs_error, SniffError::FileSystem { .. }));

        let session_error = SniffError::invalid_session("Missing session ID");
        assert!(matches!(
            session_error,
            SniffError::InvalidSession { .. }
        ));
    }

    #[test]
    fn test_error_display() {
        let error = SniffError::missing_field("uuid", "message parsing");
        let error_str = error.to_string();
        assert!(error_str.contains("Missing required field 'uuid'"));
        assert!(error_str.contains("message parsing"));
    }
}
