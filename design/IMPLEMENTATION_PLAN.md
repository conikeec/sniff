# Sniff Implementation Plan 🐕

AI Bullshit Detector - Sniff out incomplete code and false completion claims

## Project Rename Complete ✅

**From:** `claude-tree` → **To:** `sniff`
- All Rust files updated with new namespace
- Cargo.toml updated with new description and keywords
- CLI commands now use `sniff` binary name
- Database path changed to `~/.claude/sniff.redb`

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        SNIFF ARCHITECTURE                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Claude Session Analysis (CORE - REUSED)                       │
│  ├── Session Processing Engine                                  │
│  ├── File Operations Extraction                                │
│  ├── Merkle Tree Storage                                       │
│  └── Search Index                                              │
│                                                                 │
│  ↓                                                             │
│                                                                 │
│  TreeSitter Code Analysis (NEW)                                │
│  ├── rust-treesitter-agent-code-utility                       │
│  ├── Multi-language AST parsing                                │
│  ├── Symbol extraction                                         │
│  └── Code quality metrics                                      │
│                                                                 │
│  ↓                                                             │
│                                                                 │
│  Playbook-based Detection (NEW)                                │
│  ├── Community-extensible rules                                │
│  ├── Pattern matching engine                                   │
│  ├── Bullshit severity scoring                                 │
│  └── Evidence correlation                                      │
│                                                                 │
│  ↓                                                             │
│                                                                 │
│  Checkpoint Reporting (NEW)                                    │
│  ├── TODO cross-reference                                      │
│  ├── Completion claim validation                               │
│  ├── Interactive review workflow                               │
│  └── Re-iteration tracking                                     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Implementation Phases

### Phase 1: Core Engine Refactoring (Week 1-2)
**Goal:** Strip down to essential components, add bullshit detection foundation

#### 1.1 CLI Simplification
- [ ] Remove wizard interface (`src/wizard.rs`)
- [ ] Remove enhanced search (`src/search.rs`) 
- [ ] Strip main.rs to core commands: `check`, `report`, `rules`
- [ ] Keep core engine: `storage.rs`, `session.rs`, `operations.rs`, `tree.rs`

#### 1.2 New Command Structure
```bash
sniff check                    # Analyze current session for bullshit
sniff check --session <id>    # Analyze specific session
sniff report                   # Generate checkpoint report
sniff report --format json    # JSON format for automation
sniff rules list              # List available detection rules
sniff rules add <file>        # Add custom rule
```

#### 1.3 TreeSitter Integration Setup
- [ ] Add `rust-treesitter-agent-code-utility` dependency
- [ ] Create `src/treesitter.rs` module
- [ ] Basic file analysis pipeline
- [ ] Language detection and parsing

### Phase 2: Detection Engine (Week 3-4)
**Goal:** Implement core bullshit detection patterns

#### 2.1 Core Detection Patterns
```rust
// src/detection.rs
pub enum BullshitPattern {
    PlaceholderCode,      // unimplemented!(), TODO comments
    EmptyFunction,        // Functions with no implementation
    MockStub,             // Mock objects marked as complete
    UnfinishedLogic,      // Incomplete if/else, match arms
    MissingErrorHandling, // unwrap(), expect() without handling
    IncompleteTests,      // Empty test functions
    BrokenReferences,     // Missing imports, undefined functions
}
```

#### 2.2 Playbook System
```yaml
# ~/.sniff/playbooks/rust-basics.yaml
name: "Rust Basics"
version: "1.0"
author: "community"
rules:
  - id: "unimplemented-macro"
    pattern: "unimplemented!()"
    severity: "high"
    description: "Function uses unimplemented! macro"
    
  - id: "todo-comment"
    pattern: "(?i)// TODO:|// FIXME:|// XXX:"
    severity: "medium"
    description: "Code contains TODO/FIXME comments"
    
  - id: "empty-function"
    ast_pattern: "function_item[body=block[statements=empty]]"
    severity: "high"
    description: "Function has empty implementation"
```

#### 2.3 Evidence Correlation
- [ ] Map detected patterns to file operations from session
- [ ] Link to specific TODO items claimed as complete
- [ ] Calculate confidence scores for false completions

### Phase 3: TODO Cross-Reference (Week 5-6)
**Goal:** Cross-reference session TODOs with actual code state

#### 3.1 Session-TODO Mapping
```rust
// src/todo_analysis.rs
pub struct TodoMismatch {
    pub todo_id: String,
    pub claimed_status: String,      // "completed" 
    pub actual_status: String,       // "incomplete"
    pub evidence: Vec<BullshitEvidence>,
    pub file_path: PathBuf,
    pub confidence: f64,
}
```

#### 3.2 Completion Claim Validation
- [ ] Extract file operations from session scope
- [ ] Run TreeSitter analysis on modified files
- [ ] Cross-reference with TODO completion claims
- [ ] Generate evidence-based mismatches

### Phase 4: Checkpoint Reporting (Week 7-8)
**Goal:** Professional reporting system for review and iteration

#### 4.1 Report Generation
```rust
// src/reporting.rs
pub struct CheckpointReport {
    pub session_id: String,
    pub timestamp: DateTime<Utc>,
    pub summary: ReportSummary,
    pub detections: Vec<BullshitDetection>,
    pub todo_mismatches: Vec<TodoMismatch>,
    pub recommendations: Vec<String>,
    pub playbook_coverage: PlaybookCoverage,
}
```

#### 4.2 Interactive Review
- [ ] Present findings with file context
- [ ] Allow user to mark false positives
- [ ] Track review decisions for learning
- [ ] Generate action items for re-iteration

## Core Components to Keep vs Remove

### ✅ **KEEP - Core Engine**
- `src/storage.rs` - Session storage & search index
- `src/session.rs` - Session processing engine  
- `src/operations.rs` - File operation extraction
- `src/tree.rs` - Merkle tree structure
- `src/types.rs` - Core data types
- `src/jsonl.rs` - JSONL parsing
- `src/hash.rs` - Blake3 hashing
- `src/error.rs` - Error handling

### ❌ **REMOVE - Heavy CLI**
- `src/main.rs` - Replace with simple bullshit detector CLI
- `src/wizard.rs` - Natural language interface (too complex)
- `src/search.rs` - Enhanced search (keep basic search only)
- `src/watcher.rs` - File watching (not needed for detection)
- `src/progress.rs` - Progress indicators (simplify)

### 🆕 **ADD - Detection Engine**
- `src/detection.rs` - Core bullshit detection patterns
- `src/treesitter.rs` - TreeSitter integration
- `src/playbook.rs` - Rule management system
- `src/todo_analysis.rs` - TODO cross-reference
- `src/reporting.rs` - Checkpoint reporting

## Sample Usage Vision

```bash
# Quick bullshit check on current session
sniff check

# Detailed analysis with specific playbook
sniff check --playbook strict-rust --verbose

# Generate comprehensive report
sniff report --output checkpoint-2025-01-13.md

# Add custom detection rule
sniff rules add my-patterns.yaml

# List available community rules
sniff rules list --community
```

## Community Extensibility

### Rule Sharing
- GitHub repo: `sniff-community/playbooks`
- Rule rating and feedback system
- Local rule customization
- Built-in rule marketplace

### Pattern Contribution
- Easy YAML/TOML rule format
- AST-based patterns for accuracy
- Regex patterns for simplicity
- Collaborative rule improvement

## Success Metrics

1. **Detection Accuracy**: Catch 90%+ of incomplete implementations
2. **False Positive Rate**: <10% false positives
3. **Community Adoption**: 50+ community-contributed rules
4. **Integration**: Easy to add to CI/CD pipelines
5. **Performance**: <30s analysis time for typical sessions

## Next Steps

1. **Manual Folder Rename**: Move `claude-tree/` → `sniff/`
2. **Start Phase 1**: Strip down CLI and add TreeSitter
3. **Build MVP**: Core detection with 5-10 baseline rules
4. **Community Launch**: Share with rust-treesitter-agent community

Ready to start Phase 1 implementation? 🚀