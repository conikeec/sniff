# Quick Start: Claude Code + Sniff Integration  

Get up and running with Sniff quality validation in Claude Code in under 5 minutes.

## Prerequisites

✅ **Sniff installed**: `cargo install sniff` or `brew install sniff`  
✅ **Claude Code** with subagent support  
✅ **Git repository** for your project  
✅ **jq installed**: `brew install jq` or `apt-get install jq`

## Step 1: Copy Integration Files (30 seconds)

```bash
# From your project root
mkdir -p scripts .claude/agents

# Copy the validation script
cp /path/to/sniff/integrations/claude-code/sniff_validation.sh ./scripts/
chmod +x ./scripts/sniff_validation.sh

# Copy the subagent prompt
cp /path/to/sniff/integrations/claude-code/subagent_prompt.md ./.claude/agents/quality_validator.md
```

## Step 2: Test Basic Functionality (1 minute)

```bash
# Test sniff is working
sniff --version

# Test the validation script
echo 'fn main() { unimplemented!(); }' > test.rs
./scripts/sniff_validation.sh "Test Plan"
rm test.rs

# Should show validation failure with critical issues
```

## Step 3: Create a Test Implementation (2 minutes)

```bash
# Create a simple implementation with deceptive patterns
mkdir -p src
cat > src/example.rs << 'EOF'
pub fn authenticate_user(username: &str, password: &str) -> bool {
    // TODO: implement proper authentication
    unimplemented!()
}

pub fn hash_password(password: &str) -> String {
    // Quick implementation for now
    format!("hashed_{}", password)
}

pub fn validate_email(email: &str) -> bool {
    email.contains('@') // Basic validation for now
}
EOF

# Stage the file
git add src/example.rs
```

## Step 4: Test Validation in Claude Code (1 minute)

Open Claude Code and use the quality validation subagent:

```
/agent quality_validator

Please validate the authentication implementation in src/example.rs.
Plan: "Implement secure user authentication system"
```

**Expected Result**: Validation should FAIL with critical issues found.

## Step 5: Fix Issues and Re-validate (1 minute)

Update the implementation to remove deceptive patterns:

```rust
// src/example.rs - Fixed version
use bcrypt::{hash, verify, DEFAULT_COST};

pub fn authenticate_user(username: &str, password: &str) -> Result<bool, AuthError> {
    let stored_hash = get_user_hash(username)?;
    verify(password, &stored_hash).map_err(|_| AuthError::VerifyFailed)
}

pub fn hash_password(password: &str) -> Result<String, AuthError> {
    validate_password_strength(password)?;
    hash(password, DEFAULT_COST).map_err(|_| AuthError::HashFailed)
}

pub fn validate_email(email: &str) -> bool {
    let email_regex = regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
        .expect("Invalid email regex");
    email_regex.is_match(email)
}

#[derive(Debug)]
pub enum AuthError {
    HashFailed,
    VerifyFailed,
    WeakPassword,
}

fn validate_password_strength(password: &str) -> Result<(), AuthError> {
    if password.len() < 8 {
        return Err(AuthError::WeakPassword);
    }
    Ok(())
}

fn get_user_hash(username: &str) -> Result<String, AuthError> {
    // Implementation would retrieve from database
    Ok("$2b$12$example_hash".to_string())
}
```

Re-run validation:
```
/agent quality_validator

Please re-validate the improved authentication implementation.
```

**Expected Result**: Validation should PASS with high quality score.

## Quick Configuration

Set quality thresholds via environment variables:

```bash
# Strict validation
export SNIFF_QUALITY_THRESHOLD=90.0

# Standard validation (default)
export SNIFF_QUALITY_THRESHOLD=85.0

# Lenient for prototypes
export SNIFF_QUALITY_THRESHOLD=70.0
```

## Common Usage Patterns

### 1. Plan Item Validation
```
/agent quality_validator

Validate plan: "Add user registration endpoint"
Expected files: src/routes/auth.rs, src/models/user.rs
```

### 2. Before Git Commit
```bash
# Validate all staged changes
./scripts/sniff_validation.sh "Pre-commit validation"
```

### 3. Feature Branch Validation  
```bash
# Validate all changes in feature branch
git diff main --name-only | xargs sniff analyze-files --format json
```

## Troubleshooting

**"sniff command not found"**
```bash
# Install sniff
cargo install sniff
# OR
brew install sniff
```

**"No files to validate"**
```bash
# Make sure files are staged or modified
git add your_files.rs
git status  # Should show staged changes
```

**"Quality threshold too strict"**
```bash
# Lower the threshold temporarily
SNIFF_QUALITY_THRESHOLD=70.0 ./scripts/sniff_validation.sh "Your Plan"
```

## Next Steps

1. **Read the full README**: `integrations/claude-code/README.md`
2. **See complete example**: `integrations/claude-code/example_workflow.md`  
3. **Customize patterns**: Add project-specific patterns to `playbooks/`
4. **Integrate with CI**: Add validation to your GitHub Actions

You're now ready to use Sniff with Claude Code for quality-validated iterative development!