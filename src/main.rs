// Copyright (c) 2025 Chetan Conikee <conikee@gmail.com>
// Licensed under the MIT License

//! Claude Tree CLI - Advanced navigation and search for Claude Code session histories.

use clap::{Parser, Subcommand};
use claude_tree::{ClaudeTreeError, Result};
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::fmt;

/// Claude Tree CLI - Advanced navigation and search for Claude Code session histories.
#[derive(Parser)]
#[command(
    name = "claude-tree",
    version = env!("CARGO_PKG_VERSION"),
    author = "Chetan Conikee <conikee@gmail.com>",
    about = "Advanced CLI tool for navigating and searching Claude Code session histories"
)]
struct Cli {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// Path to Claude projects directory
    #[arg(long, default_value = "~/.claude/projects")]
    projects_path: PathBuf,

    /// Subcommand to execute
    #[command(subcommand)]
    command: Commands,
}

/// Available CLI commands.
#[derive(Subcommand)]
enum Commands {
    /// Parse and analyze a specific session file
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

    /// Watch for changes in the Claude projects directory
    Watch {
        /// Process existing files on startup
        #[arg(long, default_value = "true")]
        process_existing: bool,
    },

    /// List all discovered projects and sessions
    List {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Show statistics about Claude Code usage
    Stats {
        /// Project to analyze (optional)
        project: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };
    fmt().with_max_level(log_level).with_target(false).init();

    info!("Starting Claude Tree CLI v{}", env!("CARGO_PKG_VERSION"));

    // Execute the selected command
    match cli.command {
        Commands::Parse {
            file,
            max_lines,
            analyze_operations,
        } => handle_parse_command(file, max_lines, analyze_operations).await,
        Commands::Watch { process_existing } => {
            handle_watch_command(cli.projects_path, process_existing).await
        }
        Commands::List { detailed } => handle_list_command(cli.projects_path, detailed).await,
        Commands::Stats { project } => handle_stats_command(cli.projects_path, project).await,
    }
}

/// Handles the parse command.
async fn handle_parse_command(
    file: PathBuf,
    max_lines: Option<usize>,
    analyze_operations: bool,
) -> Result<()> {
    use claude_tree::jsonl::{JsonlParser, ParseConfig};
    use claude_tree::operations::OperationExtractor;

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

        use claude_tree::operations::OperationStats;
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

/// Handles the watch command.
async fn handle_watch_command(projects_path: PathBuf, process_existing: bool) -> Result<()> {
    use claude_tree::watcher::{ClaudeWatcher, WatcherConfig};

    info!("Starting file watcher for: {}", projects_path.display());

    let config = WatcherConfig {
        claude_projects_path: expand_path(projects_path),
        process_existing,
        ..Default::default()
    };

    let (watcher, mut event_receiver) = ClaudeWatcher::new(config)?;

    // Spawn the watcher in a background task
    let watcher_handle = tokio::spawn(async move {
        if let Err(e) = watcher.start_watching().await {
            eprintln!("Watcher error: {}", e);
        }
    });

    // Process events
    while let Some(event) = event_receiver.recv().await {
        println!("Event: {:?}", event);
    }

    watcher_handle.await.map_err(|e| {
        ClaudeTreeError::operation_extraction(format!("Watcher task failed: {}", e))
    })?;

    Ok(())
}

/// Handles the list command.
async fn handle_list_command(projects_path: PathBuf, detailed: bool) -> Result<()> {
    use claude_tree::watcher::utils;

    let expanded_path = expand_path(projects_path);
    info!("Listing projects in: {}", expanded_path.display());

    let projects = utils::discover_projects(&expanded_path)?;

    if projects.is_empty() {
        println!("No projects found in {}", expanded_path.display());
        return Ok(());
    }

    println!("Found {} project(s):", projects.len());

    for project_path in projects {
        let project_name = utils::extract_project_name(&project_path);
        println!("  📁 {}", project_name);

        if detailed {
            let sessions = utils::discover_sessions(&project_path)?;
            println!("    Sessions: {}", sessions.len());

            for session_path in sessions.iter().take(5) {
                if let Some(session_name) = session_path.file_stem().and_then(|s| s.to_str()) {
                    println!("      📄 {}", session_name);
                }
            }

            if sessions.len() > 5 {
                println!("      ... and {} more", sessions.len() - 5);
            }
        }
    }

    Ok(())
}

/// Handles the stats command.
async fn handle_stats_command(_projects_path: PathBuf, _project: Option<String>) -> Result<()> {
    // TODO: Implement comprehensive statistics
    println!("Statistics command not yet implemented");
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
