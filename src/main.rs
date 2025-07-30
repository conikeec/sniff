// Copyright (c) 2025 Chetan Conikee <conikee@gmail.com>
// Licensed under the MIT License

//! Sniff CLI - Code Quality Analysis and AI Deception Detection

#![allow(clippy::manual_flatten)]

use clap::{Parser, Subcommand, ValueEnum};
use sniff::{Result, SniffError};
use std::path::PathBuf;
use tracing::{info, warn, Level};
use tracing_subscriber::fmt;

/// Sniff CLI - Code Quality Analysis and AI Deception Detection
#[derive(Parser)]
#[command(
    name = "sniff",
    version = env!("CARGO_PKG_VERSION"),
    author = "Chetan Conikee <conikee@gmail.com>",
    about = "Code quality analysis tool that detects AI-generated deception patterns and provides quality gates"
)]
struct Cli {
    /// Enable verbose logging (use multiple times for more verbosity)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Subcommand to execute
    #[command(subcommand)]
    command: Commands,
}

/// Output format for commands
#[derive(ValueEnum, Clone, Debug, PartialEq)]
enum OutputFormat {
    /// Human-readable table format
    Table,
    /// JSON format
    Json,
    /// Markdown format
    Markdown,
    /// Compact one-line format
    Compact,
}

/// Available CLI commands.
#[derive(Subcommand)]
enum Commands {
    /// Analyze files for code quality issues and misalignment patterns
    AnalyzeFiles {
        /// Files or directories to analyze
        #[arg(required = true)]
        paths: Vec<PathBuf>,
        /// Output format
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,
        /// Show detailed pattern analysis
        #[arg(short, long)]
        detailed: bool,
        /// Include hidden files and directories
        #[arg(long)]
        include_hidden: bool,
        /// File extensions to include (e.g., rs,py,ts)
        #[arg(long)]
        extensions: Option<String>,
        /// Pattern to exclude files (glob pattern)
        #[arg(long)]
        exclude: Option<String>,
        /// Maximum file size to analyze (in MB)
        #[arg(long, default_value = "10")]
        max_file_size_mb: f64,
        /// Language to force for all files (overrides detection)
        #[arg(long)]
        force_language: Option<String>,
        /// Save analysis results to file
        #[arg(long)]
        output_file: Option<PathBuf>,
        /// Create checkpoint for tracking changes
        #[arg(long)]
        checkpoint: Option<String>,
        /// Compare against previous checkpoint
        #[arg(long)]
        diff_checkpoint: Option<String>,
        /// Include test files in analysis (by default test files are excluded)
        #[arg(long)]
        include_tests: bool,
        /// Confidence threshold for test file detection (0.0-1.0)
        #[arg(long, default_value = "0.3")]
        test_confidence: f64,
    },

    /// Manage analysis checkpoints for tracking changes over time
    Checkpoint {
        #[command(subcommand)]
        command: CheckpointCommands,
    },

    /// Manage learned patterns for dynamic misalignment detection
    Patterns {
        #[command(subcommand)]
        command: PatternCommands,
    },

    /// Verify TODO completion with sniff analysis
    VerifyTodo {
        /// TODO ID to verify
        #[arg(short, long)]
        todo_id: String,
        /// Files to analyze for this TODO
        #[arg(short, long)]
        files: Vec<PathBuf>,
        /// Minimum quality score required (0-100)
        #[arg(long, default_value = "80")]
        min_quality_score: f64,
        /// Maximum critical issues allowed
        #[arg(long, default_value = "0")]
        max_critical_issues: usize,
        /// Output format
        #[arg(long, default_value = "table")]
        format: OutputFormat,
        /// Use Git to discover changed files (prevents agent deception)
        #[arg(long)]
        git_discovery: bool,
    },
}

/// Checkpoint management commands
#[derive(Subcommand)]
enum CheckpointCommands {
    /// Create a new checkpoint with current file states
    Create {
        /// Checkpoint name
        #[arg(short, long)]
        name: String,
        /// Files or directories to checkpoint
        paths: Vec<PathBuf>,
        /// Description of the checkpoint
        #[arg(short, long)]
        description: Option<String>,
    },
    /// List available checkpoints
    List {
        /// Output format
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,
    },
    /// Show detailed information about a checkpoint
    Show {
        /// Checkpoint name
        name: String,
        /// Output format
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,
    },
    /// Compare current state against a checkpoint
    Diff {
        /// Checkpoint name to compare against
        checkpoint: String,
        /// Paths to compare (optional, uses checkpoint paths if not provided)
        paths: Option<Vec<PathBuf>>,
        /// Output format
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,
    },
    /// Delete a checkpoint
    Delete {
        /// Checkpoint name
        name: String,
        /// Confirm the deletion
        #[arg(long)]
        confirm: bool,
    },
}

/// Pattern management commands
#[derive(Subcommand)]
enum PatternCommands {
    /// Create a new learned pattern
    Create {
        /// Programming language for the pattern
        #[arg(short, long)]
        language: String,
        /// Pattern name
        #[arg(short, long)]
        name: String,
        /// Pattern description
        #[arg(short, long)]
        description: String,
        /// Regex pattern to match
        #[arg(short, long)]
        pattern: String,
        /// Pattern severity (info, low, medium, high, critical)
        #[arg(short, long, default_value = "medium")]
        severity: String,
        /// Pattern scope (file, function_body, class_body, comments, method_signature)
        #[arg(long, default_value = "function_body")]
        scope: String,
        /// Optional regex flags
        #[arg(long)]
        flags: Option<String>,
        /// Confidence in this pattern (0.0-1.0)
        #[arg(short, long, default_value = "0.8")]
        confidence: f64,
        /// Tags for categorization (comma-separated)
        #[arg(short, long)]
        tags: Option<String>,
        /// Example code that should trigger this pattern
        #[arg(long)]
        examples: Vec<String>,
        /// Example code that should NOT trigger this pattern
        #[arg(long)]
        false_positives: Vec<String>,
    },
    /// List learned patterns
    List {
        /// Filter by programming language
        #[arg(short, long)]
        language: Option<String>,
        /// Output format
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,
        /// Show only active patterns
        #[arg(short, long)]
        active_only: bool,
    },
    /// Show pattern statistics
    Stats {
        /// Output format
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,
    },
    /// Delete a learned pattern
    Delete {
        /// Pattern ID to delete
        pattern_id: String,
        /// Confirm the deletion
        #[arg(long)]
        confirm: bool,
    },
    /// Initialize .sniff folder structure
    Init {
        /// Force initialization even if .sniff already exists
        #[arg(short, long)]
        force: bool,
    },
    /// Export learned patterns to YAML
    Export {
        /// Programming language to export
        #[arg(short, long)]
        language: String,
        /// Output file path
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Validate existing learned patterns
    Validate {
        /// Fix invalid patterns automatically
        #[arg(long)]
        fix: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging based on verbosity level
    let log_level = match cli.verbose {
        0 => Level::WARN,
        1 => Level::INFO,
        2 => Level::DEBUG,
        _ => Level::TRACE,
    };
    fmt().with_max_level(log_level).with_target(false).init();

    info!("Starting Sniff CLI v{}", env!("CARGO_PKG_VERSION"));

    // Execute the selected command
    match cli.command {
        Commands::AnalyzeFiles {
            paths,
            format,
            detailed,
            include_hidden,
            extensions,
            exclude,
            max_file_size_mb,
            force_language,
            output_file,
            checkpoint,
            diff_checkpoint,
            include_tests,
            test_confidence,
        } => {
            handle_analyze_files_command(
                paths,
                format,
                detailed,
                include_hidden,
                extensions,
                exclude,
                max_file_size_mb,
                force_language,
                output_file,
                checkpoint,
                diff_checkpoint,
                include_tests,
                test_confidence,
            )
            .await
        }

        Commands::Checkpoint { command } => handle_checkpoint_command(command).await,

        Commands::Patterns { command } => handle_patterns_command(command).await,

        Commands::VerifyTodo {
            todo_id,
            files,
            min_quality_score,
            max_critical_issues,
            format,
            git_discovery,
        } => {
            handle_verify_todo_command(todo_id, files, min_quality_score, max_critical_issues, format, git_discovery)
                .await
        }
    }
}

// Keep only the modern command handlers from the original main.rs
// These will be copied from the original file...

/// Handles the analyze-files command - analyzes arbitrary files for misalignment patterns.
async fn handle_analyze_files_command(
    paths: Vec<PathBuf>,
    format: OutputFormat,
    detailed: bool,
    include_hidden: bool,
    extensions: Option<String>,
    exclude: Option<String>,
    max_file_size_mb: f64,
    force_language: Option<String>,
    output_file: Option<PathBuf>,
    checkpoint: Option<String>,
    diff_checkpoint: Option<String>,
    include_tests: bool,
    test_confidence: f64,
) -> Result<()> {
    use sniff::analysis::MisalignmentAnalyzer;
    use sniff::standalone::{AnalysisConfig, CheckpointManager, FileFilter, StandaloneAnalyzer};

    info!("🕵️  Starting standalone file analysis");

    // Configure file filter
    let allowed_extensions = extensions.map(|ext| {
        ext.split(',')
            .map(|e| e.trim().to_string())
            .collect::<Vec<_>>()
    });

    let filter = FileFilter {
        include_hidden,
        allowed_extensions,
        exclude_pattern: exclude,
        max_file_size_bytes: (max_file_size_mb * 1024.0 * 1024.0) as u64,
        include_test_files: include_tests,
        test_confidence_threshold: test_confidence,
    };

    // Create analysis config
    let config = AnalysisConfig {
        filter,
        force_language: force_language.and_then(|lang| match lang.to_lowercase().as_str() {
            "rust" => Some(sniff::SupportedLanguage::Rust),
            "python" => Some(sniff::SupportedLanguage::Python),
            "typescript" => Some(sniff::SupportedLanguage::TypeScript),
            "javascript" => Some(sniff::SupportedLanguage::JavaScript),
            "go" => Some(sniff::SupportedLanguage::Go),
            "c" => Some(sniff::SupportedLanguage::C),
            "cpp" => Some(sniff::SupportedLanguage::Cpp),
            _ => {
                warn!("Unknown language '{}', will auto-detect", lang);
                None
            }
        }),
        detailed_analysis: detailed,
    };

    // Initialize analyzer with learned patterns and playbooks
    let current_dir = std::env::current_dir().map_err(|e| SniffError::file_system(".", e))?;
    let mut misalignment_analyzer = match MisalignmentAnalyzer::new_with_learned_patterns(&current_dir) {
        Ok(analyzer) => analyzer,
        Err(e) => {
            warn!("Failed to load learned patterns: {}, falling back to default patterns", e);
            MisalignmentAnalyzer::new()?
        }
    };
    
    // Also load playbooks from the playbooks directory
    let playbook_dir = current_dir.join("playbooks");
    if playbook_dir.exists() {
        if let Err(e) = misalignment_analyzer.load_playbooks(&playbook_dir) {
            warn!("Failed to load playbooks from {}: {}", playbook_dir.display(), e);
        } else {
            info!("Loaded enhanced playbooks from {}", playbook_dir.display());
        }
    }
    let mut analyzer = StandaloneAnalyzer::new(misalignment_analyzer, config);

    // Handle checkpoint comparison if requested
    if let Some(checkpoint_name) = diff_checkpoint {
        let current_dir = std::env::current_dir().map_err(|e| SniffError::file_system(".", e))?;
        let checkpoint_manager = CheckpointManager::new(&current_dir)?;

        info!("📊 Comparing against checkpoint: {}", checkpoint_name);
        let comparison = checkpoint_manager
            .compare_files(&checkpoint_name, &paths)
            .await?;

        // Analyze only changed files
        let changed_files: Vec<PathBuf> = comparison
            .changed_files
            .clone()
            .into_iter()
            .chain(comparison.new_files.clone().into_iter())
            .collect();

        if changed_files.is_empty() {
            println!(
                "✅ No changes detected since checkpoint '{}'",
                checkpoint_name
            );
            return Ok(());
        }

        println!(
            "📁 Analyzing {} changed files since checkpoint '{}'",
            changed_files.len(),
            checkpoint_name
        );

        let results = analyzer.analyze_files(&changed_files).await?;
        display_standalone_results(&results, format, detailed, Some(&comparison))?;
    } else {
        // Analyze specified files/directories
        let results = analyzer.analyze_files(&paths).await?;

        // Create checkpoint if requested
        if let Some(checkpoint_name) = checkpoint {
            let current_dir =
                std::env::current_dir().map_err(|e| SniffError::file_system(".", e))?;
            let checkpoint_manager = CheckpointManager::new(&current_dir)?;

            info!("📸 Creating checkpoint: {}", checkpoint_name);
            checkpoint_manager
                .create_checkpoint(&checkpoint_name, &paths, None)
                .await?;
            println!("✅ Checkpoint '{}' created", checkpoint_name);
        }

        display_standalone_results(&results, format, detailed, None)?;
    }

    // Save results to file if requested
    if let Some(output_path) = output_file {
        // Implement result serialization
        info!("💾 Saving results to: {}", output_path.display());
        // This would serialize the results in the requested format
        println!("💡 Result saving not yet implemented");
    }

    Ok(())
}

// Additional modern command handlers would go here...
// These need to be copied from the original main.rs file

/// Displays standalone analysis results.
fn display_standalone_results(
    results: &sniff::standalone::AnalysisResults,
    format: OutputFormat,
    detailed: bool,
    comparison: Option<&sniff::standalone::FileComparison>,
) -> Result<()> {
    match format {
        OutputFormat::Table => {
            println!("🕵️  Standalone File Analysis Results");
            println!("══════════════════════════════════════");
            println!();

            if let Some(comp) = comparison {
                println!("📊 Change Summary:");
                println!("   New files: {}", comp.new_files.len());
                println!("   Modified files: {}", comp.changed_files.len());
                println!("   Deleted files: {}", comp.deleted_files.len());
                println!();
            }

            println!("📈 Analysis Summary:");
            println!("   Files analyzed: {}", results.total_files);
            println!("   Total patterns: {}", results.total_detections);
            println!("   Critical issues: {}", results.critical_issues);
            println!("   Average quality: {:.1}%", results.average_quality_score);
            println!();

            if !results.file_results.is_empty() {
                println!("📄 File Analysis:");
                for file_result in &results.file_results {
                    if !file_result.detections.is_empty() {
                        println!(
                            "   {} ({})",
                            file_result.file_path.display(),
                            file_result.language.map(|l| l.name()).unwrap_or("unknown")
                        );
                        println!(
                            "      Issues: {} | Quality: {:.1}%",
                            file_result.detections.len(),
                            file_result.quality_score
                        );

                        if detailed {
                            for detection in &file_result.detections {
                                println!(
                                    "         {} {} ({}:{}): {}",
                                    detection.severity.emoji(),
                                    detection.rule_name,
                                    detection.file_path,
                                    detection.line_number,
                                    detection.code_snippet.trim()
                                );
                            }
                        }
                        println!();
                    }
                }
            }

            if results.critical_issues > 0 {
                println!(
                    "🚨 {} critical issues detected that require immediate attention",
                    results.critical_issues
                );
            } else if results.total_detections == 0 {
                println!("✅ No issues detected! Code quality looks excellent.");
            }
        }

        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(results)?);
        }

        OutputFormat::Markdown => {
            println!("# 🕵️ Standalone File Analysis Results");
            println!();
            println!("## Summary");
            println!();
            println!("| Metric | Value |");
            println!("| ------ | ----- |");
            println!("| Files analyzed | {} |", results.total_files);
            println!("| Total patterns | {} |", results.total_detections);
            println!("| Critical issues | {} |", results.critical_issues);
            println!(
                "| Average quality | {:.1}% |",
                results.average_quality_score
            );
            println!();

            if !results.file_results.is_empty() {
                println!("## File Analysis");
                println!();
                for file_result in &results.file_results {
                    if !file_result.detections.is_empty() {
                        println!("### `{}`", file_result.file_path.display());
                        println!();
                        println!(
                            "- **Language**: {}",
                            file_result.language.map(|l| l.name()).unwrap_or("unknown")
                        );
                        println!("- **Issues**: {}", file_result.detections.len());
                        println!("- **Quality**: {:.1}%", file_result.quality_score);
                        println!();

                        if detailed {
                            println!("#### Issues");
                            println!();
                            for detection in &file_result.detections {
                                println!(
                                    "- {} **{}** (line {}): `{}`",
                                    detection.severity.emoji(),
                                    detection.rule_name,
                                    detection.line_number,
                                    detection.code_snippet.trim()
                                );
                            }
                            println!();
                        }
                    }
                }
            }
        }

        OutputFormat::Compact => {
            for file_result in &results.file_results {
                if !file_result.detections.is_empty() {
                    println!(
                        "{}: {} issues, {:.1}% quality",
                        file_result.file_path.display(),
                        file_result.detections.len(),
                        file_result.quality_score
                    );
                }
            }
        }
    }

    Ok(())
}

// Modern command handlers (copied from legacy main.rs)

/// Handles checkpoint management commands.
async fn handle_checkpoint_command(command: CheckpointCommands) -> Result<()> {
    use sniff::standalone::CheckpointManager;

    let current_dir = std::env::current_dir().map_err(|e| SniffError::file_system(".", e))?;
    let checkpoint_manager = CheckpointManager::new(&current_dir)?;

    match command {
        CheckpointCommands::Create {
            name,
            paths,
            description,
        } => {
            info!("📸 Creating checkpoint: {}", name);
            checkpoint_manager
                .create_checkpoint(&name, &paths, description)
                .await?;
            println!(
                "✅ Checkpoint '{}' created with {} files",
                name,
                paths.len()
            );
        }

        CheckpointCommands::List { format } => {
            let checkpoints = checkpoint_manager.list_checkpoints().await?;

            if checkpoints.is_empty() {
                println!("📝 No checkpoints found");
                return Ok(());
            }

            match format {
                OutputFormat::Table => {
                    println!("📸 Available Checkpoints");
                    println!("════════════════════════");
                    println!();

                    for checkpoint in checkpoints {
                        println!("🏷️  {}", checkpoint.name);
                        println!(
                            "   Created: {}",
                            checkpoint.timestamp.format("%Y-%m-%d %H:%M:%S")
                        );
                        println!("   Files: {}", checkpoint.file_count);
                        if let Some(desc) = checkpoint.description {
                            println!("   Description: {}", desc);
                        }
                        println!();
                    }
                }
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&checkpoints)?);
                }
                _ => {
                    for checkpoint in checkpoints {
                        println!(
                            "{}: {} files ({})",
                            checkpoint.name,
                            checkpoint.file_count,
                            checkpoint.timestamp.format("%Y-%m-%d %H:%M")
                        );
                    }
                }
            }
        }

        CheckpointCommands::Show { name, format: _ } => {
            if let Some(checkpoint) = checkpoint_manager.get_checkpoint(&name).await? {
                println!("📸 Checkpoint: {}", checkpoint.name);
                println!(
                    "Created: {}",
                    checkpoint.timestamp.format("%Y-%m-%d %H:%M:%S")
                );
                println!("Files: {}", checkpoint.file_count);
                if let Some(desc) = checkpoint.description {
                    println!("Description: {}", desc);
                }
                // Show file list
                let details = checkpoint_manager.get_checkpoint_files(&name).await?;
                println!("\nFiles in checkpoint:");
                for file_info in details {
                    println!("  {} ({})", file_info.path.display(), file_info.file_size);
                }
            } else {
                println!("❌ Checkpoint '{}' not found", name);
            }
        }

        CheckpointCommands::Diff {
            checkpoint,
            paths,
            format,
        } => {
            let comparison_paths = paths.unwrap_or_else(|| {
                // Get paths from checkpoint if not provided
                vec![std::env::current_dir().unwrap()]
            });

            let comparison = checkpoint_manager
                .compare_files(&checkpoint, &comparison_paths)
                .await?;

            match format {
                OutputFormat::Table => {
                    println!("📊 Changes since checkpoint '{}'", checkpoint);
                    println!("═══════════════════════════════════");
                    println!();

                    if !comparison.new_files.is_empty() {
                        println!("📄 New files ({}): ", comparison.new_files.len());
                        for file in &comparison.new_files {
                            println!("  + {}", file.display());
                        }
                        println!();
                    }

                    if !comparison.changed_files.is_empty() {
                        println!("📝 Modified files ({}): ", comparison.changed_files.len());
                        for file in &comparison.changed_files {
                            println!("  ~ {}", file.display());
                        }
                        println!();
                    }

                    if !comparison.deleted_files.is_empty() {
                        println!("🗑️  Deleted files ({}): ", comparison.deleted_files.len());
                        for file in &comparison.deleted_files {
                            println!("  - {}", file.display());
                        }
                        println!();
                    }

                    if comparison.new_files.is_empty()
                        && comparison.changed_files.is_empty()
                        && comparison.deleted_files.is_empty()
                    {
                        println!("✅ No changes detected since checkpoint");
                    }
                }
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&comparison)?);
                }
                _ => {
                    println!(
                        "Changes: +{} ~{} -{}",
                        comparison.new_files.len(),
                        comparison.changed_files.len(),
                        comparison.deleted_files.len()
                    );
                }
            }
        }

        CheckpointCommands::Delete { name, confirm } => {
            if !confirm {
                println!("❌ Checkpoint deletion requires --confirm flag for safety");
                return Ok(());
            }

            checkpoint_manager.delete_checkpoint(&name).await?;
            println!("✅ Checkpoint '{}' deleted", name);
        }
    }

    Ok(())
}

/// Handles pattern management commands.
async fn handle_patterns_command(command: PatternCommands) -> Result<()> {
    // Simplified implementation - pattern management functionality is available
    // but the full implementation needs API updates

    match command {
        PatternCommands::Init { force: _ } => {
            println!("📋 Pattern initialization available in playbooks/ directory");
            println!("💡 Sniff uses YAML playbooks for pattern definitions");
        }
        _ => {
            println!("📋 Pattern management commands simplified in streamlined version");
            println!("💡 Use playbooks/ directory for pattern definitions");
            println!("🔍 Available patterns are loaded automatically during analysis");
        }
    }

    Ok(())
}

/// Handles the verify-todo command - verifies TODO completion with sniff analysis.
async fn handle_verify_todo_command(
    todo_id: String,
    files: Vec<PathBuf>,
    min_quality_score: f64,
    max_critical_issues: usize,
    format: OutputFormat,
    git_discovery: bool,
) -> Result<()> {
    use sniff::verify_todo::{verify_todo, display_verification_result, VerificationConfig};

    let config = VerificationConfig {
        min_quality_score,
        max_critical_issues,
        include_test_files: false, // Exclude test files by default for quality verification
    };

    // Use git discovery if requested, otherwise use provided files
    let actual_files = if git_discovery {
        match sniff::verify_todo::discover_git_changes() {
            Ok(git_files) => {
                if git_files != files {
                    println!("Git discovery found {} files vs {} reported", git_files.len(), files.len());
                    println!("Using git-discovered files for verification");
                }
                git_files
            }
            Err(e) => {
                eprintln!("Git discovery failed: {}, using reported files", e);
                files
            }
        }
    } else {
        files
    };

    let result = verify_todo(&todo_id, &actual_files, config.clone()).await?;

    match format {
        OutputFormat::Json => {
            let verification_result = serde_json::json!({
                "todo_id": todo_id,
                "verification_passed": result.passed,
                "quality_score": result.quality_score,
                "min_quality_required": config.min_quality_score,
                "critical_issues": result.critical_issues,
                "max_critical_allowed": config.max_critical_issues,
                "analysis_results": result.analysis_results
            });
            println!("{}", serde_json::to_string_pretty(&verification_result)?);
        }
        _ => {
            display_verification_result(&todo_id, &result, &config);
        }
    }

    if result.passed {
        Ok(())
    } else {
        Err(SniffError::analysis_error(format!(
            "TODO '{}' failed verification: quality {:.1}% < {:.1}%, critical issues {} > {}",
            todo_id, result.quality_score, config.min_quality_score, 
            result.critical_issues, config.max_critical_issues
        )))
    }
}
