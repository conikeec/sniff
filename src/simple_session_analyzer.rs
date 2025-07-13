// Copyright (c) 2025 Chetan Conikee <conikee@gmail.com>
// Licensed under the MIT License

//! Simple, working session analyzer that uses only verified components.

use crate::analysis::{BullshitAnalyzer, BullshitDetection};
use crate::display::BullshitDisplayFormatter;
use crate::error::Result;
use crate::jsonl::JsonlParser;
use crate::operations::Operation;
use crate::operations::OperationExtractor;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Simple session analysis result that only includes verified data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleSessionAnalysis {
    /// Session ID
    pub session_id: String,
    /// Files that were modified in this session
    pub modified_files: Vec<String>,
    /// File operations performed
    pub file_operations: Vec<Operation>,
    /// Bullshit patterns detected in modified files
    pub bullshit_detections: Vec<crate::analysis::BullshitDetection>,
    /// Simple metrics
    pub metrics: SimpleMetrics,
    /// Recommendations based on actual findings
    pub recommendations: Vec<String>,
}

/// Simple metrics we can actually calculate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleMetrics {
    /// Number of files modified
    pub files_modified: usize,
    /// Total bullshit patterns detected
    pub total_bullshit_patterns: usize,
    /// Critical patterns (unimplemented, panic, etc.)
    pub critical_patterns: usize,
    /// Files with critical issues
    pub files_with_critical_issues: usize,
    /// Simple quality score based on detected issues
    pub quality_score: f64,
}

/// Simple session analyzer that only uses working components.
pub struct SimpleSessionAnalyzer {
    /// JSONL parser for session files
    jsonl_parser: JsonlParser,
    /// Operation extractor
    operation_extractor: OperationExtractor,
    /// Bullshit analyzer
    bullshit_analyzer: BullshitAnalyzer,
    /// Display formatter for enhanced output
    display_formatter: BullshitDisplayFormatter,
    /// Whether to suppress terminal output (for clean format export)
    quiet_mode: bool,
}

impl SimpleSessionAnalyzer {
    /// Analyzes multiple files in parallel using the bullshit analyzer.
    fn analyze_files_parallel(&mut self, file_paths: &[PathBuf]) -> Result<Vec<BullshitDetection>> {
        // Convert PathBuf to &Path for the analyzer
        let path_refs: Vec<&Path> = file_paths.iter().map(std::path::PathBuf::as_path).collect();
        self.bullshit_analyzer.analyze_files_parallel(&path_refs)
    }

    /// Creates a new simple session analyzer.
    pub fn new() -> Result<Self> {
        let jsonl_parser = JsonlParser::new();
        let operation_extractor = OperationExtractor::new();
        let bullshit_analyzer = BullshitAnalyzer::new()?;
        let display_formatter = BullshitDisplayFormatter::new();

        Ok(Self {
            jsonl_parser,
            operation_extractor,
            bullshit_analyzer,
            display_formatter,
            quiet_mode: false,
        })
    }

    /// Creates a new simple session analyzer with quiet mode.
    pub fn new_quiet() -> Result<Self> {
        let jsonl_parser = JsonlParser::new();
        let operation_extractor = OperationExtractor::new();
        let bullshit_analyzer = BullshitAnalyzer::new()?;
        let display_formatter = BullshitDisplayFormatter::new();

        Ok(Self {
            jsonl_parser,
            operation_extractor,
            bullshit_analyzer,
            display_formatter,
            quiet_mode: true,
        })
    }

    /// Sets quiet mode on/off.
    pub fn set_quiet_mode(&mut self, quiet: bool) {
        self.quiet_mode = quiet;
    }

    /// Analyzes a Claude Code session file.
    pub fn analyze_session(&mut self, session_file: &Path) -> Result<SimpleSessionAnalysis> {
        // Extract session ID from filename
        let session_id = session_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        if !self.quiet_mode {
            println!("📄 Parsing session file: {}", session_file.display());
        }

        // Parse the JSONL session file
        let parse_result = self.jsonl_parser.parse_file(session_file)?;
        let messages = parse_result.messages;
        if !self.quiet_mode {
            println!("✅ Parsed {} messages", messages.len());
        }

        // Extract file operations from all messages
        let all_operations = self.operation_extractor.extract_operations(&messages)?;

        if !self.quiet_mode {
            println!("📁 Found {} file operations", all_operations.len());
        }

        // Get ALL files mentioned in operations (don't filter yet)
        let all_mentioned_files = self.get_all_mentioned_files(&all_operations);
        if !self.quiet_mode {
            println!(
                "🔍 Found {} files mentioned in operations",
                all_mentioned_files.len()
            );
        }

        // HONEST analysis: separate existing vs missing files
        let mut all_detections = Vec::new();
        let mut files_analyzed = 0;
        let mut files_missing = Vec::new();
        let mut existing_files = Vec::new();

        for file_path in &all_mentioned_files {
            let path = Path::new(file_path);
            if !self.quiet_mode {
                println!("🔍 Checking file: {file_path}");
                println!("   Full path: {}", path.display());
                println!("   Exists: {}", path.exists());
            }
            if path.exists() {
                existing_files.push(file_path.clone());
                if !self.quiet_mode {
                    println!("   ✅ Added to existing files");
                }
            } else {
                files_missing.push(file_path.clone());
                if !self.quiet_mode {
                    println!("   ❌ Added to missing files");
                }
            }
        }

        if !self.quiet_mode {
            println!("📊 File Status:");
            println!("   Files existing: {}", existing_files.len());
            println!("   Files missing: {}", files_missing.len());
        }

        // Convert file paths to absolute paths for analysis
        let mut absolute_paths = Vec::new();
        for file_path in &existing_files {
            let path = Path::new(file_path);
            let absolute_path = if path.is_absolute() {
                path.to_path_buf()
            } else {
                match std::env::current_dir() {
                    Ok(cwd) => cwd.join(path),
                    Err(e) => {
                        println!("   ❌ Failed to get current directory for {file_path}: {e}");
                        continue;
                    }
                }
            };
            absolute_paths.push(absolute_path);
        }

        if !self.quiet_mode {
            println!(
                "🚀 Running parallel analysis on {} files...",
                absolute_paths.len()
            );
        }

        // Use parallel analysis for better performance
        match self.analyze_files_parallel(&absolute_paths) {
            Ok(detections) => {
                files_analyzed = absolute_paths.len();
                all_detections = detections;

                // Report results by file with adaptive formatting (only in non-quiet mode)
                if !self.quiet_mode {
                    for (i, file_path) in existing_files.iter().enumerate() {
                        let absolute_path = &absolute_paths[i];
                        let file_detections: Vec<_> = all_detections
                            .iter()
                            .filter(|d| d.file_path == absolute_path.to_string_lossy())
                            .cloned()
                            .collect();

                        // Use adaptive formatting based on terminal width
                        print!(
                            "{}",
                            self.display_formatter
                                .format_file_summary_adaptive(file_path, &file_detections)
                        );
                        println!(); // Add spacing between files
                    }
                }
            }
            Err(e) => {
                if !self.quiet_mode {
                    println!("   ❌ Parallel analysis failed, falling back to sequential: {e}");
                }

                // Fallback to sequential analysis
                for file_path in &existing_files {
                    let path = Path::new(file_path);
                    if !self.quiet_mode {
                        println!("🔍 Analyzing file: {file_path}");
                    }

                    let absolute_path = if path.is_absolute() {
                        path.to_path_buf()
                    } else {
                        match std::env::current_dir() {
                            Ok(cwd) => cwd.join(path),
                            Err(e) => {
                                if !self.quiet_mode {
                                    println!(
                                        "   ❌ Failed to get current directory for {file_path}: {e}"
                                    );
                                }
                                continue;
                            }
                        }
                    };

                    match self.bullshit_analyzer.analyze_file(&absolute_path) {
                        Ok(detections) => {
                            files_analyzed += 1;

                            // Use adaptive formatting based on terminal width (only in non-quiet mode)
                            if !self.quiet_mode {
                                print!(
                                    "{}",
                                    self.display_formatter
                                        .format_file_summary_adaptive(file_path, &detections)
                                );
                                println!(); // Add spacing between files
                            }

                            all_detections.extend(detections);
                        }
                        Err(e) => {
                            if !self.quiet_mode {
                                println!("   ❌ Failed to analyze {file_path}: {e}");
                            }
                        }
                    }
                }
            }
        }

        // Report missing files honestly (only in non-quiet mode)
        if !self.quiet_mode && !files_missing.is_empty() {
            println!("📁 Files that no longer exist:");
            for file_path in &files_missing {
                println!("   📁 {file_path}");
            }
        }

        if !self.quiet_mode {
            println!("📊 Analysis Summary:");
            println!("   Files found and analyzed: {files_analyzed}");
            println!("   Files missing/removed: {}", files_missing.len());
            println!("   Total bullshit patterns: {}", all_detections.len());

            // If many files with issues, show a compact tree summary
            if existing_files.len() > 5 && !all_detections.is_empty() {
                println!("\n📋 Issues Overview:");
                let file_summaries: Vec<(String, Vec<BullshitDetection>)> = existing_files
                    .iter()
                    .map(|file_path| {
                        let file_detections: Vec<_> = all_detections
                            .iter()
                            .filter(|d| d.file_path.contains(file_path))
                            .cloned()
                            .collect();
                        (file_path.clone(), file_detections)
                    })
                    .collect();

                print!(
                    "{}",
                    self.display_formatter.format_summary_tree(&file_summaries)
                );
            }
        }

        // Calculate simple metrics
        let metrics = self.calculate_metrics(
            &all_mentioned_files,
            &all_detections,
            files_analyzed,
            &files_missing,
        );

        // Generate simple recommendations
        let recommendations =
            self.generate_recommendations(&all_detections, files_analyzed, &files_missing);

        Ok(SimpleSessionAnalysis {
            session_id,
            modified_files: existing_files, // Only include files that actually exist
            file_operations: all_operations,
            bullshit_detections: all_detections,
            metrics,
            recommendations,
        })
    }

    /// Gets unique modified files from operations.
    #[allow(dead_code)]
    fn get_modified_files(&self, operations: &[Operation]) -> HashSet<String> {
        let mut files = HashSet::new();

        for operation in operations {
            // Check if it's a file modification operation
            if matches!(
                operation.operation_type,
                crate::operations::OperationType::FileEdit
                    | crate::operations::OperationType::FileWrite
                    | crate::operations::OperationType::FileCreate
            ) {
                for file_path in &operation.file_paths {
                    // Only include files that actually exist
                    if file_path.exists() {
                        if let Some(path_str) = file_path.to_str() {
                            files.insert(path_str.to_string());
                        }
                    }
                }
            }
        }

        files
    }

    /// Gets ALL files mentioned in operations (don't filter by existence).
    fn get_all_mentioned_files(&self, operations: &[Operation]) -> Vec<String> {
        let mut files = std::collections::HashSet::new();

        for operation in operations {
            // Check if it's a file modification operation
            if matches!(
                operation.operation_type,
                crate::operations::OperationType::FileEdit
                    | crate::operations::OperationType::FileWrite
                    | crate::operations::OperationType::FileCreate
            ) {
                for file_path in &operation.file_paths {
                    if let Some(path_str) = file_path.to_str() {
                        files.insert(path_str.to_string());
                    }
                }
            }
        }

        files.into_iter().collect()
    }

    /// Calculates simple, reliable metrics.
    fn calculate_metrics(
        &self,
        files: &[String],
        detections: &[crate::analysis::BullshitDetection],
        files_analyzed: usize,
        files_missing: &[String],
    ) -> SimpleMetrics {
        let files_modified = files.len();
        let total_bullshit_patterns = detections.len();

        let critical_patterns = detections
            .iter()
            .filter(|d| matches!(d.severity, crate::playbook::Severity::Critical))
            .count();

        let mut files_with_issues = HashSet::new();
        for detection in detections {
            if matches!(
                detection.severity,
                crate::playbook::Severity::Critical | crate::playbook::Severity::High
            ) {
                files_with_issues.insert(detection.file_path.clone());
            }
        }
        let files_with_critical_issues = files_with_issues.len();

        // Quality score: HONEST assessment based on what we actually analyzed
        let quality_score = if files_analyzed == 0 {
            // NO FILES ANALYZED = CANNOT DETERMINE QUALITY
            0.0 // Be honest: we have no data
        } else {
            // Only calculate quality for files we actually analyzed
            let base_score = 100.0;
            let pattern_deduction =
                (critical_patterns as f64 * 20.0) + (total_bullshit_patterns as f64 * 5.0);
            let missing_file_penalty = files_missing.len() as f64 * 10.0; // Penalty for missing files
            (base_score - pattern_deduction - missing_file_penalty).max(0.0)
        };

        SimpleMetrics {
            files_modified,
            total_bullshit_patterns,
            critical_patterns,
            files_with_critical_issues,
            quality_score,
        }
    }

    /// Generates simple, actionable recommendations.
    fn generate_recommendations(
        &self,
        detections: &[crate::analysis::BullshitDetection],
        files_analyzed: usize,
        files_missing: &[String],
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        // CRITICAL: Address missing files first
        if files_analyzed == 0 {
            recommendations.push("🚨 CRITICAL: NO FILES COULD BE ANALYZED - All files from this session have been removed".to_string());
            recommendations.push(
                "📊 Analysis reliability: INVALID - Cannot determine code quality".to_string(),
            );
            if !files_missing.is_empty() {
                recommendations.push(format!(
                    "📁 {} files were created/modified but no longer exist in the project",
                    files_missing.len()
                ));
            }
            return recommendations;
        }

        if !files_missing.is_empty() {
            recommendations.push(format!(
                "⚠️  {} files from this session no longer exist in the project",
                files_missing.len()
            ));
            recommendations
                .push("🔍 Analysis is incomplete - some files could not be examined".to_string());
        }

        if detections.is_empty() && files_analyzed > 0 {
            recommendations.push(format!(
                "✅ No bullshit patterns detected in {files_analyzed} analyzed files!"
            ));
            if !files_missing.is_empty() {
                recommendations
                    .push("⚠️  However, analysis is incomplete due to missing files".to_string());
            }
            return recommendations;
        }

        // Count by pattern type
        let mut unimplemented_count = 0;
        let mut todo_count = 0;
        let mut unwrap_count = 0;
        let mut panic_count = 0;

        for detection in detections {
            match detection.rule_id.as_str() {
                id if id.contains("unimplemented") => unimplemented_count += 1,
                id if id.contains("todo") => todo_count += 1,
                id if id.contains("unwrap") => unwrap_count += 1,
                id if id.contains("panic") => panic_count += 1,
                _ => {}
            }
        }

        if unimplemented_count > 0 {
            recommendations.push(format!(
                "🚨 CRITICAL: {unimplemented_count} functions use unimplemented!() - these need actual implementations"
            ));
        }

        if panic_count > 0 {
            recommendations.push(format!(
                "🔴 HIGH: {panic_count} panic!() calls with TODO messages - replace with proper error handling"
            ));
        }

        if todo_count > 0 {
            recommendations.push(format!(
                "🟡 MEDIUM: {todo_count} TODO/FIXME comments found - these indicate incomplete work"
            ));
        }

        if unwrap_count > 0 {
            recommendations.push(format!(
                "🟡 MEDIUM: {unwrap_count} .unwrap() calls without context - add proper error handling"
            ));
        }

        // Overall recommendation
        if detections.len() > 10 {
            recommendations.push("📝 Consider reviewing the TODO list - many patterns suggest incomplete implementations".to_string());
        }

        recommendations
    }

    /// Analyzes a Claude Code project directory.
    pub fn analyze_project_directory(
        &mut self,
        project_dir: &Path,
    ) -> Result<Vec<SimpleSessionAnalysis>> {
        let mut results = Vec::new();

        // Look for session files in the project directory
        if let Ok(entries) = std::fs::read_dir(project_dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                        match self.analyze_session(&path) {
                            Ok(analysis) => results.push(analysis),
                            Err(e) => {
                                eprintln!("Warning: Failed to analyze {}: {}", path.display(), e);
                            }
                        }
                    }
                }
            }
        }

        Ok(results)
    }
}

impl Default for SimpleSessionAnalyzer {
    fn default() -> Self {
        Self::new().expect("Failed to create SimpleSessionAnalyzer")
    }
}
