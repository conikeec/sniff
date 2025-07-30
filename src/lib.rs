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
// Allow these specific clippy warnings for legitimate reasons
#![allow(clippy::missing_errors_doc)] // Error docs would be repetitive for Result<T>
#![allow(clippy::cast_precision_loss)] // Necessary for quality score calculations

pub mod analysis;
pub mod display;
pub mod error;
pub mod pattern_learning;
pub mod playbook;
pub mod standalone;

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
