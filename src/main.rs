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

    /// Database maintenance operations
    Db {
        #[command(subcommand)]
        command: DbCommands,
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

        Commands::Db { command } => handle_db_command(database_path, command).await,

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
