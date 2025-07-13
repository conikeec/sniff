# Claude Tree CLI - Architecture Document

## Overview

Claude Tree CLI (`claude-tree`) is an advanced Rust-based command-line tool that provides intelligent navigation and search capabilities for Claude Code session histories. It combines Merkle tree data structures with Tantivy full-text search to enable both hierarchical traversal and cross-cutting queries across Claude Code projects and sessions.

## Goals

### Primary Objectives

- **Real-time Monitoring**: Watch `~/.claude/projects` for changes and maintain live indices
- **Hierarchical Navigation**: Leverage natural project → session → message hierarchy with ccundo-inspired interactive browsing
- **Hybrid Search**: Combine tree traversal with full-text search capabilities
- **Context Reconstruction**: Rebuild complete conversation flows with cryptographic integrity
- **Performance**: Sub-second query response times even with large session histories
- **Rich Operation Analysis**: Advanced categorization and preview of Claude Code operations (inspired by ccundo)

### Secondary Objectives

- **Audit Trail**: Cryptographically verifiable session histories
- **Pattern Discovery**: Identify recurring workflows and conversation patterns
- **Interactive CLI**: Rich TUI with operation browsing, preview panes, and cascading dependency views
- **Export Capabilities**: Generate reports and export session data
- **Integration Ready**: API foundation for future GUI/web interfaces

## System Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Claude Tree CLI                            │
├─────────────────────────────────────────────────────────────────┤
│  CLI Interface                                                  │
│  ├── Command Parser (clap)                                     │
│  ├── Interactive TUI (ratatui)                                 │
│  ├── Operation Browser (ccundo-inspired)                       │
│  ├── Preview Engine (file diffs, tool outputs)                 │
│  └── Output Formatting (tabled, colored)                       │
├─────────────────────────────────────────────────────────────────┤
│  Query Engine                                                   │
│  ├── Hybrid Query Processor                                    │
│  ├── Tree Navigation with Cascading Logic                      │
│  ├── Operation Dependency Resolver                             │
│  └── Search Results Merger                                     │
├─────────────────────────────────────────────────────────────────┤
│  Storage Layer                                                  │
│  ├── Merkle Tree Store        │  ├── Tantivy Search Index    │
│  │  ├── Project Nodes         │  │  ├── Message Content      │
│  │  ├── Session Nodes         │  │  ├── Operation Metadata   │
│  │  ├── Message Nodes         │  │  ├── Tool Use Index       │
│  │  └── Operation Nodes       │  │  ├── File Path Index      │
│  └── Hash Index               │  │  └── Temporal Index       │
├─────────────────────────────────────────────────────────────────┤
│  Data Processing Pipeline                                       │
│  ├── File Watcher (notify)                                     │
│  ├── Enhanced JSONL Parser (ccundo-style)                      │
│  ├── Operation Extractor & Categorizer                         │
│  ├── Dependency Chain Builder                                  │
│  ├── Message Validator                                         │
│  └── Multi-Index Updater                                       │
├─────────────────────────────────────────────────────────────────┤
│  Persistence Layer                                              │
│  ├── Tree Database (redb - Pure Rust, Apache 2.0)             │
│  ├── Search Index (tantivy)                                    │
│  ├── Operation Metadata Store                                  │
│  └── Configuration (config.toml)                               │
└─────────────────────────────────────────────────────────────────┘
```

## Component Design

### 1. File System Watcher

**Purpose**: Monitor `~/.claude/projects` for changes in real-time

```rust
struct FileWatcher {
    watcher: notify::RecommendedWatcher,
    event_tx: mpsc::Sender<FileEvent>,
    projects_path: PathBuf,
}

enum FileEvent {
    ProjectCreated(String),
    SessionUpdated(String, String), // project_id, session_id
    SessionDeleted(String, String),
}
```

**Responsibilities**:

- Watch for new project directories
- Monitor JSONL file modifications
- Detect file deletions and handle cleanup
- Debounce rapid file changes

### 2. Merkle Tree Implementation

**Purpose**: Hierarchical data structure with cryptographic integrity

```rust
struct MerkleNode {
    hash: Blake3Hash,
    node_type: NodeType,
    metadata: NodeMetadata,
    children: BTreeMap<String, Blake3Hash>,
    parent: Option<Blake3Hash>,
}

enum NodeType {
    Project { cwd: PathBuf, name: String },
    Session { session_id: String, start_time: DateTime<Utc> },
    Message { uuid: String, timestamp: DateTime<Utc> },
}
```

**Key Features**:

- Blake3 hashing for fast, secure digests
- Immutable nodes with copy-on-write updates
- Efficient parent-child navigation
- Automatic integrity verification

### 3. Search Engine Integration

**Purpose**: Full-text and metadata search using Tantivy

```rust
struct SearchIndex {
    index: tantivy::Index,
    schema: tantivy::schema::Schema,
    writer: tantivy::IndexWriter,
}

struct SearchableMessage {
    hash: String,
    session_id: String,
    project_name: String,
    timestamp: DateTime<Utc>,
    content: String,
    tool_name: Option<String>,
    file_paths: Vec<String>,
    cwd: String,
}
```

**Index Fields**:

- Full-text: message content, tool descriptions
- Faceted: tool names, file extensions, project names
- Temporal: timestamp ranges, duration metrics
- Hierarchical: project and session identifiers

### 4. Enhanced Operation Processing (ccundo-inspired)

**Purpose**: Rich categorization and analysis of Claude Code operations

```rust
struct Operation {
    tool_use_id: String,
    operation_type: OperationType,
    file_path: Option<PathBuf>,
    content_diff: Option<FileDiff>,
    command: Option<String>,
    timestamp: DateTime<Utc>,
    status: OperationStatus,
    dependencies: Vec<String>, // Other tool_use_ids this depends on
    children: Vec<String>,     // Operations that depend on this one
    tokens_used: Option<u32>,
    execution_time_ms: Option<u64>,
}

enum OperationType {
    FileEdit, FileCreate, FileDelete, FileRename,
    DirectoryCreate, DirectoryDelete,
    BashCommand, WebFetch, Read, Write, View,
    GlobTool, GrepTool, LS, NotebookRead, NotebookEdit,
}

enum OperationStatus {
    Active,      // Normal completed operation
    Failed,      // Operation that failed
    Interrupted, // Operation that was interrupted
}

struct FileDiff {
    additions: Vec<String>,
    deletions: Vec<String>,
    file_type: Option<String>,
    line_count_change: i32,
}
```

**Key Features**:

- Automatic operation categorization from tool_use messages
- File diff extraction and analysis
- Dependency chain construction based on temporal ordering and file relationships
- Rich metadata for search and filtering
**Purpose**: Combine tree navigation with search capabilities

```rust
enum Query {
    Tree(TreeQuery),
    Search(SearchQuery),
    Hybrid(TreeQuery, SearchQuery),
}

struct TreeQuery {
    project: Option<String>,
    session: Option<String>,
    message_path: Vec<String>,
}

struct SearchQuery {
    text: Option<String>,
    tool_filter: Vec<String>,
    time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
    file_filter: Vec<String>,
}
```

## Data Structures

### Project Hierarchy

```
Project Hash (Blake3)
├── metadata: { cwd, name, created_at, total_operations }
├── sessions: BTreeMap<SessionId, SessionHash>
├── operation_summary: OperationSummary
└── search_refs: Vec<TantivyDocId>

Session Hash (Blake3)  
├── metadata: { session_id, start_time, end_time, message_count }
├── messages: BTreeMap<Timestamp, MessageHash>
├── operations: BTreeMap<ToolUseId, OperationHash>
├── conversation_flow: Vec<MessageHash> // ordered by parentUuid
├── operation_timeline: Vec<OperationHash> // chronological operations
├── parent_project: ProjectHash
└── dependency_graph: OperationDependencyGraph

Message Hash (Blake3)
├── metadata: { uuid, timestamp, tool_name, tokens, role }
├── content: MessageContent
├── parent_session: SessionHash
├── parent_message: Option<MessageHash> // from parentUuid
├── children: Vec<MessageHash>
├── associated_operations: Vec<OperationHash> // tool_use operations in this message
└── search_ref: TantivyDocId

Operation Hash (Blake3) // New from ccundo inspiration
├── metadata: { tool_use_id, timestamp, operation_type, status }
├── file_operations: Option<FileOperationDetails>
├── command_details: Option<CommandDetails>
├── parent_message: MessageHash
├── dependencies: Vec<OperationHash> // operations this depends on
├── dependents: Vec<OperationHash>   // operations that depend on this
├── cascade_impact: CascadeMetrics
└── search_ref: TantivyDocId

struct OperationSummary {
    file_edits: u32,
    file_creates: u32,
    file_deletes: u32,
    bash_commands: u32,
    total_files_affected: HashSet<PathBuf>,
    most_edited_files: Vec<(PathBuf, u32)>,
}

struct OperationDependencyGraph {
    nodes: BTreeMap<ToolUseId, OperationHash>,
    edges: Vec<(ToolUseId, ToolUseId)>, // (parent, child)
    cascade_chains: Vec<Vec<ToolUseId>>, // sequences of dependent operations
}
```

### Storage Layout

```
~/.claude-tree/
├── config.toml
├── tree.redb            # redb database file (single file)
├── search.idx/          # Tantivy search index
│   ├── segments/
│   └── meta.json
└── logs/
    └── claude-tree.log
```

## API Design

### CLI Commands

```bash
# Basic navigation (enhanced with ccundo patterns)
claude-tree ls                           # List all projects with operation summaries
claude-tree ls <project>                 # List sessions with operation counts
claude-tree show <project> <session>     # Show conversation flow with operations

# ccundo-inspired browsing
claude-tree browse                       # Interactive TUI for project/session navigation
claude-tree operations <project>         # List all operations in project
claude-tree operations <project> <session> # List operations in session
claude-tree preview <operation-id>       # Show operation details and file diffs
claude-tree cascade <operation-id>       # Show operation dependencies and impact

# Enhanced search operations
claude-tree search "file operations"     # Full-text search
claude-tree search --tool Bash          # Filter by tool type (ccundo-style)
claude-tree search --type file_edit     # Filter by operation type
claude-tree search --files "*.py"       # File-based search
claude-tree search --since "1 week ago" # Time-based search

# Operation analysis (new ccundo-inspired features)
claude-tree analyze <project>            # Show operation patterns and statistics
claude-tree files <project>              # Show most frequently modified files
claude-tree timeline <project> <session> # Show chronological operation timeline
claude-tree dependencies <operation-id>  # Show operation dependency chain

# Hybrid queries
claude-tree query <project> --search "error" # Search within project
claude-tree trace <message-hash>              # Show conversation context
claude-tree flow <session-id>                 # Show conversation + operation flow

# Session management (ccundo-inspired)
claude-tree sessions                     # List all sessions across projects
claude-tree session <session-id>        # Switch focus to specific session
claude-tree session <session-id> --ops  # Show operations for session

# Tree operations
claude-tree verify <project>             # Verify tree integrity
claude-tree export <project> --format json # Export session data
claude-tree stats                        # Show usage statistics
claude-tree compact                      # Optimize tree storage

# Interactive modes
claude-tree interactive                  # Launch TUI interface (enhanced)
claude-tree watch                        # Real-time monitoring mode
```

### Configuration

```toml
[watching]
claude_projects_path = "~/.claude/projects"
watch_interval_ms = 100
debounce_ms = 500

[storage]
data_directory = "~/.claude-tree"
max_cache_size_mb = 512
compression_enabled = true

[search]
max_results = 100
highlight_enabled = true
fuzzy_matching = true

[tree]
hash_algorithm = "blake3"
verification_on_read = true
auto_compact_threshold = 1000

# ccundo-inspired display settings
[display]
language = "en"  # ccundo supports en/ja
default_format = "table"
color_enabled = true
pager_enabled = true
time_format = "relative"  # "2m ago" vs absolute timestamps
operation_limit = 50      # Max operations to show in lists
cascading_preview = true  # Show dependent operations in previews

# Enhanced operation filtering (ccundo-inspired)
[filters]
default_operation_types = ["file_edit", "file_create", "bash_command"]
hide_read_operations = false  # Hide Read/View operations by default
show_token_usage = true
show_file_diffs = true
max_cascade_depth = 10    # Limit dependency chain traversal

# Operation categorization settings
[operations]
auto_detect_file_types = true
extract_file_diffs = true
track_command_outputs = false  # Don't store bash command outputs (privacy)
dependency_analysis = true     # Build operation dependency graphs
cascade_analysis = true        # Analyze cascading operation impacts

# TUI settings (ccundo-inspired)
[tui]
vim_bindings = true
preview_pane_size = 40  # Percentage of screen for preview
auto_preview = true     # Show preview on selection
highlight_dependencies = true
show_operation_icons = true
```

## Implementation Strategy

### Phase 1: Core Infrastructure (5-7 weeks)

1. **File Watcher Setup**
   - Implement basic directory monitoring
   - Enhanced JSONL parsing with ccundo-style operation extraction
   - Error handling and recovery

2. **Merkle Tree Foundation**
   - Basic tree data structures with operation nodes
   - Blake3 hashing implementation
   - RocksDB persistence layer

3. **Operation Processing Pipeline**
   - Tool use ID extraction and categorization
   - Operation type classification (file_edit, file_create, etc.)
   - Basic dependency detection

4. **CLI Framework**
   - Command parsing with clap
   - Basic output formatting with operation summaries
   - Configuration management

### Phase 2: Enhanced Operation Analysis (4-5 weeks)

1. **ccundo-inspired Operation Features**
   - File diff extraction and analysis
   - Operation status tracking (active/failed/interrupted)
   - Cascading dependency analysis
   - Operation timeline construction

2. **Rich CLI Commands**
   - `operations`, `preview`, `cascade` commands
   - Interactive operation browsing
   - Enhanced listing with operation metadata

### Phase 3: Search Integration (3-4 weeks)

1. **Tantivy Integration**
   - Enhanced schema with operation metadata
   - Real-time index updates for operations
   - Multi-faceted search (content, operations, files, tools)

2. **Hybrid Query Engine**
   - Query parsing with operation filters
   - Result merging algorithms
   - Performance optimization

### Phase 4: Interactive TUI (5-6 weeks)

1. **ccundo-inspired TUI Interface**
   - Multi-pane layout (projects/sessions/operations/preview)
   - Rich operation browser with cascading visualization
   - File diff viewer and command output display
   - Keyboard navigation with vim bindings

2. **Advanced Visualization**
   - Operation dependency graphs
   - Conversation flow with operation annotations
   - Timeline view with operation markers

### Phase 5: Advanced Features (4-5 weeks)

1. **Analytics and Reporting**
   - Operation pattern analysis
   - File modification statistics
   - Workflow optimization insights
   - Multiple export formats

2. **Multi-language Support**
   - ccundo-style language preferences (en/ja)
   - Localized operation descriptions
   - Cultural formatting preferences

### Phase 6: Polish and Performance (2-3 weeks)

1. **Performance Optimization**
   - Query plan optimization for operation searches
   - Memory usage reduction for large operation sets
   - Concurrent processing of operation analysis

2. **Documentation and Testing**
   - Comprehensive test suite with operation scenarios
   - User documentation with ccundo migration guide
   - API documentation for operation structures

## Performance Considerations

### Memory Management

- **Lazy Loading**: Load tree nodes on-demand
- **LRU Caching**: Keep frequently accessed nodes in memory
- **Streaming**: Process large JSONL files without full memory load

### Query Optimization

- **Index Prewarming**: Keep search indices memory-resident
- **Query Planning**: Optimize hybrid queries based on selectivity
- **Result Caching**: Cache frequently accessed conversation threads

### Concurrency

- **Async File I/O**: Non-blocking file system operations
- **Parallel Indexing**: Index multiple sessions concurrently
- **Reader-Writer Locks**: Allow concurrent reads with exclusive writes

## Error Handling

### File System Errors

- Graceful handling of permission errors
- Recovery from corrupted JSONL files
- Automatic cleanup of orphaned indices

### Data Integrity

- Hash verification on tree access
- Automatic index rebuilding on corruption
- Backup and recovery mechanisms

### User Experience

- Progressive error recovery
- Detailed error messages with suggestions
- Rollback capabilities for failed operations

## Security Considerations

### Data Privacy

- No external network communication
- Local-only data processing
- Optional data encryption at rest

### Integrity

- Cryptographic verification of all data
- Immutable tree structure
- Audit trail for all modifications

## Storage Engine Choice: redb

### Why redb over RocksDB

**Licensing**: redb uses Apache 2.0 license, avoiding RocksDB's dual Apache/GPL licensing complexity.

**Pure Rust**: 100% Rust implementation eliminates C++ dependencies and compilation complexity.

**Performance**: Benchmarks show redb often outperforming RocksDB:

- Individual writes: 395ms (redb) vs 1129ms (RocksDB)
- Random reads: 975ms (redb) vs 3197ms (RocksDB)
- Memory-safe with zero-copy reads

**Architecture Alignment**:

- B-tree storage with copy-on-write semantics is ideal for Merkle tree operations
- ACID transactions with MVCC support concurrent reads during file watching
- Single-file storage simplifies deployment and backup
- Savepoints feature could enable checkpointing functionality

**Production Ready**: Version 1.0+ with stable file format, though newer than RocksDB.

### redb Integration Strategy

```rust
// Tree storage with multiple tables for different node types
const PROJECT_NODES: TableDefinition<&str, &[u8]> = TableDefinition::new("projects");
const SESSION_NODES: TableDefinition<&str, &[u8]> = TableDefinition::new("sessions");  
const MESSAGE_NODES: TableDefinition<&str, &[u8]> = TableDefinition::new("messages");
const OPERATION_NODES: TableDefinition<&str, &[u8]> = TableDefinition::new("operations");

// Hash-based indexing tables
const HASH_TO_NODE: TableDefinition<&str, &str> = TableDefinition::new("hash_index");
const PROJECT_SESSIONS: TableDefinition<&str, &str> = TableDefinition::new("project_sessions");

struct TreeDatabase {
    db: Database,
}

impl TreeDatabase {
    pub fn new(path: PathBuf) -> Result<Self> {
        let db = Database::create(path)?;
        Ok(Self { db })
    }
    
    pub fn store_node(&self, hash: &str, node_type: NodeType, data: &[u8]) -> Result<()> {
        let write_txn = self.db.begin_write()?;
        {
            let mut table = match node_type {
                NodeType::Project => write_txn.open_table(PROJECT_NODES)?,
                NodeType::Session => write_txn.open_table(SESSION_NODES)?,
                NodeType::Message => write_txn.open_table(MESSAGE_NODES)?,
                NodeType::Operation => write_txn.open_table(OPERATION_NODES)?,
            };
            table.insert(hash, data)?;
        }
        write_txn.commit()?;
        Ok(())
    }
}
```

**Benefits for Claude Tree CLI**:

- Simpler deployment (single binary + single database file)
- Better performance for hash-based lookups
- Native Rust error handling and type safety
- Easier cross-compilation and distribution
- No external C++ runtime dependencies

### Adopted Features (Read-Only Analysis)

**Operation Categorization**:

- File operations: create, edit, delete, rename
- Directory operations: create, delete
- Command operations: bash, tool usage
- Enhanced metadata extraction from tool_use messages

**Interactive CLI Patterns**:

- `claude-tree browse` - Interactive project/session/operation navigation
- `claude-tree preview <operation-id>` - Rich operation previews with file diffs
- `claude-tree cascade <operation-id>` - Dependency visualization
- `claude-tree operations` - Operation listing with filters

**Rich Display Features**:

- Relative timestamps ("2m ago", "5m ago")
- Operation status indicators ([ACTIVE], [FAILED], [INTERRUPTED])
- File diff previews with syntax highlighting
- Cascading operation counts ("+2 more would be affected")
- Tool use ID display for cross-referencing

**Multi-Session Management**:

- `claude-tree sessions` - List sessions across all projects
- `claude-tree session <id>` - Focus on specific session
- Cross-project operation analysis

**Configuration and Internationalization**:

- Language preferences (en/ja support)
- Display customization (time format, operation limits)
- Filter presets for operation types

### Excluded Features (System-Modifying)

**File System Modifications**:

- No actual undo/redo functionality that modifies files
- No backup creation or restoration capabilities
- No file system writes beyond our own index files

**Command Execution**:

- No execution of bash commands or tool operations
- No integration with Claude Code's execution environment
- Purely analytical and exploratory

**State Modification**:

- No modification of Claude Code session files
- No intervention in active Claude Code sessions
- Read-only access to all Claude Code data

### Migration Path from ccundo

For users currently using ccundo, Claude Tree CLI provides complementary functionality:

1. **Enhanced Exploration**: While ccundo focuses on undoing operations, Claude Tree CLI focuses on understanding and analyzing operation history
2. **Cross-Session Analysis**: Analyze patterns across multiple sessions and projects
3. **Long-term History**: Maintain searchable history of all Claude Code activities
4. **Workflow Optimization**: Identify recurring patterns and optimization opportunities

Users can run both tools simultaneously:

- Use ccundo for immediate undo/redo needs
- Use Claude Tree CLI for historical analysis and workflow insights

### Implementation Priority for ccundo Features

**Phase 1 (Essential)**:

- Operation extraction and categorization
- Basic interactive browsing
- Preview functionality

**Phase 2 (Enhanced)**:

- Cascading dependency analysis
- Rich TUI with multi-pane layout
- Advanced filtering and search

**Phase 3 (Advanced)**:

- Multi-language support
- Advanced analytics and pattern detection
- Export capabilities for workflow documentation

## Future Extensions

### Integration Possibilities

- **Claude Code Plugin**: Direct integration with Claude Code CLI
- **Web Interface**: React-based web UI for visualization
- **API Server**: REST API for external tool integration
- **VS Code Extension**: IDE integration for session browsing

### Advanced Analytics

- **Workflow Mining**: Discover common development patterns
- **Performance Analysis**: Token usage and timing analytics
- **Collaboration Features**: Multi-user session sharing
- **Machine Learning**: Predictive conversation completion

### Export Capabilities

- **Session Replay**: Reconstruct exact conversation flows
- **Documentation Generation**: Auto-generate development docs
- **Training Data**: Export for LLM fine-tuning
- **Audit Reports**: Compliance and usage reporting

## Dependencies

### Core Dependencies

```toml
[dependencies]
tokio = { version = "1.0", features = ["full"] }
clap = { version = "4.0", features = ["derive"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
notify = "6.0"
tantivy = "0.21"
redb = "2.1"              # Pure Rust embedded database (Apache 2.0)
blake3 = "1.4"
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1.0"
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"

# Enhanced TUI dependencies (ccundo-inspired)
ratatui = "0.24"         # Modern TUI framework (replacement for tui)
crossterm = "0.27"       # Terminal manipulation
tabled = "0.14"          # Table formatting
colored = "2.0"          # Colored output
console = "0.15"         # Enhanced console utilities

# File processing and diff analysis
similar = "2.3"          # Text diffing library
ignore = "0.4"           # gitignore-style file filtering
walkdir = "2.4"          # Directory traversal
mime_guess = "2.0"       # File type detection

# Enhanced CLI features
dialoguer = "0.11"       # Interactive CLI prompts (ccundo-style selection)
indicatif = "0.17"       # Progress bars and spinners
fuzzy-matcher = "0.3"    # Fuzzy search in TUI
unicode-width = "0.1"    # Better text width calculation

# Configuration and internationalization
config = "0.13"          # Configuration file handling
fluent = "0.16"          # Internationalization (for multi-language support)
```

### Development Dependencies

```toml
[dev-dependencies]
criterion = "0.5"      # Benchmarking
proptest = "1.0"       # Property-based testing
tempfile = "3.0"       # Temporary directories for tests
wiremock = "0.5"       # Mock external services
```

## Conclusion

Claude Tree CLI represents a sophisticated approach to managing and exploring Claude Code session histories. By combining the hierarchical benefits of Merkle trees with the search capabilities of Tantivy, it provides both structural integrity and flexible querying capabilities.

The hybrid architecture ensures that users can efficiently navigate large amounts of conversation data while maintaining cryptographic verification of the data's integrity. The real-time monitoring capabilities make it a living tool that grows with the user's Claude Code usage.

This foundation creates opportunities for advanced analytics, workflow optimization, and integration with other development tools, making it a valuable addition to any AI-assisted development workflow.
