# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Claude Tree CLI is a Rust-based command-line tool for navigating and searching Claude Code session histories. It combines Merkle tree data structures with Tantivy full-text search to provide hierarchical navigation and cross-cutting queries across Claude Code projects and sessions.

## Common Development Commands

### Building and Running
```bash
cargo build                    # Build the project
cargo run                      # Run the application
cargo check                    # Quick syntax check
cargo test                     # Run tests
cargo clippy                   # Run linter
cargo fmt                      # Format code
```

### Development
```bash
cargo build --release          # Build optimized release version
cargo test -- --nocapture      # Run tests with output visible
cargo doc --open              # Generate and open documentation
```

## Architecture Overview

### Core Components (from DESIGN.md)

The project is designed around several key architectural layers:

1. **File System Watcher** - Monitors `~/.claude/projects` for real-time changes
2. **Merkle Tree Implementation** - Hierarchical data structure with cryptographic integrity using Blake3
3. **Search Engine Integration** - Full-text search using Tantivy
4. **Enhanced Operation Processing** - Rich categorization of Claude Code operations (inspired by ccundo)
5. **Storage Layer** - Uses redb (pure Rust embedded database) for persistence

### Data Structure Hierarchy
```
Project Hash (Blake3)
├── Sessions (BTreeMap<SessionId, SessionHash>)
└── Operations (categorized by type: file_edit, bash_command, etc.)

Session Hash (Blake3)  
├── Messages (ordered conversation flow)
├── Operations (chronological timeline)
└── Dependencies (operation dependency graph)
```

### Key Dependencies
- **Storage**: redb (Apache 2.0, pure Rust embedded database)
- **Search**: Tantivy for full-text indexing
- **Hashing**: Blake3 for fast, secure digests
- **CLI**: clap for command parsing, ratatui for TUI
- **File Watching**: notify for real-time monitoring

## Important Implementation Notes

### Storage Engine Choice
The project uses **redb** instead of RocksDB for:
- Apache 2.0 licensing (avoiding GPL complexity)
- Pure Rust implementation (no C++ dependencies)
- Better performance for hash-based lookups
- Simpler deployment (single binary + single database file)

### Operation Types
The system categorizes Claude Code operations into:
- File operations: create, edit, delete, rename
- Directory operations: create, delete  
- Command operations: bash, tool usage
- Status tracking: active, failed, interrupted

### ccundo Inspiration
The project draws inspiration from ccundo for:
- Operation categorization and analysis
- Interactive browsing patterns
- Rich preview functionality with file diffs
- Dependency visualization
- Multi-language support (en/ja)

Note: This is a **read-only analysis tool** - it does not modify files or execute commands, only analyzes Claude Code session history.

## Claude Code JSONL Format Analysis

Based on analysis of actual session files from `~/.claude/projects/`, the JSONL format structure is:

### Message Types

1. **User Messages** (`type: "user"`)
   - Initial user input: `{"role": "user", "content": "text"}`
   - Tool result responses: `{"role": "user", "content": [{"tool_use_id": "...", "type": "tool_result", "content": "..."}]}`

2. **Assistant Messages** (`type: "assistant"`)
   - Text responses: `{"content": [{"type": "text", "text": "..."}]}`
   - Tool use requests: `{"content": [{"type": "tool_use", "id": "...", "name": "ToolName", "input": {...}}]}`

### Common Fields

All messages include:
- `uuid`: Unique message identifier
- `timestamp`: ISO 8601 timestamp
- `parentUuid`: Parent message UUID (null for session start)
- `sessionId`: Session identifier
- `cwd`: Current working directory
- `version`: Claude Code version
- `isSidechain`: Boolean flag
- `userType`: "external"

### Tool Operation Extraction

For operation analysis, key patterns to extract:
- Tool names: `"name": "Bash"`, `"name": "Read"`, `"name": "Edit"`, etc.
- Tool IDs: `"id": "toolu_..."` for linking requests to results
- File paths: In tool inputs and `cwd` field
- Command execution: Bash tool `"input": {"command": "..."}`
- File operations: Read/Edit/Write tools with `"file_path"` parameters
- Tool results: Success/failure status and output content

### Session Structure

Files are named `{session-id}.jsonl` within project directories like:
`~/.claude/projects/-Users-path-to-project/{session-id}.jsonl`

Each line is a complete JSON object representing one message in the conversation flow.

## Current Status

The project is in early development with a basic Rust skeleton. The main implementation will follow the detailed architecture outlined in DESIGN.md, progressing through phases:
1. Core Infrastructure (file watching, Merkle tree, operation processing)
2. Enhanced Operation Analysis (ccundo-inspired features)
3. Search Integration (Tantivy)
4. Interactive TUI
5. Advanced Features (analytics, reporting)
6. Polish and Performance