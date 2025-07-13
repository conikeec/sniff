// Copyright (c) 2025 Chetan Conikee <conikee@gmail.com>
// Licensed under the MIT License

//! Claude Tree CLI - Advanced navigation and search for Claude Code session histories.
//!
//! This library provides comprehensive tools for analyzing, indexing, and searching
//! through Claude Code session data using Merkle trees and full-text search.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod error;
pub mod hash;
pub mod jsonl;
pub mod operations;
pub mod session;
pub mod storage;
pub mod tree;
pub mod types;
pub mod watcher;

// Re-export commonly used types
pub use error::{ClaudeTreeError, Result};
pub use types::{ClaudeMessage, MessageUuid, SessionId, ToolUseId, ToolUseOperation};
