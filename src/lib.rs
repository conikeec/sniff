// Copyright (c) 2025 Chetan Conikee <conikee@gmail.com>
// Licensed under the MIT License

//! Sniff CLI - Advanced navigation and search for Claude Code session histories.
//!
//! This library provides comprehensive tools for analyzing, indexing, and searching
//! through Claude Code session data using Merkle trees and full-text search.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod analysis;
pub mod display;
pub mod error;
pub mod hash;
pub mod jsonl;
pub mod operations;
pub mod playbook;
pub mod progress;
pub mod search;
pub mod session;
pub mod simple_session_analyzer;
pub mod storage;
pub mod tree;
pub mod types;
pub mod watcher;

// Re-export commonly used types
pub use analysis::{
    BullshitAnalyzer, BullshitDetection, SupportedLanguage,
    EnhancedBullshitAnalysis, PerformanceImpact, QualityAssessment, SemanticContextResult, ContextLines,
};
pub use display::BullshitDisplayFormatter;
pub use error::{SniffError, Result};
pub use simple_session_analyzer::{SimpleSessionAnalyzer, SimpleSessionAnalysis};
pub use types::{ClaudeMessage, MessageUuid, SessionId, ToolUseId, ToolUseOperation};
