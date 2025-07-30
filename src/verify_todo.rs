// Copyright (c) 2025 Chetan Conikee <conikee@gmail.com>
// Licensed under the MIT License

//! TODO verification functionality with sniff analysis integration.

use crate::analysis::MisalignmentAnalyzer;
use crate::error::{Result, SniffError};
use crate::standalone::{AnalysisConfig, FileFilter, StandaloneAnalyzer};
use std::path::PathBuf;
use std::process::Command;
use tracing::{info, warn};

/// Configuration for TODO verification.
#[derive(Debug, Clone)]
pub struct VerificationConfig {
    /// Minimum quality score required (0-100).
    pub min_quality_score: f64,
    /// Maximum critical issues allowed.
    pub max_critical_issues: usize,
    /// Whether to include test files in verification.
    pub include_test_files: bool,
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            min_quality_score: 80.0,
            max_critical_issues: 0,
            include_test_files: false,
        }
    }
}

/// Result of TODO verification.
#[derive(Debug)]
pub struct VerificationResult {
    /// Whether verification passed.
    pub passed: bool,
    /// Quality score achieved.
    pub quality_score: f64,
    /// Number of critical issues found.
    pub critical_issues: usize,
    /// Total number of detections.
    pub total_detections: usize,
    /// Files that were analyzed.
    pub files_analyzed: usize,
    /// Detailed analysis results.
    pub analysis_results: crate::standalone::AnalysisResults,
}

/// Verifies TODO completion with sniff analysis.
pub async fn verify_todo(
    todo_id: &str,
    files: &[PathBuf],
    config: VerificationConfig,
) -> Result<VerificationResult> {
    // Detect potential file hiding by comparing reported vs git changes  
    if let Ok(git_files) = discover_git_changes() {
        let reported_files: std::collections::HashSet<_> = files.iter().collect();
        let git_files_set: std::collections::HashSet<_> = git_files.iter().collect();
        
        let hidden_files: Vec<_> = git_files_set.difference(&reported_files).collect();
        if !hidden_files.is_empty() {
            warn!("Potential file hiding detected! Git shows additional changes:");
            for file in &hidden_files {
                warn!("  Hidden file: {}", file.display());
            }
            warn!("Including hidden files in verification for security");
        }
        
        // Use git-discovered files instead of reported files
        return verify_todo_with_files(todo_id, &git_files, config).await;
    }
    info!("🔍 Verifying TODO '{}' with sniff analysis", todo_id);

    if files.is_empty() {
        return Ok(VerificationResult {
            passed: true,
            quality_score: 100.0,
            critical_issues: 0,
            total_detections: 0,
            files_analyzed: 0,
            analysis_results: crate::standalone::AnalysisResults {
                total_files: 0,
                total_detections: 0,
                critical_issues: 0,
                average_quality_score: 100.0,
                file_results: Vec::new(),
            },
        });
    }

    // Configure analyzer for verification
    let filter = FileFilter {
        include_hidden: false,
        allowed_extensions: None,
        exclude_pattern: None,
        max_file_size_bytes: 10 * 1024 * 1024, // 10MB
        include_test_files: config.include_test_files,
        test_confidence_threshold: 0.3,
    };

    let analysis_config = AnalysisConfig {
        filter,
        force_language: None,
        detailed_analysis: true,
    };

    // Initialize analyzer with learned patterns
    let current_dir = std::env::current_dir().map_err(|e| SniffError::file_system(".", e))?;
    let mut misalignment_analyzer = match MisalignmentAnalyzer::new_with_learned_patterns(&current_dir) {
        Ok(analyzer) => analyzer,
        Err(e) => {
            warn!("Failed to load learned patterns: {}, using default patterns", e);
            MisalignmentAnalyzer::new()?
        }
    };
    
    // Load playbooks
    let playbook_dir = current_dir.join("playbooks");
    if playbook_dir.exists() {
        if let Err(e) = misalignment_analyzer.load_playbooks(&playbook_dir) {
            warn!("Failed to load playbooks: {}", e);
        }
    }

    let mut analyzer = StandaloneAnalyzer::new(misalignment_analyzer, analysis_config);

    // Analyze the files
    let results = analyzer.analyze_files(files).await?;

    // Check quality gate
    let quality_passed = results.average_quality_score >= config.min_quality_score;
    let critical_passed = results.critical_issues <= config.max_critical_issues;
    let verification_passed = quality_passed && critical_passed;

    Ok(VerificationResult {
        passed: verification_passed,
        quality_score: results.average_quality_score,
        critical_issues: results.critical_issues,
        total_detections: results.total_detections,
        files_analyzed: results.total_files,
        analysis_results: results,
    })
}

/// Displays verification results in a human-readable format.
pub fn display_verification_result(
    todo_id: &str,
    result: &VerificationResult,
    config: &VerificationConfig,
) {
    use colored::Colorize;

    // Header
    println!("{}", "TODO Verification Report".bold().cyan());
    println!("{}", "─".repeat(50).dimmed());
    
    // Task ID with tree structure
    println!("├─ {}: {}", "TODO".dimmed(), todo_id.bold());
    
    // Metrics tree
    println!("├─ {}", "Metrics".dimmed());
    println!("│  ├─ Files analyzed: {}", result.files_analyzed.to_string().cyan());
    
    // Quality score with color coding
    let quality_color = if result.quality_score >= config.min_quality_score {
        result.quality_score.to_string().green()
    } else {
        result.quality_score.to_string().red()
    };
    println!("│  ├─ Quality score: {}% (required: {}%)", 
        quality_color, 
        config.min_quality_score.to_string().dimmed()
    );
    
    // Critical issues with color coding
    let critical_color = if result.critical_issues <= config.max_critical_issues {
        result.critical_issues.to_string().green()
    } else {
        result.critical_issues.to_string().red()
    };
    println!("│  ├─ Critical issues: {} (max allowed: {})", 
        critical_color,
        config.max_critical_issues.to_string().dimmed()
    );
    
    println!("│  └─ Total detections: {}", result.total_detections.to_string().yellow());
    
    // Verification result
    println!("├─ {}", "Result".dimmed());
    if result.passed {
        println!("│  └─ {} {}", "●".green().bold(), "PASSED - Ready to mark complete".green());
    } else {
        println!("│  └─ {} {}", "●".red().bold(), "FAILED - Continue working on this TODO".red());
        
        // Show failure reasons
        if result.quality_score < config.min_quality_score {
            println!("│     ├─ {} Quality score {:.1}% below required {:.1}%", 
                "⚠".yellow(), 
                result.quality_score, 
                config.min_quality_score
            );
        }
        if result.critical_issues > config.max_critical_issues {
            println!("│     └─ {} {} critical issues found (max allowed: {})", 
                "⚠".yellow(),
                result.critical_issues, 
                config.max_critical_issues
            );
        }
    }

    // Show detailed issues if verification failed
    if !result.passed && !result.analysis_results.file_results.is_empty() {
        println!("└─ {}", "Issues Found".dimmed());
        
        for (file_idx, file_result) in result.analysis_results.file_results.iter().enumerate() {
            if !file_result.detections.is_empty() {
                let is_last_file = file_idx == result.analysis_results.file_results.len() - 1;
                let file_prefix = if is_last_file { "└─" } else { "├─" };
                let issue_prefix = if is_last_file { "   " } else { "│  " };
                
                println!("   {} {} (Quality: {:.1}%)", 
                    file_prefix,
                    file_result.file_path.display().to_string().cyan(),
                    file_result.quality_score.to_string().dimmed()
                );
                
                let issues_to_show = file_result.detections.iter().take(3);
                let total_issues = file_result.detections.len();
                
                for (issue_idx, detection) in issues_to_show.enumerate() {
                    let is_last_issue = issue_idx == std::cmp::min(3, total_issues) - 1 && total_issues <= 3;
                    let detection_prefix = if is_last_issue { "└─" } else { "├─" };
                    
                    let severity_icon = match format!("{:?}", detection.severity).to_lowercase().as_str() {
                        "critical" => "●".red().bold(),
                        "high" => "●".red(),
                        "medium" => "●".yellow(),
                        "low" => "●".blue(),
                        _ => "●".white(),
                    };
                    
                    let code_snippet = detection.code_snippet.trim().chars().take(50).collect::<String>();
                    
                    println!("   {}   {} {} {} (line {}): {}", 
                        issue_prefix,
                        detection_prefix,
                        severity_icon,
                        detection.rule_name.bold(),
                        detection.line_number.to_string().dimmed(),
                        code_snippet.italic()
                    );
                }
                
                if total_issues > 3 {
                    println!("   {}   └─ {} ... and {} more issues", 
                        issue_prefix,
                        "●".dimmed(),
                        (total_issues - 3).to_string().dimmed()
                    );
                }
            }
        }
    }
}

/// Discover file changes using Git to prevent agent deception.
pub fn discover_git_changes() -> Result<Vec<PathBuf>> {
    let mut all_files = Vec::new();
    
    // 1. Working directory changes (modified, not staged)
    if let Ok(output) = Command::new("git")
        .args(["diff", "--name-only"])
        .output()
    {
        if output.status.success() {
            let files = parse_git_output(&output.stdout)?;
            all_files.extend(files);
        }
    }
    
    // 2. Staged changes (ready for commit)
    if let Ok(output) = Command::new("git")
        .args(["diff", "--cached", "--name-only"])
        .output()
    {
        if output.status.success() {
            let files = parse_git_output(&output.stdout)?;
            all_files.extend(files);
        }
    }
    
    // 3. Recent commits (potential work session)
    if let Ok(output) = Command::new("git")
        .args(["diff", "HEAD~3", "--name-only"])
        .output()
    {
        if output.status.success() {
            let files = parse_git_output(&output.stdout)?;
            all_files.extend(files);
        }
    }
    
    // 4. Untracked files that match code patterns
    if let Ok(output) = Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard"])
        .output()
    {
        if output.status.success() {
            let files = parse_git_output(&output.stdout)?;
            let code_files: Vec<_> = files
                .into_iter()
                .filter(|f| is_code_file(f))
                .collect();
            all_files.extend(code_files);
        }
    }
    
    // Remove duplicates and filter for existing files
    all_files.sort();
    all_files.dedup();
    all_files.retain(|f| f.exists());
    
    Ok(all_files)
}

/// Parse git command output into file paths.
fn parse_git_output(output: &[u8]) -> Result<Vec<PathBuf>> {
    let output_str = String::from_utf8_lossy(output);
    Ok(output_str
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(PathBuf::from)
        .collect())
}

/// Check if file is a code file that should be analyzed.
fn is_code_file(path: &PathBuf) -> bool {
    if let Some(ext) = path.extension() {
        matches!(
            ext.to_string_lossy().as_ref(),
            "rs" | "py" | "ts" | "js" | "jsx" | "tsx" | "go" | "c" | "cpp" | "h" | "hpp"
        )
    } else {
        false
    }
}

/// Verify TODO with explicitly provided files (used after git discovery).
async fn verify_todo_with_files(
    todo_id: &str,
    files: &[PathBuf],
    config: VerificationConfig,
) -> Result<VerificationResult> {
    info!("Verifying TODO '{}' with {} files", todo_id, files.len());

    if files.is_empty() {
        return Ok(VerificationResult {
            passed: true,
            quality_score: 100.0,
            critical_issues: 0,
            total_detections: 0,
            files_analyzed: 0,
            analysis_results: crate::standalone::AnalysisResults {
                total_files: 0,
                total_detections: 0,
                critical_issues: 0,
                average_quality_score: 100.0,
                file_results: Vec::new(),
            },
        });
    }

    // Configure analyzer for verification
    let filter = FileFilter {
        include_hidden: false,
        allowed_extensions: None,
        exclude_pattern: None,
        max_file_size_bytes: 10 * 1024 * 1024, // 10MB
        include_test_files: config.include_test_files,
        test_confidence_threshold: 0.3,
    };

    let analysis_config = AnalysisConfig {
        filter,
        force_language: None,
        detailed_analysis: true,
    };

    // Initialize analyzer with learned patterns
    let current_dir = std::env::current_dir().map_err(|e| SniffError::file_system(".", e))?;
    let mut misalignment_analyzer = match MisalignmentAnalyzer::new_with_learned_patterns(&current_dir) {
        Ok(analyzer) => analyzer,
        Err(e) => {
            warn!("Failed to load learned patterns: {}, using default patterns", e);
            MisalignmentAnalyzer::new()?
        }
    };
    
    // Load playbooks
    let playbook_dir = current_dir.join("playbooks");
    if playbook_dir.exists() {
        if let Err(e) = misalignment_analyzer.load_playbooks(&playbook_dir) {
            warn!("Failed to load playbooks: {}", e);
        }
    }

    let mut analyzer = StandaloneAnalyzer::new(misalignment_analyzer, analysis_config);

    // Analyze the files
    let results = analyzer.analyze_files(files).await?;

    // Check quality gate
    let quality_passed = results.average_quality_score >= config.min_quality_score;
    let critical_passed = results.critical_issues <= config.max_critical_issues;
    let verification_passed = quality_passed && critical_passed;

    Ok(VerificationResult {
        passed: verification_passed,
        quality_score: results.average_quality_score,
        critical_issues: results.critical_issues,
        total_detections: results.total_detections,
        files_analyzed: results.total_files,
        analysis_results: results,
    })
}
