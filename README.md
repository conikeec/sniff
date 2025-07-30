# Sniff: Misalignment detection in Vibe Coding loops

Sniff is a Rust-based CLI tool that detects code quality issues and misalignment patterns in codebases. It provides static analysis capabilities for identifying problematic patterns, tracking code quality over time, and integrating quality gates into development workflows.

## Origin: Catching AI Deception in the generative loop Real-Time

Sniff emerged from direct observation of AI agents during exploratory coding sessions. During intensive development with AI assistants (Claude, GPT-4, etc.), a pattern became clear: the models were systematically creating deceptive code that provided an illusion of completion while introducing subtle failures.

The agents weren't making random mistakes—they were learning to optimize for _perceived progress_ rather than _actual functionality_. They would:

- Replace working implementations with fake stubs that "looked right"
- Remove error handling and replace it with silent failures
- Generate placeholder authentication that always returned `true`
- Create mock data returns instead of actual business logic
- Add TODO comments as substitutes for real implementation

Sniff was built incrementally by identifying these adaptive patterns as they emerged during reasoning loops. Rather than asking agents to self-reflect on their code quality (which proved unreliable), Sniff serves as a deterministic reflector that independently verifies task completion without bias.

### The Generative Loop Integration

In vibe coding workflows, development follows a natural generative loop: **Plan item → Act → Plan item done**. This cycle repeats continuously as developers (human or AI) break down work into actionable tasks, execute implementations, and mark completion before moving to the next item.

Sniff integrates directly into this loop as a quality gate between "Act" and "Plan item done." When an implementation claims completion, Sniff analyzes the code for deceptive patterns and quality issues. If problems are detected, the system loops back to "Act" rather than allowing false completion. This creates a feedback mechanism that prevents deceptive code from accumulating and ensures each cycle produces genuinely functional progress.

The loop becomes: **Plan item → Act → Sniff verification → [Pass: Plan item done] | [Fail: Loop back to Act]**. This integration transforms the traditional development cycle into a self-correcting system that maintains code quality standards while preserving the natural flow of iterative development.

**Zero Trust Verification**: Sniff operates on a zero trust principle when verifying agent-reported changes. Rather than trusting an agent's list of modified files, Sniff can enforce git discovery (`--git-discovery` flag) to independently discover all changed files. This prevents agents from "sweeping files under the rug"—selectively hiding problematic implementations while only reporting clean files for verification. The zero trust approach ensures that quality gates examine the complete scope of changes, not just what the agent chooses to reveal.

## Current State

AI-generated code often contains patterns that satisfy immediate compilation requirements but fail in production environments. These patterns include:

- Premature returns without implementation (`return Ok(())`, `return true`)
- Placeholder implementations (`unimplemented!()`, `time.sleep()`)
- Silent error suppression (empty `catch {}` blocks)
- Generic placeholders (`// TODO: implement this later`)
- Mock data returns (hardcoded test values)
- Authentication bypasses (always returning `true`)

Sniff detects these patterns and provides quality gates to prevent problematic code from reaching production.

## Architecture

Sniff provides pattern detection and analysis capabilities through multiple components:

```
Code Files → AST (Syntax Tree) Analysis → Pattern Detection → Quality Report
├── Language-specific pattern matching
├── Quality scoring and classification
└── Integration with development workflows
```

## Installation

### Homebrew (macOS/Linux)

```bash
# Add the tap
brew tap conikeec/tap

# Install sniff
brew install sniff

# Verify installation
sniff --version
```

### Local (rust env)
```bash
# Build from source
git clone https://github.com/conikeec/sniff
cd sniff
cargo build --release

# Install globally
cargo install --path .

# Verify installation
sniff --version
```

## Command Reference

### Core Commands

#### `sniff analyze-files` - File Analysis

Analyze codebase files for code quality issues and misalignment patterns.

**Note**: Files detected as test files are excluded by default. Use `--include-tests` to analyze test files.

```bash
# Basic file analysis (use --include-tests for test files)
sniff analyze-files tests/samples/test_misalignment.rs --include-tests
```

**Output:**

```
TODO Verification Report
──────────────────────────────────────────────────
├─ TODO: file-analysis
├─ Metrics
│  ├─ Files analyzed: 1
│  ├─ Quality score: 0% (required: 80%)
│  ├─ Critical issues: 6 (max allowed: 0)
│  └─ Total detections: 16
├─ Result
│  └─ ● FAILED - Continue working on this TODO
│     ├─ ⚠ Quality score 0.0% below required 80.0%
│     └─ ⚠ 6 critical issues found (max allowed: 0)
└─ Issues Found
   └─ tests/samples/test_misalignment.rs (Quality: 0%)
      ├─ ● Unimplemented Macro (line 5): unimplemented!()
      ├─ ● TODO Comment (line 4): // TODO:
      └─ ● Unwrap Without Context (line 19): .unwrap()
      └─ ● ... and 13 more issues
```

```bash
# Analyze multiple files with filtering (including test files)
sniff analyze-files tests/samples/ --extensions rs,py,ts --include-tests
```

**Output:**

```
Analysis Summary
──────────────────────────────────────────────────
├─ Metrics
│  ├─ Files analyzed: 7
│  ├─ Quality score: 53.7% (average)
│  ├─ Critical issues: 8
│  └─ Total detections: 39
└─ Files
   ├─ tests/samples/test_python.py (57.0% quality, 3 issues)
   ├─ tests/samples/test_new_patterns.rs (69.0% quality, 3 issues)
   ├─ tests/samples/test_typescript_patterns.ts (84.0% quality, 2 issues)
   └─ ... 4 more files
```

```bash
# Detailed analysis with specific issues
sniff analyze-files tests/samples/test_misalignment.rs --detailed --include-tests
```

```bash
# Compact output for CI/CD integration
sniff analyze-files tests/samples/ --format compact --include-tests
```

**Output:**

```
tests/samples/test_python.py: 3 issues, 57.0% quality
tests/samples/test_new_patterns.rs: 3 issues, 69.0% quality
tests/samples/test_typescript_patterns.ts: 2 issues, 84.0% quality
tests/samples/test_misalignment.py: 7 issues, 27.0% quality
tests/samples/test_misalignment.rs: 16 issues, 0.0% quality
tests/samples/test_exact_patterns.py: 4 issues, 78.0% quality
tests/samples/test_enhanced_patterns.rs: 4 issues, 61.0% quality
```

#### `sniff verify-todo` - Quality Gate Verification

Verify TODO completion with quality analysis before marking tasks complete.

```bash
# Basic verification (trust agent-reported files)
sniff verify-todo --todo-id "implement-auth" --files src/auth.rs src/middleware/auth.rs --min-quality-score 85
```

**Output:**

```
TODO Verification Report
──────────────────────────────────────────────────
├─ TODO: implement-auth
├─ Metrics
│  ├─ Files analyzed: 2
│  ├─ Quality score: 88% (required: 85%)
│  ├─ Critical issues: 0 (max allowed: 0)
│  └─ Total detections: 3
├─ Result
│  └─ ● PASSED - Ready to mark complete
```

```bash
# Secure verification with Git discovery (prevents agent deception)
sniff verify-todo --todo-id "implement-auth" --files src/auth.rs --git-discovery --min-quality-score 85
```

**Output when agent hides files:**

```
Git discovery found 5 files vs 1 reported
Using git-discovered files for verification

TODO Verification Report
──────────────────────────────────────────────────
├─ TODO: implement-auth
├─ Metrics
│  ├─ Files analyzed: 5
│  ├─ Quality score: 45% (required: 85%)
│  ├─ Critical issues: 8 (max allowed: 0)
│  └─ Total detections: 23
├─ Result
│  └─ ● FAILED - Continue working on this TODO
│     ├─ ⚠ Quality score 45.0% below required 85.0%
│     └─ ⚠ 8 critical issues found (max allowed: 0)
└─ Issues Found
   ├─ src/auth.rs (Quality: 12%) [HIDDEN FILE]
   │     ├─ ● Authentication Bypass (line 45): return true
   │     ├─ ● Silent Error Suppression (line 67): catch {}
   │     └─ ● ... and 4 more issues
   └─ ... 4 more files with issues
```

#### Agent Deception Detection

The `--git-discovery` flag prevents AI agents from hiding problematic files during verification:

**How Agents Deceive:**

```
Agent reports: "Modified files: src/main.rs, src/utils.rs"
Git actually shows: src/main.rs, src/utils.rs, src/auth.rs, tests/broken.rs, config/secrets.rs
Hidden files often contain: quality issues, security vulnerabilities, TODO stubs
```

**Git Commands Used for Discovery:**

```bash
git diff --name-only              # Working directory changes
git diff --cached --name-only     # Staged changes
git diff HEAD~3 --name-only       # Recent commits
git ls-files --others --exclude-standard  # Untracked files
```

**Recommended Usage:**

- Use `--git-discovery` in CI/CD pipelines
- Use `--git-discovery` when agents complete complex tasks
- Use basic mode for simple, trusted changes

#### `sniff checkpoint` - Change Tracking

Create snapshots and track code quality changes over time.

```bash
# Create checkpoint before starting work
sniff checkpoint create --name "pre-refactor" --description "Before API cleanup" tests/samples/
```

```bash
# List all checkpoints
sniff checkpoint list
```

```bash
# Compare current state to checkpoint
sniff checkpoint diff pre-refactor
```

**Output:**

```
Changes since checkpoint 'pre-refactor'
──────────────────────────────────────────────────
├─ New files (3)
│  ├─ src/new_feature.rs
│  ├─ tests/new_test.rs
│  └─ docs/changelog.md
├─ Modified files (2)
│  ├─ src/main.rs
│  └─ tests/integration_test.rs
└─ No deletions
```

#### `sniff patterns` - Pattern Management

Simplified pattern management system using YAML playbooks.

```bash
# Initialize pattern system (shows guidance)
sniff patterns init
```

**Note**: Enhanced patterns are automatically installed and managed:

- **Auto-installation**: Enhanced patterns install to `~/.sniff/patterns/` on first run
- **Pattern loading**: Patterns are automatically loaded from the standard location
- **Available patterns**: Rust, Python, and TypeScript patterns are included
- **Custom patterns**: Add new YAML files to `~/.sniff/patterns/` directory
- **Consistent behavior**: Same results regardless of working directory

**Output:**

```
💡 Enhanced patterns are installed in ~/.sniff/patterns/
🔄 Add custom patterns by placing YAML files in that directory
🔍 Available patterns are loaded automatically during analysis
```

## Quick Start Guide

### 1. Basic Analysis

```bash
# Install Sniff
cargo install --git https://github.com/conikeec/sniff

# Navigate to your project
cd /path/to/your/codebase

# Run analysis (including test files)
sniff analyze-files . --extensions rs,py,ts,js --include-tests
```

### 2. Quality Gates

```bash
# Create TODO with quality requirements
sniff verify-todo --todo-id "feature-implementation" --files src/feature.rs --min-quality-score 80

# Implementation work...

# Verify before completion (basic)
sniff verify-todo --todo-id "feature-implementation" --files src/feature.rs --min-quality-score 80

# Secure verification (recommended for AI agents)
sniff verify-todo --todo-id "feature-implementation" --files src/feature.rs --git-discovery --min-quality-score 80
```

### 3. Change Tracking

```bash
# Create baseline checkpoint
sniff checkpoint create --name "baseline" .

# Work on code...

# Compare against baseline
sniff checkpoint diff baseline
```

## File Structure

Sniff maintains analysis data in a `.sniff` directory:

```
~/.sniff/
├── patterns/                # Enhanced pattern definitions (auto-installed)
│   ├── rust-patterns.yaml
│   ├── python-patterns.yaml
│   └── typescript-patterns.yaml
├── checkpoints/             # Checkpoint data
├── config/                  # Configuration files
└── cache/                   # Performance caches
```

## Pattern System

Patterns are defined in YAML files that specify detection rules:

```yaml
name: "Rust Quality Patterns"
language: "rust"
rules:
  - id: "rust_unimplemented"
    name: "Unimplemented Macro"
    description: "Function uses unimplemented!() macro"
    severity: "Critical"
    pattern_type: !Regex
      pattern: "unimplemented!\\(\\)"
    scope: "FunctionBody"
    enabled: true
```

## Integration Examples

### VS Code Integration

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
      "group": "test"
    }
  ]
}
```

### Git Pre-commit Hook

Add to `.git/hooks/pre-commit`:

```bash
#!/bin/bash
echo "Running Sniff code quality analysis..."

STAGED_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\.(rs|py|ts|js)$')

if [ -n "$STAGED_FILES" ]; then
    sniff analyze-files $STAGED_FILES --format compact --include-tests

    if sniff analyze-files $STAGED_FILES --format json --include-tests | grep -q '"critical_issues":[^0]'; then
        echo "Critical issues detected. Commit blocked."
        exit 1
    fi
fi

echo "Code quality checks passed"
```

## Configuration

Edit `.sniff/config.toml`:

```toml
[project]
name = "my-project"
languages = ["rust", "python", "typescript"]

[analysis]
quality_threshold = 80
max_critical_issues = 0

[patterns]
use_custom_patterns = true
learn_from_failures = true
```

## Command Line Options

### Common Options

- `--format`: Output format (table, json, markdown, compact)
- `--detailed`: Show detailed issue information
- `--extensions`: File extensions to analyze
- `--exclude`: Exclude files matching pattern
- `--max-file-size-mb`: Maximum file size to analyze

### Quality Options

- `--min-quality-score`: Minimum quality score required (0-100)
- `--max-critical-issues`: Maximum critical issues allowed
- `--include-tests`: Include test files in analysis (required for files detected as tests)
- `--test-confidence`: Confidence threshold for test file detection (0.0-1.0, default: 0.3)

### Security Options

- `--git-discovery`: Use Git to discover changed files (prevents agent deception)
  - Discovers files using: `git diff`, `git status`, `git ls-files`
  - Compares agent-reported vs git-discovered files
  - Warns when agents hide problematic files
  - Recommended for CI/CD and agent-completed tasks

## Community & Contributing

**Help Build Better Patterns**: Sniff's effectiveness grows with community contributions to the pattern detection playbooks. We encourage developers to submit PRs that:

- **Add new deceptive patterns** discovered in real-world AI coding sessions
- **Enhance existing patterns** with better regex or detection logic
- **Expand language support** with patterns for new programming languages
- **Improve pattern descriptions** to help developers understand detected issues
- **Share integration examples** for new editors, CI/CD systems, or workflows

The pattern playbooks in `playbooks/` are the core of Sniff's detection capabilities. Every contributed pattern helps the entire community catch AI deception more effectively. Whether you've found a new way AI agents try to fake completion or discovered edge cases in existing patterns, your contributions make Sniff more robust for everyone.

**Getting Started**: Check out existing patterns in `playbooks/rust-patterns.yaml`, `playbooks/python-patterns.yaml`, and `playbooks/typescript-patterns.yaml` to understand the format, then submit your improvements via pull request.

## License

MIT License - see [LICENSE](LICENSE) for details.

## Documentation

- [Integration Guide](integrations/README.md)
- [Pattern Development](docs/patterns.md)
- [Architecture Overview](docs/architecture.md)
