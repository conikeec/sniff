# Claude Code Integration for Sniff

This directory contains scripts and documentation for integrating Sniff with Claude Code subagents to create an iterative quality validation loop.

## Overview

The integration enables Claude Code subagents to:
1. **Detect changed files** in the current plan iteration
2. **Run Sniff analysis** to identify deceptive patterns  
3. **Interpret validation results** with specific guidance
4. **Iterate until quality gates pass** before marking plans complete

## How to Integrate with Claude Code

### Prerequisites

1. **Install Sniff**: Ensure `sniff` is available in your PATH
2. **Git Repository**: Your project must be a Git repository for change detection
3. **Claude Code Subagents**: Familiarity with creating and using Claude Code subagents

### Integration Steps

#### Step 1: Set Up Validation Scripts

Copy the provided scripts to your project:

```bash
# Copy validation scripts to your project
cp integrations/claude-code/sniff_validation.sh ./scripts/
cp integrations/claude-code/subagent_prompt.md ./.claude/agents/quality_validator.md
chmod +x ./scripts/sniff_validation.sh
```

#### Step 2: Create Quality Validation Subagent

Create a subagent specialized for quality validation:

```bash
# Create the subagent directory
mkdir -p .claude/agents

# The quality_validator.md file defines the subagent's behavior
# It will automatically use the validation scripts
```

#### Step 3: Integration in Development Workflow

Use the subagent in your Claude Code sessions:

```markdown
/agent quality_validator

Please validate the current plan implementation:
- Plan: "Implement user authentication system"
- Expected files: src/auth.rs, src/user.rs, tests/auth_tests.rs

Run quality validation and report if ready for completion.
```

### Validation Workflow

#### Automatic File Detection

The integration automatically detects files relevant to the current plan:

1. **Staged files**: `git diff --cached --name-only`
2. **Modified files**: `git diff --name-only`  
3. **New code files**: Untracked files matching code patterns

#### Quality Gates

**Completion Criteria**:
- ✅ **Zero critical issues** (no `unimplemented!()`, `panic!()`, etc.)
- ✅ **Quality score ≥ 85%** (configurable threshold)
- ✅ **All deceptive patterns addressed**

**Iteration Triggers**:
- ❌ Critical misalignment patterns found
- ❌ Quality score below threshold
- ❌ TODO comments in production code

#### Example Validation Output

```bash
🎯 Validating Plan: Implement user authentication system
🔍 Analyzing: 3 files

📊 Validation Results:
  Critical Issues: 2
  Quality Score: 67.8%
  Total Patterns: 8

❌ VALIDATION FAILED - Continue iteration

🔧 Issues to Fix:
  src/auth.rs:42 - Unimplemented Macro: Using unimplemented!() instead of proper implementation
  src/auth.rs:67 - TODO Comment: Contains TODO comment indicating incomplete work
  src/user.rs:23 - Panic with TODO: Using panic!() with TODO message
  tests/auth_tests.rs:15 - Unwrap Without Context: .unwrap() call without proper error context

🎯 Focus Areas:
  src/auth.rs: 4 issues, 45.0% quality
  src/user.rs: 3 issues, 71.2% quality  
  tests/auth_tests.rs: 1 issues, 89.1% quality
```

### Advanced Integration Patterns

#### 1. Checkpoint-Based Validation

```bash
# Validate against specific checkpoints
./scripts/sniff_validation.sh "User Registration" --checkpoint "pre-auth-feature"
```

#### 2. Progressive Quality Gates

```bash
# Different thresholds for different contexts
SNIFF_QUALITY_THRESHOLD=90.0 ./scripts/sniff_validation.sh "Production Release"
SNIFF_QUALITY_THRESHOLD=70.0 ./scripts/sniff_validation.sh "Development Feature"
```

#### 3. Pattern-Specific Validation

```bash
# Focus on specific pattern types
sniff analyze-files src/ --format json | jq '.files[].detections[] | select(.severity == "critical")'
```

### Configuration Options

#### Environment Variables

```bash
# Set in your shell or .claude/config
export SNIFF_QUALITY_THRESHOLD=85.0    # Minimum quality score (default: 85.0)
export SNIFF_ALLOW_TODOS=false         # Allow TODO comments (default: false)  
export SNIFF_MAX_CRITICAL=0            # Max critical issues (default: 0)
export SNIFF_VERBOSE=true              # Detailed output (default: false)
```

#### Custom Quality Thresholds

Different thresholds for different contexts:

- **Production**: 95% quality, 0 critical issues
- **Development**: 80% quality, 0 critical issues
- **Prototyping**: 60% quality, minimal critical issues

### Subagent Prompt Templates

#### Quality Validator Subagent

```markdown
# Role: Quality validation specialist for plan completion

When asked to validate a plan implementation:

1. **Detect Changes**: Use git to find files modified for this plan
2. **Run Analysis**: Execute sniff validation with appropriate thresholds  
3. **Interpret Results**: Provide specific, actionable feedback
4. **Make Decision**: PASS (mark complete) or FAIL (continue iteration)

Use the provided sniff_validation.sh script for consistent results.
```

#### Development Orchestrator Subagent

```markdown
# Role: Development workflow orchestrator

For each plan item:
1. Implement the required functionality
2. **ALWAYS** validate with quality_validator subagent before completion
3. If validation fails, iterate based on specific feedback
4. Only mark plan complete when validation passes

Never skip quality validation - it ensures alignment with requirements.
```

### Troubleshooting

#### Common Issues

**"No files to validate"**
- Ensure files are staged or modified in git
- Check that file extensions match Sniff's supported languages

**"Sniff command not found"**  
- Install Sniff: `cargo install sniff` or `brew install sniff`
- Ensure Sniff is in your PATH

**"Quality threshold too strict"**
- Adjust `SNIFF_QUALITY_THRESHOLD` environment variable
- Review specific patterns flagged - some may be acceptable in context

#### Debug Mode

Enable detailed logging:

```bash
RUST_LOG=debug SNIFF_VERBOSE=true ./scripts/sniff_validation.sh "Debug Plan"
```

### Best Practices

1. **Always validate before completion**: Never mark plans complete without quality validation
2. **Address critical issues first**: Focus on `unimplemented!()`, `panic!()`, and obvious placeholders
3. **Use appropriate thresholds**: Different contexts need different quality standards
4. **Iterate based on specific feedback**: Don't just re-implement, fix the specific patterns identified
5. **Track quality trends**: Monitor quality scores across iterations to ensure improvement

### Integration Benefits

- **Prevents deceptive completions**: No more "TODO: implement this" marked as done
- **Maintains code quality**: Consistent standards across all implementations  
- **Specific feedback**: Know exactly what to fix, not just "improve quality"
- **Objective validation**: Remove subjective judgment from completion decisions
- **Iterative improvement**: Each cycle addresses specific alignment issues

---

For more information, see the individual script documentation and the main Sniff README.