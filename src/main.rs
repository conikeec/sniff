// Copyright (c) 2025 Chetan Conikee <conikee@gmail.com>
// Licensed under the MIT License

//! Sniff CLI - Advanced navigation and search for Claude Code session histories.

#![allow(clippy::manual_flatten)]

use clap::{Parser, Subcommand, ValueEnum};
use sniff::{Result, SimpleSessionAnalysis, SniffError};
use std::path::PathBuf;
use tracing::{error, info, warn, Level};
use tracing_subscriber::fmt;

/// Sniff CLI - Advanced navigation and search for Claude Code session histories.
#[derive(Parser)]
#[command(
    name = "sniff",
    version = env!("CARGO_PKG_VERSION"),
    author = "Chetan Conikee <conikee@gmail.com>",
    about = "Advanced CLI tool for navigating and searching Claude Code session histories using Merkle trees"
)]
struct Cli {
    /// Enable verbose logging (use multiple times for more verbosity)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Path to Claude projects directory
    #[arg(long, default_value = "~/.claude/projects")]
    projects_path: PathBuf,

    /// Path to database file
    #[arg(long, default_value = "~/.claude/sniff.redb")]
    database_path: PathBuf,

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
    /// Scan and index Claude projects into the database
    Scan {
        /// Specific project to scan (optional, scans all if not provided)
        #[arg(short, long)]
        project: Option<String>,
        /// Force re-indexing of already processed sessions
        #[arg(short, long)]
        force: bool,
        /// Maximum number of messages per session to process
        #[arg(long)]
        max_messages: Option<usize>,
        /// Skip operation extraction for faster processing
        #[arg(long)]
        skip_operations: bool,
    },

    /// Search indexed sessions and operations
    Search {
        /// Search query (supports patterns and filters)
        query: Option<String>,
        /// Limit number of results
        #[arg(short, long, default_value = "10")]
        limit: usize,
        /// Output format
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,
        /// Search in specific project only
        #[arg(short, long)]
        project: Option<String>,
        /// Show conversation context around matches
        #[arg(short, long)]
        context: bool,
    },

    /// Show information about indexed data
    Info {
        /// Show information about a specific session
        #[arg(short, long)]
        session: Option<String>,
        /// Show information about a specific project
        #[arg(short, long)]
        project: Option<String>,
        /// Output format
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,
    },

    /// Show database and processing statistics
    Stats {
        /// Project to analyze (optional, shows all if not provided)
        #[arg(short, long)]
        project: Option<String>,
        /// Output format
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,
        /// Show detailed breakdown
        #[arg(short, long)]
        detailed: bool,
    },

    /// Analyze Claude Code sessions for bullshit patterns
    Analyze {
        /// Specific session file to analyze (optional, analyzes all if not provided)
        #[arg(short, long)]
        session: Option<PathBuf>,
        /// Project to analyze (optional, analyzes all if not provided)
        #[arg(short, long)]
        project: Option<String>,
        /// Output format
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,
        /// Show detailed pattern analysis
        #[arg(short, long)]
        detailed: bool,
    },

    /// Analyze arbitrary files for bullshit patterns (independent of Claude Code)
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
    },

    /// Manage analysis checkpoints for tracking changes over time
    Checkpoint {
        #[command(subcommand)]
        command: CheckpointCommands,
    },

    /// Database maintenance operations
    Db {
        #[command(subcommand)]
        command: DbCommands,
    },

    /// Manage learned patterns for dynamic bullshit detection
    Patterns {
        #[command(subcommand)]
        command: PatternCommands,
    },

    /// Legacy commands (for backward compatibility)
    #[command(hide = true)]
    Parse {
        /// Path to the JSONL session file
        file: PathBuf,
        /// Maximum number of lines to parse
        #[arg(long)]
        max_lines: Option<usize>,
        /// Show detailed operation analysis
        #[arg(long)]
        analyze_operations: bool,
    },
}

/// Database maintenance commands
#[derive(Subcommand)]
enum DbCommands {
    /// Compact the database to reclaim space
    Compact,
    /// Show database statistics and health
    Status,
    /// Clear all indexed data
    Clear {
        /// Confirm the operation
        #[arg(long)]
        confirm: bool,
    },
    /// Export data to JSON format
    Export {
        /// Output file path
        output: PathBuf,
        /// Export specific project only
        #[arg(short, long)]
        project: Option<String>,
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

    // Expand paths
    let projects_path = expand_path(cli.projects_path);
    let database_path = expand_path(cli.database_path);

    // Execute the selected command
    match cli.command {
        Commands::Scan {
            project,
            force,
            max_messages,
            skip_operations,
        } => {
            handle_scan_command(
                projects_path,
                database_path,
                project,
                force,
                max_messages,
                skip_operations,
            )
            .await
        }

        Commands::Search {
            query,
            limit,
            format,
            project,
            context,
        } => handle_search_command(database_path, query, limit, format, project, context).await,

        Commands::Info {
            session,
            project,
            format,
        } => handle_info_command(database_path, session, project, format).await,

        Commands::Stats {
            project,
            format,
            detailed,
        } => handle_stats_command(database_path, project, format, detailed).await,

        Commands::Analyze {
            session,
            project,
            format,
            detailed,
        } => handle_analyze_command(projects_path, session, project, format, detailed).await,

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
        } => handle_analyze_files_command(
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
        ).await,

        Commands::Checkpoint { command } => handle_checkpoint_command(command).await,

        Commands::Db { command } => handle_db_command(database_path, command).await,

        Commands::Patterns { command } => handle_patterns_command(command).await,

        // Legacy command for backward compatibility
        Commands::Parse {
            file,
            max_lines,
            analyze_operations,
        } => handle_legacy_parse_command(file, max_lines, analyze_operations).await,
    }
}

/// Handles the scan command - processes and indexes Claude projects.
async fn handle_scan_command(
    projects_path: PathBuf,
    database_path: PathBuf,
    project_filter: Option<String>,
    _force: bool,
    max_messages: Option<usize>,
    skip_operations: bool,
) -> Result<()> {
    use sniff::progress::{show_success, ProgressIndicator};
    use sniff::session::{SessionProcessor, SessionProcessorConfig};
    use sniff::storage::{StorageConfig, TreeStorage};

    // Start progress indicator
    let progress = ProgressIndicator::new("scan");

    info!("Scanning Claude projects in: {}", projects_path.display());
    info!("Database: {}", database_path.display());

    // Initialize storage
    progress.update("Initializing database connection...");
    let storage_config = StorageConfig {
        db_path: database_path,
        ..Default::default()
    };
    let storage = TreeStorage::open(storage_config)?;

    // Configure session processor
    let processor_config = SessionProcessorConfig {
        extract_operations: !skip_operations,
        max_messages,
        ..Default::default()
    };
    let mut processor = SessionProcessor::new(storage, processor_config)?;

    // Discover projects to scan
    let mut projects_to_scan = Vec::new();

    if let Some(project_name) = project_filter {
        let project_path = projects_path.join(&project_name);
        if project_path.exists() {
            projects_to_scan.push((project_name, project_path));
        } else {
            progress.finish_with_error(&format!(
                "Project '{}' not found in {}",
                project_name,
                projects_path.display()
            ));
            return Ok(());
        }
    } else {
        // Discover all projects
        progress.update("Discovering Claude projects...");
        use sniff::watcher::utils;
        let discovered_projects = utils::discover_projects(&projects_path)?;

        for project_path in discovered_projects {
            let project_name = utils::extract_project_name(&project_path);
            projects_to_scan.push((project_name, project_path));
        }
    }

    if projects_to_scan.is_empty() {
        progress.finish_with_error("No projects found to scan");
        return Ok(());
    }

    // Process each project
    let mut total_processed = 0;
    let mut total_errors = 0;
    let total_projects = projects_to_scan.len();

    for (i, (project_name, project_path)) in projects_to_scan.iter().enumerate() {
        progress.update(&format!(
            "Processing project {} of {}: {}",
            i + 1,
            total_projects,
            project_name
        ));
        info!("Processing project: {}", project_name);

        match processor.process_project_directory(project_path) {
            Ok(_project_hash) => {
                let stats = processor.stats();
                info!(
                    "✓ Processed project '{}': {} sessions, {} messages, {} operations",
                    project_name,
                    stats.sessions_processed,
                    stats.total_messages,
                    stats.total_operations
                );
                total_processed += stats.sessions_processed;
                total_errors += stats.error_count;
            }
            Err(e) => {
                error!("✗ Failed to process project '{}': {}", project_name, e);
                total_errors += 1;
            }
        }

        // Reset stats for next project
        processor.reset_stats();
    }

    // Show completion message
    if total_errors > 0 {
        progress.finish_with_error(&format!(
            "Scan completed with errors: {} sessions processed, {} errors encountered",
            total_processed, total_errors
        ));
    } else {
        progress.finish(Some(&format!(
            "🎉 Successfully indexed {} sessions across {} projects!",
            total_processed, total_projects
        )));

        // Show summary stats
        let final_stats = processor.stats();
        show_success(&format!(
            "📊 Summary: {} messages processed, {} operations extracted",
            final_stats.total_messages, final_stats.total_operations
        ));
    }

    Ok(())
}

/// Handles the search command - queries indexed data.
async fn handle_search_command(
    database_path: PathBuf,
    query: Option<String>,
    limit: usize,
    format: OutputFormat,
    _project_filter: Option<String>,
    context: bool,
) -> Result<()> {
    use sniff::storage::{StorageConfig, TreeStorage};

    // Initialize storage
    let storage_config = StorageConfig {
        db_path: database_path,
        ..Default::default()
    };
    let storage = TreeStorage::open(storage_config)?;

    // Handle context mode or regular search
    if let Some(query_str) = query {
        info!("Searching indexed data for: '{}'", query_str);

        if context {
            // Use enhanced search with conversation context
            use sniff::search::{EnhancedSearchEngine, SearchConfig};

            println!("🔍 Context-aware search for: '{}'", query_str);
            println!();

            let config = SearchConfig::default();
            let mut engine = EnhancedSearchEngine::new(storage, config);
            let threads = engine.search(&query_str)?;

            if threads.is_empty() {
                println!("❌ No conversation threads found for '{}'", query_str);
                return Ok(());
            }

            println!("🎯 Found {} debugging session(s):", threads.len());
            println!();

            for (i, thread) in threads.iter().enumerate() {
                println!(
                    "{}. 📄 Session: {} ({})",
                    i + 1,
                    thread.session_id,
                    thread
                        .root_message
                        .message
                        .timestamp()
                        .map(|ts| ts.format("%Y-%m-%d %H:%M").to_string())
                        .unwrap_or_else(|| "Unknown time".to_string())
                );

                // Extract and show files that were modified
                let mut files_modified = Vec::new();
                let mut commands_run = Vec::new();
                let mut thinking_insights = Vec::new();

                for message_ctx in &thread.messages {
                    if let Some(search_match) = &message_ctx.search_match {
                        if search_match.snippet.starts_with("📁") {
                            files_modified.push(&search_match.snippet[4..]);
                        } else if search_match.snippet.starts_with("💻") {
                            commands_run.push(&search_match.snippet[4..]);
                        } else if search_match.snippet.starts_with("💭") {
                            thinking_insights.push(&search_match.snippet[4..]);
                        }
                    }
                }

                // Show files modified
                if !files_modified.is_empty() {
                    println!("   📁 Files modified:");
                    for file in files_modified.into_iter().take(5) {
                        println!("      • {}", file);
                    }
                }

                // Show tools and their purpose
                if !thread.tools_used.is_empty() {
                    println!("   🔧 Tools used:");
                    for tool in &thread.tools_used {
                        if let Some(file_path) = tool.input.get("file_path") {
                            println!(
                                "      • {} → {}",
                                tool.name,
                                file_path.as_str().unwrap_or("unknown")
                            );
                        } else if let Some(command) = tool.input.get("command") {
                            println!(
                                "      • {} → {}",
                                tool.name,
                                command.as_str().unwrap_or("unknown")
                            );
                        } else {
                            println!("      • {}", tool.name);
                        }
                    }
                }

                // Show reasoning/thinking that led to changes
                if !thinking_insights.is_empty() {
                    println!("   💭 Key reasoning:");
                    for insight in thinking_insights.into_iter().take(2) {
                        println!("      • {}", insight);
                    }
                }

                println!();
            }

            return Ok(());
        }

        // Perform regular content search through indexed messages
        let search_results = storage.search_content(&query_str, limit)?;

        match format {
            OutputFormat::Table => {
                if search_results.is_empty() {
                    println!("No content found matching '{}'", query_str);
                } else {
                    println!(
                        "Found {} session(s) with matching content:",
                        search_results.len()
                    );
                    for (session_id, snippets) in search_results {
                        println!("\n📄 Session: {}", session_id);
                        for (i, snippet) in snippets.iter().enumerate() {
                            if i < 3 {
                                // Show max 3 snippets per session
                                println!("   💬 {}", snippet.trim());
                            }
                        }
                        if snippets.len() > 3 {
                            println!("   ... and {} more matches", snippets.len() - 3);
                        }
                    }
                }
            }
            OutputFormat::Json => {
                let json_output = serde_json::json!({
                    "query": query_str,
                    "limit": limit,
                    "results": search_results.iter().map(|(session_id, snippets)| {
                        serde_json::json!({
                            "session_id": session_id,
                            "snippets": snippets
                        })
                    }).collect::<Vec<_>>()
                });
                println!("{}", serde_json::to_string_pretty(&json_output)?);
            }
            OutputFormat::Markdown => {
                println!("# Search Results");
                println!();
                for (session_id, snippets) in search_results {
                    println!("## Session: `{}`", session_id);
                    println!();
                    for snippet in snippets {
                        println!("```");
                        println!("{}", snippet);
                        println!("```");
                        println!();
                    }
                }
            }
            OutputFormat::Compact => {
                for (session_id, _) in search_results {
                    println!("{}", session_id);
                }
            }
        }
    } else {
        // No query provided - show help or launch interactive mode
        println!("🔍 Sniff Search");
        println!("════════════════════");
        println!();
        println!("Usage:");
        println!("  sniff search \"your query\"           # Basic search");
        println!("  sniff search \"query\" --context      # Context-aware search");
        println!();
        println!("💡 Try: sniff search \"your query\" --context for the full experience!");
    }

    Ok(())
}

/// Handles the info command - shows information about indexed data.
async fn handle_info_command(
    database_path: PathBuf,
    session_id: Option<String>,
    project_name: Option<String>,
    format: OutputFormat,
) -> Result<()> {
    use sniff::storage::{StorageConfig, TreeStorage};

    // Initialize storage
    let storage_config = StorageConfig {
        db_path: database_path,
        ..Default::default()
    };
    let mut storage = TreeStorage::open(storage_config)?;

    if let Some(session) = session_id {
        // Show session info
        if let Some(root_hash) = storage.get_session_root(&session)? {
            if let Some(session_node) = storage.get_node(&root_hash)? {
                match format {
                    OutputFormat::Table => {
                        println!("Session Information:");
                        println!("  ID: {}", session);
                        println!("  Root Hash: {}", root_hash);
                        println!("  Messages: {}", session_node.metadata.message_count);
                        println!("  Operations: {}", session_node.metadata.operation_count);
                        println!(
                            "  Content Size: {} bytes",
                            session_node.metadata.content_size
                        );
                        println!("  Created: {}", session_node.metadata.created_at);
                        println!("  Updated: {}", session_node.metadata.updated_at);
                    }
                    OutputFormat::Json => {
                        let json_output = serde_json::json!({
                            "session_id": session,
                            "root_hash": root_hash.to_string(),
                            "message_count": session_node.metadata.message_count,
                            "operation_count": session_node.metadata.operation_count,
                            "content_size": session_node.metadata.content_size,
                            "created_at": session_node.metadata.created_at,
                            "updated_at": session_node.metadata.updated_at
                        });
                        println!("{}", serde_json::to_string_pretty(&json_output)?);
                    }
                    OutputFormat::Markdown => {
                        println!("# Session Information");
                        println!();
                        println!("| Property | Value |");
                        println!("| -------- | ----- |");
                        println!("| ID | `{}` |", session);
                        println!("| Root Hash | `{}` |", root_hash);
                        println!("| Messages | {} |", session_node.metadata.message_count);
                        println!("| Operations | {} |", session_node.metadata.operation_count);
                        println!("| Content Size | {} bytes |", session_node.metadata.content_size);
                        println!("| Created | {} |", session_node.metadata.created_at);
                        println!("| Updated | {} |", session_node.metadata.updated_at);
                    }
                    OutputFormat::Compact => {
                        println!(
                            "{}: {} msgs, {} ops, {} bytes",
                            session,
                            session_node.metadata.message_count,
                            session_node.metadata.operation_count,
                            session_node.metadata.content_size
                        );
                    }
                }
            } else {
                error!("Session node not found for hash: {}", root_hash);
            }
        } else {
            error!("Session '{}' not found", session);
        }
    } else if let Some(project) = project_name {
        // Display project-specific information
        println!("📁 Project Information for: {}", project);

        // Get all sessions for this project
        let sessions = storage.list_sessions()?;
        let project_sessions: Vec<_> = sessions
            .into_iter()
            .filter(|s| s.contains(&project))
            .collect();

        if project_sessions.is_empty() {
            println!("   No sessions found for project '{}'", project);
        } else {
            println!("   📊 Sessions found: {}", project_sessions.len());
            for session in &project_sessions {
                println!("     - {}", session);
            }
        }
    } else {
        // Show general database info
        let stats = storage.get_stats()?;
        match format {
            OutputFormat::Table => {
                println!("Database Information:");
                println!("  Total Nodes: {}", stats.total_nodes);
                println!("  Total Sessions: {}", stats.total_sessions);
                println!("  Total Projects: {}", stats.total_projects);
                println!("  File Size: {} bytes", stats.file_size_bytes);
                println!("  Schema Version: {}", stats.schema_version);
            }
            OutputFormat::Json => {
                let json_output = serde_json::to_value(&stats)?;
                println!("{}", serde_json::to_string_pretty(&json_output)?);
            }
            OutputFormat::Markdown => {
                println!("# Database Information");
                println!();
                println!("| Property | Value |");
                println!("| -------- | ----- |");
                println!("| Total Nodes | {} |", stats.total_nodes);
                println!("| Total Sessions | {} |", stats.total_sessions);
                println!("| Total Projects | {} |", stats.total_projects);
                println!("| File Size | {} bytes |", stats.file_size_bytes);
                println!("| Schema Version | {} |", stats.schema_version);
            }
            OutputFormat::Compact => {
                println!(
                    "{} nodes, {} sessions, {} projects",
                    stats.total_nodes, stats.total_sessions, stats.total_projects
                );
            }
        }
    }

    Ok(())
}

/// Handles the stats command - shows database and processing statistics.
async fn handle_stats_command(
    database_path: PathBuf,
    project_filter: Option<String>,
    format: OutputFormat,
    detailed: bool,
) -> Result<()> {
    use sniff::storage::{StorageConfig, TreeStorage};

    // Initialize storage
    let storage_config = StorageConfig {
        db_path: database_path.clone(),
        ..Default::default()
    };
    let storage = TreeStorage::open(storage_config)?;

    let db_stats = storage.get_stats()?;
    let cache_stats = storage.cache_stats();

    match format {
        OutputFormat::Table => {
            println!("Sniff Statistics");
            println!("=====================");
            println!();
            println!("Database:");
            println!("  Total Nodes: {}", db_stats.total_nodes);
            println!("  Total Sessions: {}", db_stats.total_sessions);
            println!("  Total Projects: {}", db_stats.total_projects);
            println!(
                "  File Size: {:.2} MB",
                db_stats.file_size_bytes as f64 / 1_048_576.0
            );
            println!("  Schema Version: {}", db_stats.schema_version);

            if detailed {
                println!();
                println!("Cache Performance:");
                println!("  Hit Ratio: {:.2}%", cache_stats.hit_ratio() * 100.0);
                println!("  Total Hits: {}", cache_stats.hits);
                println!("  Total Misses: {}", cache_stats.misses);
                println!("  Evictions: {}", cache_stats.evictions);
            }

            if let Some(project) = project_filter {
                println!();
                println!("Project Filter: {}", project);

                // Get project-specific statistics
                let sessions = storage.list_sessions()?;
                let project_sessions: Vec<_> = sessions
                    .into_iter()
                    .filter(|s| s.contains(&project))
                    .collect();

                println!("  Project Sessions: {}", project_sessions.len());
                if !project_sessions.is_empty() {
                    println!("  Sessions:");
                    for session in &project_sessions {
                        println!("    - {}", session);
                    }
                }
            }
        }
        OutputFormat::Json => {
            let mut json_output = serde_json::json!({
                "database": {
                    "total_nodes": db_stats.total_nodes,
                    "total_sessions": db_stats.total_sessions,
                    "total_projects": db_stats.total_projects,
                    "file_size_bytes": db_stats.file_size_bytes,
                    "schema_version": db_stats.schema_version
                }
            });

            if detailed {
                json_output["cache"] = serde_json::json!({
                    "hit_ratio": cache_stats.hit_ratio(),
                    "hits": cache_stats.hits,
                    "misses": cache_stats.misses,
                    "evictions": cache_stats.evictions
                });
            }

            if let Some(project) = project_filter {
                json_output["project_filter"] = serde_json::json!(project);
            }

            println!("{}", serde_json::to_string_pretty(&json_output)?);
        }
        OutputFormat::Markdown => {
            println!("# Sniff Statistics");
            println!();
            
            println!("## Database");
            println!();
            println!("| Metric | Value |");
            println!("| ------ | ----- |");
            println!("| Total Nodes | {} |", db_stats.total_nodes);
            println!("| Total Sessions | {} |", db_stats.total_sessions);
            println!("| Total Projects | {} |", db_stats.total_projects);
            println!("| File Size | {:.2} MB |", db_stats.file_size_bytes as f64 / 1_048_576.0);
            println!("| Schema Version | {} |", db_stats.schema_version);
            println!();

            if detailed {
                println!("## Cache Performance");
                println!();
                println!("| Metric | Value |");
                println!("| ------ | ----- |");
                println!("| Hit Ratio | {:.2}% |", cache_stats.hit_ratio() * 100.0);
                println!("| Total Hits | {} |", cache_stats.hits);
                println!("| Total Misses | {} |", cache_stats.misses);
                println!("| Evictions | {} |", cache_stats.evictions);
                println!();
            }

            if let Some(project) = project_filter {
                println!("## Project Filter");
                println!();
                println!("**Project**: `{}`", project);
                println!();
                
                // Get project-specific statistics
                let storage_config = sniff::storage::StorageConfig {
                    db_path: database_path,
                    ..Default::default()
                };
                let storage = sniff::storage::TreeStorage::open(storage_config)?;
                let sessions = storage.list_sessions()?;
                let project_sessions: Vec<_> = sessions
                    .into_iter()
                    .filter(|s| s.contains(&project))
                    .collect();

                println!("**Project Sessions**: {}", project_sessions.len());
                if !project_sessions.is_empty() {
                    println!();
                    println!("### Sessions:");
                    for session in &project_sessions {
                        println!("- `{}`", session);
                    }
                    println!();
                }
            }
        }
        OutputFormat::Compact => {
            println!(
                "{} nodes, {} sessions, {} projects, {:.1}MB",
                db_stats.total_nodes,
                db_stats.total_sessions,
                db_stats.total_projects,
                db_stats.file_size_bytes as f64 / 1_048_576.0
            );
        }
    }

    Ok(())
}

/// Handles database maintenance commands.
async fn handle_db_command(database_path: PathBuf, command: DbCommands) -> Result<()> {
    use sniff::storage::{StorageConfig, TreeStorage};

    match command {
        DbCommands::Compact => {
            info!("Compacting database: {}", database_path.display());

            let storage_config = StorageConfig {
                db_path: database_path,
                ..Default::default()
            };
            let mut storage = TreeStorage::open(storage_config)?;

            storage.compact()?;
            println!("✓ Database compaction completed");
        }

        DbCommands::Status => {
            let storage_config = StorageConfig {
                db_path: database_path.clone(),
                ..Default::default()
            };
            let storage = TreeStorage::open(storage_config)?;

            let stats = storage.get_stats()?;
            let cache_stats = storage.cache_stats();

            println!("Database Status");
            println!("===============");
            println!("Path: {}", database_path.display());
            println!("Exists: {}", database_path.exists());
            if database_path.exists() {
                println!("Size: {:.2} MB", stats.file_size_bytes as f64 / 1_048_576.0);
            }
            println!("Schema Version: {}", stats.schema_version);
            println!("Nodes: {}", stats.total_nodes);
            println!("Sessions: {}", stats.total_sessions);
            println!("Projects: {}", stats.total_projects);
            println!("Cache Hit Ratio: {:.2}%", cache_stats.hit_ratio() * 100.0);
        }

        DbCommands::Clear { confirm } => {
            if !confirm {
                error!("Database clear requires --confirm flag for safety");
                return Ok(());
            }

            info!("Clearing database: {}", database_path.display());

            if database_path.exists() {
                std::fs::remove_file(&database_path).map_err(|e| {
                    SniffError::storage_error(format!("Failed to remove database: {e}"))
                })?;
                println!("✓ Database cleared");
            } else {
                println!("Database does not exist");
            }
        }

        DbCommands::Export { output, project } => {
            info!("Exporting database to: {}", output.display());

            let storage_config = StorageConfig {
                db_path: database_path,
                ..Default::default()
            };
            let storage = TreeStorage::open(storage_config)?;

            let sessions = storage.list_sessions()?;
            let filtered_sessions: Vec<_> = if let Some(project_filter) = project {
                sessions
                    .into_iter()
                    .filter(|s| s.contains(&project_filter))
                    .collect()
            } else {
                sessions
            };

            let export_data = serde_json::json!({
                "export_timestamp": chrono::Utc::now(),
                "total_sessions": filtered_sessions.len(),
                "sessions": filtered_sessions
            });

            std::fs::write(&output, serde_json::to_string_pretty(&export_data)?)?;
            println!(
                "✓ Exported {} sessions to {}",
                filtered_sessions.len(),
                output.display()
            );
        }
    }

    Ok(())
}

/// Legacy parse command for backward compatibility.
async fn handle_legacy_parse_command(
    file: PathBuf,
    max_lines: Option<usize>,
    analyze_operations: bool,
) -> Result<()> {
    use sniff::jsonl::{JsonlParser, ParseConfig};
    use sniff::operations::OperationExtractor;

    warn!("Using legacy parse command. Consider using 'scan' for full indexing.");
    info!("Parsing JSONL file: {}", file.display());

    let config = ParseConfig {
        max_lines: max_lines.unwrap_or(0),
        validate_consistency: true,
        skip_malformed: false,
    };

    let parser = JsonlParser::with_config(config);
    let parse_result = parser.parse_file(&file)?;

    println!("Parse Results:");
    println!("  Messages: {}", parse_result.messages.len());
    println!("  Lines processed: {}", parse_result.lines_processed);
    println!("  Malformed lines: {}", parse_result.malformed_lines);

    if let Some(ref session_id) = parse_result.session_id {
        println!("  Session ID: {}", session_id);
    }

    if !parse_result.warnings.is_empty() {
        println!("  Warnings:");
        for warning in &parse_result.warnings {
            println!("    - {}", warning);
        }
    }

    if analyze_operations {
        println!("\nOperation Analysis:");
        let extractor = OperationExtractor::new();
        let operations = extractor.extract_operations(&parse_result.messages)?;

        use sniff::operations::OperationStats;
        let stats = OperationStats::from_operations(&operations);

        println!("  Total operations: {}", stats.total_operations);
        println!(
            "  File-modifying operations: {}",
            stats.file_modifying_operations
        );

        if !stats.tool_usage.is_empty() {
            println!("  Tool usage:");
            for (tool, count) in &stats.tool_usage {
                println!("    {}: {}", tool, count);
            }
        }
    }

    Ok(())
}

/// Expands ~ in file paths to the home directory.
fn expand_path(path: PathBuf) -> PathBuf {
    if let Ok(path_str) = path.to_str().ok_or("Invalid path").and_then(|s| {
        if s.starts_with('~') {
            dirs::home_dir()
                .ok_or("Could not find home directory")
                .map(|home| home.join(&s[2..]).to_string_lossy().to_string())
        } else {
            Ok(s.to_string())
        }
    }) {
        PathBuf::from(path_str)
    } else {
        path
    }
}

/// Handles the analyze command - analyzes Claude Code sessions for bullshit patterns.
async fn handle_analyze_command(
    projects_path: PathBuf,
    session_file: Option<PathBuf>,
    project_filter: Option<String>,
    format: OutputFormat,
    detailed: bool,
) -> Result<()> {
    use sniff::SimpleSessionAnalyzer;

    info!("🕵️  Starting bullshit analysis of Claude Code sessions");

    // Create the session analyzer (use quiet mode for non-table formats)
    let mut analyzer = if format == OutputFormat::Table {
        SimpleSessionAnalyzer::new()?
    } else {
        SimpleSessionAnalyzer::new_quiet()?
    };
    let mut all_analyses = Vec::new();

    if let Some(session_path) = session_file {
        // Analyze a specific session file
        info!(
            "Analyzing specific session file: {}",
            session_path.display()
        );

        match analyzer.analyze_session(&session_path) {
            Ok(analysis) => {
                all_analyses.push(analysis);
            }
            Err(e) => {
                error!("Failed to analyze session: {}", e);
                return Err(e);
            }
        }
    } else {
        // Analyze sessions in the projects directory
        info!("Analyzing Claude projects in: {}", projects_path.display());

        if let Some(project_name) = project_filter {
            // Analyze specific project
            let project_path = projects_path.join(&project_name);
            if !project_path.exists() {
                error!(
                    "Project '{}' not found in {}",
                    project_name,
                    projects_path.display()
                );
                return Ok(());
            }

            match analyzer.analyze_project_directory(&project_path) {
                Ok(analyses) => {
                    all_analyses.extend(analyses);
                }
                Err(e) => {
                    error!("Failed to analyze project '{}': {}", project_name, e);
                    return Err(e);
                }
            }
        } else {
            // Analyze all projects
            if let Ok(entries) = std::fs::read_dir(&projects_path) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let project_path = entry.path();
                        if project_path.is_dir() {
                            let project_name = project_path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy();

                            info!("Analyzing project: {}", project_name);
                            match analyzer.analyze_project_directory(&project_path) {
                                Ok(analyses) => {
                                    all_analyses.extend(analyses);
                                }
                                Err(e) => {
                                    warn!("Failed to analyze project '{}': {}", project_name, e);
                                }
                            }
                        }
                    }
                }
            } else {
                error!(
                    "Failed to read projects directory: {}",
                    projects_path.display()
                );
                return Ok(());
            }
        }
    }

    // Display results based on format
    display_analysis_results(&all_analyses, format, detailed)?;

    Ok(())
}

/// Displays analysis results in the specified format.
fn display_analysis_results(
    analyses: &[SimpleSessionAnalysis],
    format: OutputFormat,
    detailed: bool,
) -> Result<()> {
    if analyses.is_empty() {
        println!("No sessions analyzed or found.");
        return Ok(());
    }

    match format {
        OutputFormat::Table => {
            display_table_format(analyses, detailed);
        }
        OutputFormat::Json => {
            let json_output = serde_json::to_string_pretty(&analyses)?;
            println!("{}", json_output);
        }
        OutputFormat::Markdown => {
            display_markdown_format(analyses, detailed);
        }
        OutputFormat::Compact => {
            display_compact_format(analyses);
        }
    }

    Ok(())
}

fn display_table_format(analyses: &[SimpleSessionAnalysis], detailed: bool) {
    println!("🕵️  Bullshit Analysis Results");
    println!("═══════════════════════════════");
    println!();

    // Summary statistics
    let total_sessions = analyses.len();
    let total_files = analyses
        .iter()
        .map(|a| a.modified_files.len())
        .sum::<usize>();
    let total_patterns = analyses
        .iter()
        .map(|a| a.metrics.total_bullshit_patterns)
        .sum::<usize>();
    let critical_patterns = analyses
        .iter()
        .map(|a| a.metrics.critical_patterns)
        .sum::<usize>();
    let avg_quality = if total_sessions > 0 {
        analyses
            .iter()
            .map(|a| a.metrics.quality_score)
            .sum::<f64>()
            / total_sessions as f64
    } else {
        0.0
    };

    println!("📊 Summary:");
    println!("   Sessions analyzed: {}", total_sessions);
    println!("   Files modified: {}", total_files);
    println!("   Bullshit patterns: {}", total_patterns);
    println!("   Critical patterns: {}", critical_patterns);
    println!("   Average quality: {:.1}%", avg_quality);
    println!();

    if critical_patterns > 0 {
        println!("🚨 Sessions with Critical Issues:");
        for analysis in analyses {
            if analysis.metrics.critical_patterns > 0 {
                println!(
                    "   {} - {} critical patterns, {:.1}% quality",
                    analysis.session_id,
                    analysis.metrics.critical_patterns,
                    analysis.metrics.quality_score
                );
            }
        }
        println!();
    }

    // Always show session analysis with issues and recommendations
    if !analyses.is_empty() && analyses.iter().any(|a| !a.bullshit_detections.is_empty() || !a.recommendations.is_empty()) {
        println!("📄 Session Analysis:");
        for analysis in analyses {
            if !analysis.bullshit_detections.is_empty() || !analysis.recommendations.is_empty() {
                println!("   Session: {}", analysis.session_id);
                println!("      Files: {}", analysis.modified_files.len());
                println!("      Operations: {}", analysis.file_operations.len());
                println!(
                    "      Patterns: {}",
                    analysis.metrics.total_bullshit_patterns
                );
                println!("      Quality: {:.1}%", analysis.metrics.quality_score);

                if !analysis.bullshit_detections.is_empty() {
                    println!("      Issues:");
                    for detection in &analysis.bullshit_detections {
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

                if !analysis.recommendations.is_empty() {
                    println!("      Recommendations:");
                    for rec in &analysis.recommendations {
                        println!("         {}", rec);
                    }
                }
                println!();
            }
        }
    }

    // Additional detailed information only when --detailed flag is used
    if detailed {
        println!("📄 Additional Details:");
        for analysis in analyses {
            println!("   Session: {}", analysis.session_id);
            println!("      Modified Files:");
            for file in &analysis.modified_files {
                println!("         {}", file);
            }
            println!();
        }
    }

    // Overall recommendation
    if total_patterns > 0 {
        println!("💡 Overall Recommendations:");
        if critical_patterns > 0 {
            println!(
                "   🚨 Address {} critical patterns immediately",
                critical_patterns
            );
        }
        if avg_quality < 80.0 {
            println!(
                "   📝 Code quality needs improvement (current: {:.1}%)",
                avg_quality
            );
        }
        if total_patterns > total_sessions * 5 {
            println!(
                "   🔍 High pattern density detected - consider reviewing development practices"
            );
        }
    } else {
        println!("✅ No bullshit patterns detected! Excellent work!");
    }
}

fn display_markdown_format(analyses: &[SimpleSessionAnalysis], detailed: bool) {
    println!("# 🕵️ Bullshit Analysis Results");
    println!();

    // Summary statistics
    let total_sessions = analyses.len();
    let total_files = analyses
        .iter()
        .map(|a| a.modified_files.len())
        .sum::<usize>();
    let total_patterns = analyses
        .iter()
        .map(|a| a.metrics.total_bullshit_patterns)
        .sum::<usize>();
    let critical_patterns = analyses
        .iter()
        .map(|a| a.metrics.critical_patterns)
        .sum::<usize>();
    let avg_quality = if total_sessions > 0 {
        analyses
            .iter()
            .map(|a| a.metrics.quality_score)
            .sum::<f64>()
            / total_sessions as f64
    } else {
        0.0
    };

    println!("## 📊 Summary");
    println!();
    println!("| Metric | Value |");
    println!("| ------ | ----- |");
    println!("| Sessions analyzed | {} |", total_sessions);
    println!("| Files modified | {} |", total_files);
    println!("| Bullshit patterns | {} |", total_patterns);
    println!("| Critical patterns | {} |", critical_patterns);
    println!("| Average quality | {:.1}% |", avg_quality);
    println!();

    if critical_patterns > 0 {
        println!("## 🚨 Sessions with Critical Issues");
        println!();
        for analysis in analyses {
            if analysis.metrics.critical_patterns > 0 {
                println!(
                    "- **{}** - {} critical patterns, {:.1}% quality",
                    analysis.session_id,
                    analysis.metrics.critical_patterns,
                    analysis.metrics.quality_score
                );
            }
        }
        println!();
    }

    // Always show session analysis with issues and recommendations
    if !analyses.is_empty() && analyses.iter().any(|a| !a.bullshit_detections.is_empty() || !a.recommendations.is_empty()) {
        println!("## 📄 Session Analysis");
        println!();
        for analysis in analyses {
            if !analysis.bullshit_detections.is_empty() || !analysis.recommendations.is_empty() {
                println!("### Session: `{}`", analysis.session_id);
                println!();
                println!("- **Files**: {}", analysis.modified_files.len());
                println!("- **Operations**: {}", analysis.file_operations.len());
                println!("- **Patterns**: {}", analysis.metrics.total_bullshit_patterns);
                println!("- **Quality**: {:.1}%", analysis.metrics.quality_score);
                println!();

                if !analysis.bullshit_detections.is_empty() {
                    println!("#### Issues");
                    println!();
                    for detection in &analysis.bullshit_detections {
                        println!(
                            "- {} **{}** (`{}:{}`):",
                            detection.severity.emoji(),
                            detection.rule_name,
                            detection.file_path,
                            detection.line_number
                        );
                        println!("  ```");
                        println!("  {}", detection.code_snippet.trim());
                        println!("  ```");
                        println!("  *{}*", detection.description);
                        println!();
                    }
                }

                if !analysis.recommendations.is_empty() {
                    println!("#### Recommendations");
                    println!();
                    for rec in &analysis.recommendations {
                        println!("- {}", rec);
                    }
                    println!();
                }
            }
        }
    }

    // Additional detailed information only when --detailed flag is used
    if detailed {
        println!("## 📄 Additional Details");
        println!();
        for analysis in analyses {
            println!("### Session: `{}`", analysis.session_id);
            println!();
            println!("- **Modified Files**:");
            for file in &analysis.modified_files {
                println!("  - `{}`", file);
            }
            println!();
        }
    }

    // Overall recommendation
    if total_patterns > 0 {
        println!("## 💡 Overall Recommendations");
        println!();
        if critical_patterns > 0 {
            println!(
                "- 🚨 **Address {} critical patterns immediately**",
                critical_patterns
            );
        }
        if avg_quality < 80.0 {
            println!(
                "- 📝 **Code quality needs improvement** (current: {:.1}%)",
                avg_quality
            );
        }
        if total_patterns > total_sessions * 5 {
            println!(
                "- 🔍 **High pattern density detected** - consider reviewing development practices"
            );
        }
    } else {
        println!("## ✅ Results");
        println!();
        println!("**No bullshit patterns detected! Excellent work!**");
    }
}

fn display_compact_format(analyses: &[SimpleSessionAnalysis]) {
    for analysis in analyses {
        println!(
            "{}: {} files, {} patterns, {:.1}% quality",
            analysis.session_id,
            analysis.modified_files.len(),
            analysis.metrics.total_bullshit_patterns,
            analysis.metrics.quality_score
        );
    }
}

/// Handles pattern management commands.
async fn handle_patterns_command(command: PatternCommands) -> Result<()> {
    use sniff::{PatternCreationRequest, PatternLearningManager, SupportedLanguage};
    use sniff::playbook::{PatternScope, Severity};
    
    match command {
        PatternCommands::Init { force } => {
            let current_dir = std::env::current_dir()
                .map_err(|e| SniffError::file_system(".", e))?;
            let sniff_dir = current_dir.join(".sniff");
            
            if sniff_dir.exists() && !force {
                println!("❌ .sniff directory already exists. Use --force to reinitialize.");
                return Ok(());
            }
            
            let manager = PatternLearningManager::new(&current_dir)?;
            println!("✅ Initialized .sniff folder structure at: {}", sniff_dir.display());
            println!("📁 Pattern learning system is ready!");
            Ok(())
        }

        PatternCommands::Create {
            language,
            name,
            description,
            pattern,
            severity,
            scope,
            flags,
            confidence,
            tags,
            examples,
            false_positives,
        } => {
            let current_dir = std::env::current_dir()
                .map_err(|e| SniffError::file_system(".", e))?;
            
            let mut manager = PatternLearningManager::new(&current_dir)?;
            
            // Parse language
            let supported_language = match language.to_lowercase().as_str() {
                "rust" => SupportedLanguage::Rust,
                "python" => SupportedLanguage::Python,
                "typescript" => SupportedLanguage::TypeScript,
                "javascript" => SupportedLanguage::JavaScript,
                "go" => SupportedLanguage::Go,
                "c" => SupportedLanguage::C,
                "cpp" => SupportedLanguage::Cpp,
                _ => {
                    println!("❌ Unsupported language: {}. Supported: rust, python, typescript, javascript, go, c, cpp", language);
                    return Ok(());
                }
            };
            
            // Parse severity
            let pattern_severity = match severity.to_lowercase().as_str() {
                "info" => Severity::Info,
                "low" => Severity::Low,
                "medium" => Severity::Medium,
                "high" => Severity::High,
                "critical" => Severity::Critical,
                _ => {
                    println!("❌ Invalid severity: {}. Use: info, low, medium, high, critical", severity);
                    return Ok(());
                }
            };
            
            // Parse scope
            let pattern_scope = match scope.to_lowercase().as_str() {
                "file" => PatternScope::File,
                "function_body" => PatternScope::FunctionBody,
                "class_body" => PatternScope::ClassBody,
                "comments" => PatternScope::Comments,
                "method_signature" => PatternScope::MethodSignature,
                _ => {
                    println!("❌ Invalid scope: {}. Use: file, function_body, class_body, comments, method_signature", scope);
                    return Ok(());
                }
            };
            
            // Parse tags
            let pattern_tags = tags.map(|t| t.split(',').map(|s| s.trim().to_string()).collect()).unwrap_or_default();
            
            let request = PatternCreationRequest {
                name,
                description,
                severity: pattern_severity,
                pattern,
                flags,
                scope: pattern_scope,
                language: supported_language,
                tags: pattern_tags,
                examples,
                false_positives,
                confidence,
                source: "cli".to_string(),
                metadata: std::collections::HashMap::new(),
            };
            
            match manager.create_pattern(request) {
                Ok(response) => {
                    if response.success {
                        println!("✅ Pattern created successfully!");
                        if let Some(pattern_id) = response.pattern_id {
                            println!("📋 Pattern ID: {}", pattern_id);
                        }
                        if let Some(storage_path) = response.storage_path {
                            println!("💾 Stored at: {}", storage_path.display());
                        }
                        if !response.warnings.is_empty() {
                            println!("⚠️  Warnings:");
                            for warning in response.warnings {
                                println!("   • {}", warning);
                            }
                        }
                    } else {
                        println!("❌ Failed to create pattern:");
                        if let Some(error) = response.error {
                            println!("   {}", error);
                        }
                    }
                }
                Err(e) => {
                    println!("❌ Error creating pattern: {}", e);
                }
            }
            
            Ok(())
        }

        PatternCommands::List { language, format, active_only } => {
            let current_dir = std::env::current_dir()
                .map_err(|e| SniffError::file_system(".", e))?;
            
            let manager = PatternLearningManager::new(&current_dir)?;
            
            let languages = if let Some(lang) = language {
                match lang.to_lowercase().as_str() {
                    "rust" => vec![SupportedLanguage::Rust],
                    "python" => vec![SupportedLanguage::Python],
                    "typescript" => vec![SupportedLanguage::TypeScript],
                    "javascript" => vec![SupportedLanguage::JavaScript],
                    "go" => vec![SupportedLanguage::Go],
                    "c" => vec![SupportedLanguage::C],
                    "cpp" => vec![SupportedLanguage::Cpp],
                    _ => {
                        println!("❌ Unsupported language: {}", lang);
                        return Ok(());
                    }
                }
            } else {
                vec![
                    SupportedLanguage::Rust,
                    SupportedLanguage::Python,
                    SupportedLanguage::TypeScript,
                    SupportedLanguage::JavaScript,
                    SupportedLanguage::Go,
                    SupportedLanguage::C,
                    SupportedLanguage::Cpp,
                ]
            };
            
            let mut all_patterns = Vec::new();
            for lang in languages {
                let patterns = manager.get_patterns_for_language(lang);
                for pattern in patterns {
                    if !active_only || pattern.metadata.active {
                        all_patterns.push((lang, pattern));
                    }
                }
            }
            
            if all_patterns.is_empty() {
                println!("📝 No learned patterns found.");
                println!("💡 Use 'sniff patterns create' to add new patterns!");
                return Ok(());
            }
            
            match format {
                OutputFormat::Table => {
                    println!("🧠 Learned Patterns");
                    println!("═══════════════════");
                    println!();
                    
                    for (lang, pattern) in all_patterns {
                        let status = if pattern.metadata.active { "✅" } else { "❌" };
                        println!("{}  {} | {} | {}", 
                            status,
                            lang.name().to_uppercase(),
                            pattern.rule.severity.name(),
                            pattern.rule.name
                        );
                        println!("   ID: {}", pattern.metadata.id);
                        println!("   Pattern: {}", match &pattern.rule.pattern_type {
                            sniff::playbook::PatternType::Regex { pattern, .. } => pattern,
                            _ => "N/A",
                        });
                        println!("   Detections: {} | Confidence: {:.1}%", 
                            pattern.metadata.detection_count,
                            pattern.metadata.confidence * 100.0
                        );
                        println!();
                    }
                }
                OutputFormat::Json => {
                    let json_data: Vec<_> = all_patterns.iter().map(|(lang, pattern)| {
                        serde_json::json!({
                            "language": lang.name(),
                            "pattern": pattern
                        })
                    }).collect();
                    println!("{}", serde_json::to_string_pretty(&json_data).unwrap());
                }
                _ => {
                    println!("❌ Format {} not supported for pattern listing", format as u8);
                }
            }
            
            Ok(())
        }

        PatternCommands::Stats { format } => {
            let current_dir = std::env::current_dir()
                .map_err(|e| SniffError::file_system(".", e))?;
            
            let manager = PatternLearningManager::new(&current_dir)?;
            let stats = manager.get_statistics();
            
            match format {
                OutputFormat::Table => {
                    println!("📊 Pattern Learning Statistics");
                    println!("════════════════════════════");
                    println!();
                    println!("Total Patterns: {}", stats.total_patterns);
                    println!("Average Confidence: {:.1}%", stats.average_confidence * 100.0);
                    println!("Total Detections: {}", stats.total_detections);
                    println!();
                    
                    if !stats.patterns_by_language.is_empty() {
                        println!("📋 Patterns by Language:");
                        for (lang, count) in &stats.patterns_by_language {
                            println!("   {}: {}", lang.name(), count);
                        }
                        println!();
                    }
                    
                    if !stats.patterns_by_severity.is_empty() {
                        println!("⚡ Patterns by Severity:");
                        for (severity, count) in &stats.patterns_by_severity {
                            println!("   {}: {}", severity.name(), count);
                        }
                        println!();
                    }
                    
                    if !stats.most_active_patterns.is_empty() {
                        println!("🏆 Most Active Patterns:");
                        for (name, detections) in stats.most_active_patterns.iter().take(5) {
                            println!("   {}: {} detections", name, detections);
                        }
                    }
                }
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&stats).unwrap());
                }
                _ => {
                    println!("❌ Format {} not supported for pattern statistics", format as u8);
                }
            }
            
            Ok(())
        }

        PatternCommands::Delete { pattern_id, confirm } => {
            if !confirm {
                println!("❌ Pattern deletion requires --confirm flag for safety");
                return Ok(());
            }
            
            println!("🚧 Pattern deletion not yet implemented");
            println!("💡 For now, manually edit the learned-patterns.yaml files in .sniff/patterns/");
            Ok(())
        }

        PatternCommands::Export { language, output } => {
            let current_dir = std::env::current_dir()
                .map_err(|e| SniffError::file_system(".", e))?;
            
            let manager = PatternLearningManager::new(&current_dir)?;
            
            let supported_language = match language.to_lowercase().as_str() {
                "rust" => SupportedLanguage::Rust,
                "python" => SupportedLanguage::Python,
                "typescript" => SupportedLanguage::TypeScript,
                "javascript" => SupportedLanguage::JavaScript,
                "go" => SupportedLanguage::Go,
                "c" => SupportedLanguage::C,
                "cpp" => SupportedLanguage::Cpp,
                _ => {
                    println!("❌ Unsupported language: {}", language);
                    return Ok(());
                }
            };
            
            if let Some(playbook) = manager.to_playbook(supported_language) {
                let yaml_content = serde_yaml::to_string(&playbook)
                    .map_err(|e| SniffError::invalid_format("YAML export".to_string(), e.to_string()))?;
                
                if let Some(output_path) = output {
                    std::fs::write(&output_path, yaml_content)
                        .map_err(|e| SniffError::file_system(&output_path, e))?;
                    println!("✅ Exported {} patterns to: {}", language, output_path.display());
                } else {
                    println!("{}", yaml_content);
                }
            } else {
                println!("📝 No learned patterns found for {}", language);
            }
            
            Ok(())
        }

        PatternCommands::Validate { fix: _ } => {
            println!("🚧 Pattern validation not yet implemented");
            println!("💡 Patterns are validated automatically when created");
            Ok(())
        }
    }
}

/// Handles the analyze-files command - analyzes arbitrary files for bullshit patterns.
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
) -> Result<()> {
    use sniff::standalone::{StandaloneAnalyzer, AnalysisConfig, FileFilter, CheckpointManager};
    use sniff::analysis::BullshitAnalyzer;

    info!("🕵️  Starting standalone file analysis");

    // Configure file filter
    let allowed_extensions = extensions.map(|ext| {
        ext.split(',').map(|e| e.trim().to_string()).collect::<Vec<_>>()
    });

    let filter = FileFilter {
        include_hidden,
        allowed_extensions,
        exclude_pattern: exclude,
        max_file_size_bytes: (max_file_size_mb * 1024.0 * 1024.0) as u64,
    };

    // Create analysis config
    let config = AnalysisConfig {
        filter,
        force_language: force_language.map(|lang| match lang.to_lowercase().as_str() {
            "rust" => sniff::SupportedLanguage::Rust,
            "python" => sniff::SupportedLanguage::Python,
            "typescript" => sniff::SupportedLanguage::TypeScript,
            "javascript" => sniff::SupportedLanguage::JavaScript,
            "go" => sniff::SupportedLanguage::Go,
            "c" => sniff::SupportedLanguage::C,
            "cpp" => sniff::SupportedLanguage::Cpp,
            _ => {
                warn!("Unknown language '{}', will auto-detect", lang);
                return None;
            }
        }).flatten(),
        detailed_analysis: detailed,
    };

    // Initialize analyzer
    let bullshit_analyzer = BullshitAnalyzer::new()?;
    let mut analyzer = StandaloneAnalyzer::new(bullshit_analyzer, config);

    // Handle checkpoint comparison if requested
    if let Some(checkpoint_name) = diff_checkpoint {
        let current_dir = std::env::current_dir()
            .map_err(|e| SniffError::file_system(".", e))?;
        let checkpoint_manager = CheckpointManager::new(&current_dir)?;
        
        info!("📊 Comparing against checkpoint: {}", checkpoint_name);
        let comparison = checkpoint_manager.compare_files(&checkpoint_name, &paths).await?;
        
        // Analyze only changed files
        let changed_files: Vec<PathBuf> = comparison.changed_files.into_iter()
            .chain(comparison.new_files.into_iter())
            .collect();
            
        if changed_files.is_empty() {
            println!("✅ No changes detected since checkpoint '{}'", checkpoint_name);
            return Ok(());
        }
        
        println!("📁 Analyzing {} changed files since checkpoint '{}'", 
                changed_files.len(), checkpoint_name);
                
        let results = analyzer.analyze_files(&changed_files).await?;
        display_standalone_results(&results, format, detailed, Some(&comparison))?;
    } else {
        // Analyze specified files/directories
        let results = analyzer.analyze_files(&paths).await?;
        
        // Create checkpoint if requested
        if let Some(checkpoint_name) = checkpoint {
            let current_dir = std::env::current_dir()
                .map_err(|e| SniffError::file_system(".", e))?;
            let checkpoint_manager = CheckpointManager::new(&current_dir)?;
            
            info!("📸 Creating checkpoint: {}", checkpoint_name);
            checkpoint_manager.create_checkpoint(&checkpoint_name, &paths, None).await?;
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

/// Handles checkpoint management commands.
async fn handle_checkpoint_command(command: CheckpointCommands) -> Result<()> {
    use sniff::standalone::CheckpointManager;

    let current_dir = std::env::current_dir()
        .map_err(|e| SniffError::file_system(".", e))?;
    let checkpoint_manager = CheckpointManager::new(&current_dir)?;

    match command {
        CheckpointCommands::Create { name, paths, description } => {
            info!("📸 Creating checkpoint: {}", name);
            checkpoint_manager.create_checkpoint(&name, &paths, description).await?;
            println!("✅ Checkpoint '{}' created with {} files", name, paths.len());
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
                        println!("   Created: {}", checkpoint.timestamp.format("%Y-%m-%d %H:%M:%S"));
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
                        println!("{}: {} files ({})", 
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
                println!("Created: {}", checkpoint.timestamp.format("%Y-%m-%d %H:%M:%S"));
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

        CheckpointCommands::Diff { checkpoint, paths, format } => {
            let comparison_paths = paths.unwrap_or_else(|| {
                // Get paths from checkpoint if not provided
                vec![std::env::current_dir().unwrap()]
            });
            
            let comparison = checkpoint_manager.compare_files(&checkpoint, &comparison_paths).await?;
            
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
                    
                    if comparison.new_files.is_empty() && comparison.changed_files.is_empty() && comparison.deleted_files.is_empty() {
                        println!("✅ No changes detected since checkpoint");
                    }
                }
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&comparison)?);
                }
                _ => {
                    println!("Changes: +{} ~{} -{}", 
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
                        println!("   {} ({})", 
                            file_result.file_path.display(),
                            file_result.language.map(|l| l.name()).unwrap_or("unknown")
                        );
                        println!("      Issues: {} | Quality: {:.1}%", 
                            file_result.detections.len(),
                            file_result.quality_score
                        );
                        
                        if detailed {
                            for detection in &file_result.detections {
                                println!("         {} {} ({}:{}): {}", 
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
                println!("🚨 {} critical issues detected that require immediate attention", results.critical_issues);
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
            println!("| Average quality | {:.1}% |", results.average_quality_score);
            println!();
            
            if !results.file_results.is_empty() {
                println!("## File Analysis");
                println!();
                for file_result in &results.file_results {
                    if !file_result.detections.is_empty() {
                        println!("### `{}`", file_result.file_path.display());
                        println!();
                        println!("- **Language**: {}", file_result.language.map(|l| l.name()).unwrap_or("unknown"));
                        println!("- **Issues**: {}", file_result.detections.len());
                        println!("- **Quality**: {:.1}%", file_result.quality_score);
                        println!();
                        
                        if detailed {
                            println!("#### Issues");
                            println!();
                            for detection in &file_result.detections {
                                println!("- {} **{}** (line {}): `{}`", 
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
                    println!("{}: {} issues, {:.1}% quality", 
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
