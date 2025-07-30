# Example Claude Code + Sniff Integration Workflow

This document shows a complete example of how to integrate Sniff with Claude Code subagents for iterative quality validation.

## Setup

### Project Structure
```
my-project/
├── .claude/
│   └── agents/
│       └── quality_validator.md    # Quality validation subagent
├── scripts/
│   └── sniff_validation.sh         # Validation script
├── src/
│   ├── auth.rs                     # Implementation files
│   └── user.rs
└── tests/
    └── auth_tests.rs
```

### Environment Configuration
```bash
# Set quality standards
export SNIFF_QUALITY_THRESHOLD=85.0
export SNIFF_MAX_CRITICAL=0
export SNIFF_VERBOSE=true
```

## Example Session

### 1. Initial Plan Implementation

**Developer Request**:
```
Please implement a user authentication system with the following requirements:
- Secure password hashing using bcrypt
- Session management with JWT tokens  
- Input validation for email and password
- Proper error handling for authentication failures

Files to create/modify:
- src/auth.rs - Main authentication logic
- src/user.rs - User model and validation
- tests/auth_tests.rs - Comprehensive tests
```

**Claude Code Response**:
```rust
// src/auth.rs - Initial implementation
use bcrypt::{hash, verify, DEFAULT_COST};

pub struct AuthService;

impl AuthService {
    pub fn hash_password(password: &str) -> Result<String, AuthError> {
        // TODO: Add password complexity validation
        hash(password, DEFAULT_COST).map_err(|_| AuthError::HashError)
    }
    
    pub fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
        verify(password, hash).map_err(|_| AuthError::VerifyError)
    }
    
    pub fn create_jwt_token(user_id: u64) -> Result<String, AuthError> {
        unimplemented!() // TODO: Implement JWT creation
    }
    
    pub fn validate_email(email: &str) -> bool {
        email.contains('@') // Basic validation for now
    }
}

#[derive(Debug)]
pub enum AuthError {
    HashError,
    VerifyError,
    TokenError,
}
```

### 2. Quality Validation (First Iteration)

**Developer invokes validation subagent**:
```
/agent quality_validator

Please validate the user authentication implementation:
- Plan: "Implement secure user authentication with password hashing and session management"
- Expected files: src/auth.rs, src/user.rs, tests/auth_tests.rs
```

**Quality Validator Subagent Response**:
```bash
# Subagent runs validation
./scripts/sniff_validation.sh "User Authentication System"
```

**Validation Output**:
```
🎯 Sniff Quality Validation
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

🔍 Analyzing 3 files for plan: User Authentication System

📋 Validation Results for: User Authentication System
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 Summary:
   Files Analyzed: 3
   Total Patterns: 7
   Critical Issues: 2
   Quality Score: 64.3%

❌ VALIDATION FAILED
   Quality: 64.3% (≥85.0% required) ❌
   Critical: 2 (≤0 allowed) ❌

🔧 Issues to Address:

🚨 CRITICAL (Must Fix - 2 issues):
   src/auth.rs:13 - Unimplemented Macro: Replace unimplemented!() with actual JWT logic
   src/auth.rs:17 - AI Shortcut Comments: Basic validation comment indicates incomplete implementation

🟡 HIGH PRIORITY (3 issues):
   src/auth.rs:7 - TODO Comment: Add password complexity validation
   src/user.rs:15 - Unwrap Without Context: .unwrap() call without proper error handling  
   tests/auth_tests.rs:8 - TODO Comment: Add comprehensive authentication tests

📄 File Quality Breakdown:
   src/auth.rs: 4 issues, 45.2% quality
   src/user.rs: 2 issues, 72.1% quality
   tests/auth_tests.rs: 1 issues, 89.0% quality

💡 Next Steps:
   1. Address CRITICAL issues first (blocking completion)
   2. Improve HIGH PRIORITY issues (affects quality score)
   3. Re-run validation: ./scripts/sniff_validation.sh "User Authentication System"
   4. Continue iteration until validation passes
```

**Subagent Decision**:
```
❌ VALIDATION FAILED

Critical issues found that must be addressed before completion:

🚨 CRITICAL ISSUES:
- src/auth.rs:13 - Unimplemented Macro: The JWT token creation is not implemented (unimplemented!() placeholder)
- src/auth.rs:17 - AI Shortcut Comments: Email validation is too simplistic ("Basic validation for now")

🟡 HIGH PRIORITY ISSUES:
- src/auth.rs:7 - TODO Comment: Password complexity validation is missing
- src/user.rs:15 - Unwrap Without Context: Error handling needs improvement
- tests/auth_tests.rs:8 - TODO Comment: Tests are incomplete

The implementation contains deceptive patterns that make it appear complete when it's actually missing critical functionality. Please address these issues and re-run validation.

Continue iteration - plan not ready for completion.
```

### 3. Implementation Refinement (Second Iteration)

**Developer addresses issues**:
```rust
// src/auth.rs - Improved implementation
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{encode, Header, EncodingKey};
use regex::Regex;

pub struct AuthService;

impl AuthService {
    pub fn hash_password(password: &str) -> Result<String, AuthError> {
        // Validate password complexity
        Self::validate_password_complexity(password)?;
        hash(password, DEFAULT_COST).map_err(|_| AuthError::HashError)
    }
    
    pub fn verify_password(password: &str, hash: &str) -> Result<bool, AuthError> {
        verify(password, hash).map_err(|_| AuthError::VerifyError)
    }
    
    pub fn create_jwt_token(user_id: u64) -> Result<String, AuthError> {
        let claims = Claims { user_id, exp: get_expiration_time() };
        let secret = std::env::var("JWT_SECRET")
            .map_err(|_| AuthError::ConfigError("JWT_SECRET not set"))?;
        
        encode(&Header::default(), &claims, &EncodingKey::from_secret(secret.as_ref()))
            .map_err(|_| AuthError::TokenError)
    }
    
    pub fn validate_email(email: &str) -> bool {
        let email_regex = Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
            .expect("Invalid email regex");
        email_regex.is_match(email)
    }
    
    fn validate_password_complexity(password: &str) -> Result<(), AuthError> {
        if password.len() < 8 {
            return Err(AuthError::WeakPassword("Password must be at least 8 characters"));
        }
        if !password.chars().any(|c| c.is_uppercase()) {
            return Err(AuthError::WeakPassword("Password must contain uppercase letter"));
        }
        if !password.chars().any(|c| c.is_numeric()) {
            return Err(AuthError::WeakPassword("Password must contain number"));
        }
        Ok(())
    }
}

// Proper error handling with specific messages
#[derive(Debug)]
pub enum AuthError {
    HashError,
    VerifyError, 
    TokenError,
    WeakPassword(&'static str),
    ConfigError(&'static str),
}
```

### 4. Quality Validation (Second Iteration)

**Developer re-invokes validation**:
```
/agent quality_validator

Please re-validate the improved authentication implementation.
```

**Validation Output**:
```
🎯 Sniff Quality Validation
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

🔍 Analyzing 3 files for plan: User Authentication System

📋 Validation Results for: User Authentication System
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 Summary:
   Files Analyzed: 3
   Total Patterns: 1
   Critical Issues: 0
   Quality Score: 91.7%

✅ VALIDATION PASSED
   Quality: 91.7% (≥85.0% required) ✅
   Critical: 0 (≤0 allowed) ✅

🎉 Plan 'User Authentication System' is ready for completion!
```

**Subagent Decision**:
```
✅ VALIDATION PASSED

Quality Score: 91.7%
Critical Issues: 0
Files Analyzed: 3

The authentication implementation has been successfully validated:

✅ All critical deceptive patterns have been resolved
✅ JWT token creation is properly implemented
✅ Password complexity validation is complete
✅ Email validation uses proper regex patterns
✅ Error handling is comprehensive and specific
✅ Quality threshold met (91.7% > 85.0%)

Plan "Implement secure user authentication with password hashing and session management" is ready for completion.

The implementation now truly aligns with the stated requirements and contains no deceptive shortcuts or placeholders.
```

## Key Benefits Demonstrated

### 1. **Objective Quality Gates**
- Clear pass/fail criteria based on actual code analysis
- No subjective judgment about "completeness"
- Specific thresholds that can be adjusted per context

### 2. **Specific, Actionable Feedback**
- Exact line numbers and pattern types
- Clear descriptions of what needs to be fixed
- Prioritized by severity (critical first)

### 3. **Iterative Improvement**
- Each cycle addresses specific identified issues
- Progress tracked through quality scores
- No plan marked complete until truly aligned

### 4. **Prevention of Deceptive Completions**
- Catches `unimplemented!()`, `todo!()`, obvious placeholders
- Identifies shortcuts disguised as implementations
- Ensures actual functionality matches plan descriptions

## Configuration for Different Contexts

### Production Release (Strict)
```bash
export SNIFF_QUALITY_THRESHOLD=95.0
export SNIFF_MAX_CRITICAL=0
export SNIFF_ALLOW_TODOS=false
```

### Development Feature (Standard)
```bash
export SNIFF_QUALITY_THRESHOLD=85.0
export SNIFF_MAX_CRITICAL=0
export SNIFF_ALLOW_TODOS=false
```

### Prototype/Exploration (Lenient)
```bash
export SNIFF_QUALITY_THRESHOLD=70.0
export SNIFF_MAX_CRITICAL=1
export SNIFF_ALLOW_TODOS=true
```

This integration ensures that Claude Code subagents maintain high implementation standards while providing clear, actionable feedback for continuous improvement.