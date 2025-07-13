# Claude Code Architecture Analysis

## Overview

Claude Code operates as a stateful CLI tool that maintains conversation histories and project contexts through a file-based storage system located in `~/.claude/`. Understanding its architecture reveals both its strengths and fundamental limitations that impact performance, concurrency, and analysis capabilities.

## Storage Architecture

### Projects Directory Structure

```
~/.claude/
├── projects/
│   └── -Users-path-to-project/
│       ├── session-1234.jsonl
│       ├── session-5678.jsonl
│       └── session-9abc.jsonl
└── todos/
    └── project-todos.json
```

### JSONL Session Format

Each session is stored as a single JSONL file where each line represents a message in the conversation:

```json
{"type": "user", "uuid": "msg-1", "parentUuid": null, "content": "Create a new function"}
{"type": "assistant", "uuid": "msg-2", "parentUuid": "msg-1", "content": [{"type": "tool_use", "name": "Edit", "input": {...}}]}
{"type": "user", "uuid": "msg-3", "parentUuid": "msg-2", "content": [{"type": "tool_result", "tool_use_id": "...", "content": "Success"}]}
```

### Message Relationships

Messages are linked through `parentUuid` fields, creating a conversation graph:

```
msg-1 (user)
├── msg-2 (assistant, tools)
│   ├── msg-3 (tool result)
│   └── msg-4 (tool result)
└── msg-5 (follow-up)
```

## Architectural Limitations

### 1. Monolithic Session Files

**Problem**: Entire conversation state stored in single JSONL files

**Implications**:
- File locking prevents concurrent access during active sessions
- Memory overhead increases linearly with conversation length
- No granular access to individual operations or messages
- Parsing requires reading entire file for any query

**Example Impact**:
```
Session with 1000 messages = 50MB+ JSONL file
- Loading session: Full file parse required
- Searching operations: Linear scan through all messages
- Concurrent analysis: Blocked by active session locks
```

### 2. Lock Contention Issues

**Read/Write Lock Problems**:
- Active Claude Code sessions hold exclusive write locks
- Analysis tools cannot access session data during execution
- No read-only access patterns for historical analysis
- Race conditions between tool execution and file updates

**Concurrency Bottleneck**:
```
Claude Code Session (Write Lock)
         ↓
   JSONL File Access
         ↓
Analysis Tool (Blocked) ❌
```

### 3. Inefficient Operation Extraction

**Current Process**:
1. Parse entire JSONL file
2. Filter for tool_use messages
3. Match tool_use with tool_result messages
4. Reconstruct operation timeline manually

**Performance Cost**:
- O(n) scan for each operation query
- No indexed access to operations by type
- Missing dependency relationships between operations
- Tool results scattered across conversation flow

### 4. Limited Search Capabilities

**Text Search Limitations**:
- No full-text indexing
- Grep-style searches across large files
- No semantic search across operation contexts
- Missing faceted search (by tool, file, time range)

**Query Examples**:
```bash
# Current: Expensive file operations
grep -r "Edit.*config" ~/.claude/projects/  # Scans all files
find ~/.claude -name "*.jsonl" -exec grep -l "Failed" {} \;  # No context

# Missing: Indexed queries
search "edit operations on config files last week"
search "failed bash commands in project X"
```

### 5. Metadata Extraction Overhead

**Missing Structured Data**:
- No pre-computed operation summaries
- File modification tracking requires manual parsing
- Command execution status buried in conversation flow
- No dependency graph between operations

**Analysis Requirements**:
```python
# Current: Manual parsing required
for line in session_file:
    message = json.loads(line)
    if is_tool_use(message):
        extract_operation_details(message)
        find_corresponding_result(session_file, tool_use_id)
        # Repeat for every analysis...
```

## Data Access Patterns

### Linear Processing Model

Claude Code's architecture assumes linear, sequential access:

```
Session Start → Message 1 → Message 2 → ... → Message N → Session End
```

**Problems with Analysis Workflows**:
- Cross-session analysis requires multiple full file parses
- No random access to specific operations
- Temporal queries span multiple files
- Aggregation operations require full dataset scans

### Missing Relational Structure

**No Built-in Relationships**:
- Operations not linked to affected files
- Command dependencies not tracked
- File modification chains not maintained
- Tool failure cascades not captured

**Example Analysis Complexity**:
```
Query: "Find all file edits that led to compilation errors"

Current Approach:
1. Parse all session files (expensive)
2. Extract Edit tool operations (manual)
3. Find subsequent Bash operations (pattern matching)
4. Correlate error outputs (heuristic)
5. Build timeline manually (error-prone)

Result: Complex, slow, unreliable
```

## Storage Efficiency Issues

### File System Overhead

**Large File Problems**:
- JSONL files grow unbounded during long sessions
- File system cache invalidation on every append
- Backup and sync overhead for large files
- Disk I/O amplification for small queries

### Redundant Data Storage

**Duplication Issues**:
- Tool schemas repeated in every message
- Working directory paths duplicated
- Session metadata scattered across messages

**Space Utilization**:
```
Typical Session File:
- 70% repetitive structure (UUIDs, timestamps, schema)
- 20% actual content (user input, tool parameters)
- 10% tool results and outputs

Compression ratio could be 3-5x with proper normalization
```

## Concurrent Access Limitations

### Single Writer Model

Claude Code enforces exclusive access during sessions:

```
Session Active: Write Lock Held
├── Analysis tools: Blocked
├── Backup systems: Blocked
└── Monitoring: Blocked
```

**Real-world Impact**:
- Development workflow analysis during active coding
- Real-time monitoring of tool usage patterns
- Continuous integration with session analysis
- Multi-user collaborative analysis

### Missing Read-Only Patterns

**No Historical Analysis Mode**:
- Cannot analyze completed sessions during active development
- Missing read-only snapshots for consistent analysis
- No way to safely export data during active sessions

## Summary of Architectural Constraints

| Aspect | Current Limitation | Impact |
|--------|-------------------|---------|
| **Concurrency** | Single writer, exclusive locks | Blocks analysis during active sessions |
| **Access Patterns** | Linear file processing | O(n) cost for targeted queries |
| **Indexing** | No structured indices | Full file scans for all searches |
| **Relationships** | Manual extraction required | Complex analysis workflows |
| **Storage** | Monolithic JSONL files | Memory and I/O overhead |
| **Scalability** | Per-file processing | Performance degrades with session count |

These limitations become critical as Claude Code usage scales beyond simple script execution to complex, long-running development workflows. The need for concurrent analysis, efficient querying, and relationship tracking drives the requirement for a more sophisticated storage and analysis architecture.

The following documents explore how Sniff addresses these fundamental limitations through Merkle tree-based decomposition and intelligent analysis capabilities.