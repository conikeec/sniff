# Sniff: Advanced Claude Code Session Analysis

Sniff is a Rust-based CLI tool that decomposes Claude Code session histories into a cryptographically-verified Merkle tree structure, enabling real-time analysis, dependency tracking, and intelligent bullshit detection for LLM-generated code.

## Architecture Overview

Sniff transforms Claude Code's monolithic JSONL session files into a hierarchical, indexed structure that supports concurrent access, granular queries, and pattern-based code quality analysis.

```
Claude Code Sessions (JSONL) → Sniff Analysis → Merkle Tree + Pattern Detection
├── Real-time monitoring during active sessions
├── Dependency graph construction
├── Bullshit pattern detection and learning
└── Advanced search and analytics
```

## Installation

```bash
# Build from source
git clone https://github.com/your-org/sniff
cd sniff
cargo build --release

# Install globally
cargo install --path .

# Verify installation
sniff --version
```

## Quick Start

### 1. Initialize Sniff for a Project

```bash
# Navigate to your project directory
cd /path/to/your/project

# Initialize Sniff analysis
sniff patterns init

# This creates:
# .sniff/
# ├── config.toml
# ├── patterns/
# ├── checkpoints/
# └── analysis/
```

### 2. Standalone File Analysis (Works with Any Editor)

```bash
# Analyze specific files
sniff analyze-files src/main.rs src/lib.rs

# Analyze entire directory
sniff analyze-files . --extensions rs,py,ts

# Create checkpoint and analyze changes
sniff analyze-files . --checkpoint "before-refactor"
# ... make changes ...
sniff analyze-files . --diff-checkpoint "before-refactor"

# Analyze with detailed output
sniff analyze-files src/ --detailed --format markdown > analysis-report.md
```

### 3. Analyze Existing Claude Code Sessions

```bash
# Import all Claude Code sessions for current project
sniff scan --project $(pwd | sed 's/\//-/g')

# Analyze specific session
sniff analyze --session session-abc123.jsonl

# View operation timeline
sniff info --session abc123-def456
```

### 4. Real-time Monitoring

```bash
# Monitor active Claude Code sessions
sniff watch

# Monitor with bullshit detection
sniff watch --detect-bullshit --alert-critical
```

## Persistent File Structure

Sniff maintains its analysis data in a `.sniff` directory within your project:

```
.sniff/
├── config.toml              # Sniff configuration
├── tree.redb                # Merkle tree storage (redb database)
├── search.idx/              # Tantivy full-text search index
│   ├── segments/
│   └── meta.json
├── patterns/                # Pattern library
│   ├── rust/
│   │   ├── baseline.yaml    # Built-in patterns
│   │   └── learned.yaml     # Project-specific learned patterns
│   ├── python/
│   │   ├── baseline.yaml
│   │   └── learned.yaml
│   └── typescript/
│       ├── baseline.yaml
│       └── learned.yaml
├── analysis/                # Analysis results and reports
│   ├── sessions/
│   │   ├── abc123-def456.json
│   │   └── xyz789-ghi012.json
│   ├── bullshit-detections.jsonl
│   └── pattern-matches.jsonl
└── cache/                   # Performance caches
    ├── operation-index.db
    └── dependency-graph.db
```

### File Structure Details

**`tree.redb`**: Single-file embedded database containing:
- Merkle tree nodes (projects, sessions, messages, operations)
- Hash-based indices for O(1) lookups
- Parent-child relationship mappings
- Operation dependency graphs

**`search.idx/`**: Tantivy search index enabling:
- Full-text search across message content
- Faceted search by tool, file, timestamp
- Operation metadata queries
- Cross-session content discovery

**`patterns/`**: Language-specific pattern libraries:
- `baseline.yaml`: Built-in bullshit detection patterns
- `learned.yaml`: Project-specific patterns learned from failures
- Pattern definitions with regex, scope, and severity

## Pattern Library System

### Pattern Definition Format

Patterns are defined in YAML files with the following structure:

```yaml
language: "rust"
version: "1.0"
rules:
  - name: "AI Shortcut Comments"
    pattern_type: !Regex
      pattern: "(?i)(for now|todo|hack|temp|quick fix|later|placeholder)"
    scope: !Comments
    severity: !High
    description: "Placeholder comments indicating deferred implementation"
    examples:
      - "// TODO: Add proper error handling later"
      - "// Quick fix for now"
    remediation: "Replace with specific implementation requirements"

  - name: "Hardcoded Magic Numbers"
    pattern_type: !Regex
      pattern: "\\b(3|5|10|100|1000)\\b(?=\\s*[;,)])"
    scope: !FunctionBody
    severity: !Medium
    description: "Suspicious hardcoded values that should be configurable"
    examples:
      - "let timeout = 5000;"
      - "for i in 0..100 {"
```

### Pattern Types and Scopes

**Pattern Types**:
- `!Regex`: Regular expression matching
- `!Literal`: Exact string matching
- `!Semantic`: AST-based pattern matching (future)

**Scopes**:
- `!Comments`: Match within code comments
- `!FunctionBody`: Match within function implementations
- `!Imports`: Match in import/use statements
- `!TopLevel`: Match at module/file level
- `!All`: Match anywhere in the file

**Severity Levels**:
- `!Critical`: Blocks code acceptance
- `!High`: Requires immediate attention
- `!Medium`: Should be addressed soon
- `!Low`: Improvement suggestion

### Built-in Pattern Categories

**AI Shortcuts**:
- Placeholder comments ("TODO", "for now", "quick fix")
- Hardcoded magic numbers
- Bare except clauses (Python)
- Any type usage (TypeScript)
- Unsafe blocks for convenience (Rust)

**Error Handling Issues**:
- Silent error suppression
- Generic error types
- Missing error context
- Unvalidated assumptions

**Code Quality Issues**:
- Deep nesting levels
- Long parameter lists
- Duplicate code blocks
- Missing documentation

## Adding Custom Patterns

### Command-Line Pattern Creation

```bash
# Create a new pattern interactively
sniff pattern create --language rust

# Example interaction:
Pattern name: Unwrap Abuse
Pattern regex: \.unwrap\(\)
Scope: FunctionBody
Severity: High
Description: Direct unwrap() calls that could panic
Example: let value = result.unwrap();

# Add pattern from command line
sniff pattern add \
  --language python \
  --name "Print Debugging" \
  --pattern "print\(" \
  --scope FunctionBody \
  --severity Medium \
  --description "Print statements left in production code"
```

### Manual Pattern Files

Create or edit pattern files directly:

```bash
# Edit project-specific patterns
vim .sniff/patterns/rust/learned.yaml

# Edit global baseline patterns (affects all projects)
vim ~/.sniff/patterns/rust/baseline.yaml
```

### Pattern Learning from Failures

Sniff can automatically learn patterns from compilation failures and user corrections:

```bash
# Learn from recent failures
sniff pattern learn --from-failures --session abc123-def456

# Learn from specific correction
sniff pattern learn \
  --code "let result = unsafe { transmute(data) };" \
  --issue "Unnecessary unsafe usage for type conversion" \
  --language rust \
  --severity High
```

## Standalone File Analysis (Editor Independent)

Sniff can analyze any codebase without requiring Claude Code sessions, making it perfect for integration with Cursor, Windsurf, VS Code, and other editors.

### Basic File Analysis

```bash
# Analyze specific files
sniff analyze-files src/main.rs lib/utils.py

# Analyze directory with file type filtering
sniff analyze-files . --extensions rs,py,ts,js --exclude "target/*,node_modules/*"

# Force language detection for files without extensions
sniff analyze-files scripts/deploy --force-language python

# Analyze with size limits
sniff analyze-files . --max-file-size-mb 5 --include-hidden
```

### Checkpoint-Based Change Tracking

Perfect for tracking code quality over development iterations:

```bash
# Create checkpoint before starting work
sniff checkpoint create --name "pre-refactor" --description "Before API refactoring" src/ tests/

# Work on your code with any editor...

# Analyze only changed files since checkpoint
sniff analyze-files . --diff-checkpoint "pre-refactor"

# Compare current state to checkpoint
sniff checkpoint diff pre-refactor

# List all checkpoints
sniff checkpoint list

# Create analysis report comparing checkpoints
sniff analyze-files . --diff-checkpoint "pre-refactor" --format markdown --output-file quality-report.md
```

### Editor Integration Examples

#### VS Code Integration

Create `.vscode/tasks.json`:

```json
{
    "version": "2.0.0",
    "tasks": [
        {
            "label": "Sniff: Analyze Current File",
            "type": "shell",
            "command": "sniff",
            "args": ["analyze-files", "${file}", "--detailed"],
            "group": "test",
            "presentation": {
                "echo": true,
                "reveal": "always",
                "focus": false,
                "panel": "new"
            }
        },
        {
            "label": "Sniff: Analyze Workspace",
            "type": "shell",
            "command": "sniff",
            "args": ["analyze-files", ".", "--extensions", "rs,py,ts,js", "--format", "json"],
            "group": "test"
        },
        {
            "label": "Sniff: Create Checkpoint",
            "type": "shell",
            "command": "sniff",
            "args": ["checkpoint", "create", "--name", "${input:checkpointName}", "."],
            "group": "build"
        }
    ],
    "inputs": [
        {
            "id": "checkpointName",
            "description": "Checkpoint name",
            "default": "checkpoint-${CURRENT_YEAR}${CURRENT_MONTH}${CURRENT_DATE}",
            "type": "promptString"
        }
    ]
}
```

#### Cursor/Windsurf Integration

Add to your project's `.cursor/rules.md` or similar:

```markdown
# Code Quality Rules

Run Sniff analysis before commits:
- `sniff analyze-files . --critical-only`
- Address all critical issues before proceeding
- Use `sniff checkpoint create` before major changes
- Compare against checkpoints after refactoring
```

#### Git Hooks Integration

Add to `.git/hooks/pre-commit`:

```bash
#!/bin/bash
echo "Running Sniff code quality analysis..."

# Analyze staged files only
STAGED_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\.(rs|py|ts|js)$')

if [ -n "$STAGED_FILES" ]; then
    # Create temporary checkpoint
    sniff checkpoint create --name "pre-commit-$(date +%s)" $STAGED_FILES
    
    # Analyze staged files
    sniff analyze-files $STAGED_FILES --format compact
    
    # Check for critical issues
    if sniff analyze-files $STAGED_FILES --format json | grep -q '"critical_issues":[^0]'; then
        echo "❌ Critical issues detected. Commit blocked."
        echo "Run 'sniff analyze-files $STAGED_FILES --detailed' for details."
        exit 1
    fi
fi

echo "✅ Code quality checks passed"
```

### Advanced Filtering and Analysis

```bash
# Analyze with complex filters
sniff analyze-files . \
  --extensions rs,py,ts \
  --exclude "target/*,node_modules/*,*.test.*,*.spec.*" \
  --max-file-size-mb 2 \
  --detailed

# Language-specific analysis
sniff analyze-files src/ --force-language rust --detailed
sniff analyze-files scripts/ --force-language python --detailed

# Generate comprehensive reports
sniff analyze-files . \
  --checkpoint "analysis-$(date +%Y%m%d)" \
  --format markdown \
  --detailed \
  --output-file "quality-report-$(date +%Y%m%d).md"
```

## Claude Code Integration

### Real-time Session Monitoring

Monitor active Claude Code sessions with automatic bullshit detection:

```bash
# Start monitoring in background
sniff watch --daemon --project /path/to/project

# Monitor with notifications
sniff watch --notify-on high --email alerts@company.com

# Monitor specific session
sniff watch --session-id abc123-def456 --verbose
```

### Checkpoint Analysis

Integrate Sniff into Claude Code workflows using checkpoints:

#### 1. Pre-commit Hook Integration

```bash
# Add to .git/hooks/pre-commit
#!/bin/bash
echo "Running Sniff analysis..."

# Analyze recent changes
sniff analyze --diff HEAD~1..HEAD --fail-on critical

if [ $? -ne 0 ]; then
    echo "❌ Critical bullshit patterns detected. Commit blocked."
    echo "Run 'sniff report --last-analysis' for details."
    exit 1
fi

echo "✅ Code quality checks passed"
```

#### 2. Claude Code Tool Wrapper

Create a wrapper script to analyze after each tool operation:

```bash
#!/bin/bash
# claude-with-sniff.sh

# Run original Claude Code command
claude-code "$@"
CLAUDE_EXIT_CODE=$?

# Analyze if any files were modified
if [ $CLAUDE_EXIT_CODE -eq 0 ]; then
    echo "Analyzing changes with Sniff..."
    sniff analyze --recent-changes --real-time
    
    if sniff status --has-critical-issues; then
        echo "⚠️  Critical issues detected in generated code:"
        sniff report --critical-only --suggest-prompts
    fi
fi

exit $CLAUDE_EXIT_CODE
```

#### 3. VS Code Extension Integration

Configure VS Code to run Sniff analysis on Claude Code operations:

```json
// .vscode/settings.json
{
    "claude-code.onOperationComplete": [
        {
            "command": "sniff analyze --file ${file} --highlight-issues",
            "when": "fileModified"
        }
    ],
    "sniff.realTimeAnalysis": true,
    "sniff.showInlineWarnings": true
}
```

### Generative Loop Integration

#### Continuous Quality Feedback

```bash
# Start Claude Code session with Sniff monitoring
claude-code chat --project /path/to/project &
CLAUDE_PID=$!

# Start Sniff monitoring in parallel
sniff watch --claude-pid $CLAUDE_PID --interactive-feedback

# This enables:
# 1. Real-time analysis of each tool operation
# 2. Immediate feedback on detected patterns
# 3. Suggested prompt improvements
# 4. Learning from user corrections
```

#### Automatic Prompt Refinement

Sniff can suggest better prompts when bullshit patterns are detected:

```bash
# After detecting issues, get prompt suggestions
sniff suggest-prompts --last-analysis

# Example output:
# Detected: Hardcoded timeout values
# Current prompt: "Add error handling to the network client"
# Suggested: "Implement robust network client with configurable timeouts, 
#            exponential backoff retry logic, and comprehensive error handling 
#            for connection failures, timeouts, and server errors"
```

## Advanced Usage

### Dependency Analysis

```bash
# Show operation dependency graph
sniff dependencies --session abc123-def456 --graph

# Analyze impact of specific operation
sniff impact --operation op_12345

# Find operation chains
sniff chains --starting-with "Edit config.rs" --depth 5
```

### Search and Analytics

```bash
# Full-text search across sessions
sniff search "error handling" --last-week

# Find operations by type
sniff search --tool Edit --files "*.rs" --failed-only

# Analytics queries
sniff analytics --most-edited-files --time-range "last month"
sniff analytics --failure-patterns --group-by tool
sniff analytics --bullshit-trends --project-comparison
```

### Pattern Management

```bash
# List all patterns
sniff pattern list --language rust --severity high

# Test pattern against codebase
sniff pattern test --pattern-id "hardcoded-values" --dry-run

# Export patterns for sharing
sniff pattern export --team-pack > team-patterns.yaml

# Import team patterns
sniff pattern import team-patterns.yaml --merge
```

### Reporting

```bash
# Generate comprehensive report
sniff report --session abc123-def456 --format html --output session-report.html

# Bullshit detection summary
sniff report --bullshit-summary --last-week --pdf

# Team analytics dashboard
sniff report --team-dashboard --all-projects --serve localhost:8080
```

## Configuration

### Project Configuration

Edit `.sniff/config.toml`:

```toml
[project]
name = "my-awesome-project"
languages = ["rust", "python", "typescript"]

[watching]
claude_projects_path = "~/.claude/projects"
real_time_analysis = true
auto_learn_patterns = true

[bullshit_detection]
enabled = true
auto_block_critical = false
learning_mode = true
notification_threshold = "high"

[patterns]
use_team_patterns = true
learn_from_failures = true
false_positive_learning = true

[reporting]
auto_generate_daily = true
export_format = ["json", "html"]
team_dashboard_port = 8080
```

### Global Configuration

Edit `~/.sniff/config.toml` for system-wide defaults:

```toml
[defaults]
notification_email = "dev@company.com"
team_pattern_repo = "git@github.com:company/sniff-patterns.git"
analysis_retention_days = 90

[integrations]
slack_webhook = "https://hooks.slack.com/..."
jira_integration = true
github_actions = true
```

## Common Workflows

### Daily Development Workflow

1. **Start monitoring**: `sniff watch --daemon`
2. **Work with Claude Code** normally
3. **Review detections**: `sniff status --daily-summary`
4. **Learn from issues**: `sniff pattern learn --interactive`
5. **Generate report**: `sniff report --daily --share-team`

### Code Review Integration

1. **Analyze PR changes**: `sniff analyze --diff origin/main..HEAD`
2. **Generate review comments**: `sniff report --github-comments`
3. **Update team patterns**: `sniff pattern export --review-feedback`

### Team Onboarding

1. **Set up team patterns**: `sniff pattern import team-patterns.yaml`
2. **Configure notifications**: `sniff config --team-defaults`
3. **Train on project patterns**: `sniff tutorial --project-specific`

## Troubleshooting

### Common Issues

**Database corruption**:
```bash
sniff repair --verify-integrity
sniff repair --rebuild-index
```

**Pattern conflicts**:
```bash
sniff pattern validate --all
sniff pattern resolve-conflicts --interactive
```

**Performance issues**:
```bash
sniff optimize --compact-database
sniff optimize --rebuild-search-index
```

### Debug Mode

```bash
# Run with detailed logging
RUST_LOG=debug sniff watch --verbose

# Analyze with tracing
sniff analyze --trace --session abc123-def456
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup and contribution guidelines.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Documentation

- [Architecture Analysis](docs/claude-code-architecture.md)
- [Merkle Tree Advantages](docs/sniff-merkle-advantages.md) 
- [Bullshit Detector Integration](docs/bullshit-detector-integration.md)