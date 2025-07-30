// Copyright (c) 2025 Chetan Conikee <conikee@gmail.com>
// Licensed under the MIT License

//! Sniff - Code Quality Analysis and AI Deception Detection
//!
//! This library provides comprehensive tools for analyzing code quality,
//! detecting AI-generated deception patterns, and implementing quality gates
//! in development workflows.

#![warn(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::unused_self)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::explicit_counter_loop)]
#![allow(clippy::ref_option)]
#![allow(clippy::match_same_arms)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::manual_let_else)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::manual_flatten)]

pub mod analysis;
pub mod display;
pub mod error;
pub mod pattern_learning;
pub mod playbook;
pub mod standalone;
pub mod types;
pub mod verify_todo;

// Re-export commonly used types
pub use analysis::{
    MisalignmentAnalyzer, MisalignmentDetection, ContextLines, EnhancedMisalignmentAnalysis, PerformanceImpact,
    QualityAssessment, SemanticContextResult, SupportedLanguage,
};
pub use display::MisalignmentDisplayFormatter;
pub use error::{Result, SniffError};
pub use pattern_learning::{
    LearnedPattern, LearningConfig, PatternCreationRequest, PatternCreationResponse,
    PatternLearningManager, PatternMetadata, PatternStatistics,
};
// Note: types.rs can be further cleaned up to remove Claude-specific types
