# Sniff Test Files

This directory contains organized test files for the Sniff AI misalignment detector.

## Directory Structure

### `/samples/` - Test Code Samples
Test files containing various AI misalignment patterns for testing detection capabilities:

- **`test_misalignment.rs`** - Primary Rust test file with comprehensive misalignment patterns
- **`test_misalignment.py`** - Primary Python test file with comprehensive misalignment patterns  
- **`test_enhanced_patterns.rs`** - Enhanced Rust patterns testing new detection rules
- **`test_exact_patterns.py`** - Python patterns with exact syntax for pattern matching
- **`test_typescript_patterns.ts`** - TypeScript/JavaScript misalignment patterns
- **`test_new_patterns.rs`** - Additional Rust patterns for development testing
- **`test_python.py`** - Additional Python test samples

### `/fixtures/` - Test Fixtures
Fixed test data used for consistent testing scenarios:

- **`final_test.rs`** - Simple test fixture for basic functionality
- **`test_file.rs`** - Generic test file fixture
- **`clean_test.rs`** - Clean code sample without misalignment patterns
- **`simple_test.rs`** - Minimal test case
- **`comprehensive_test.rs`** - Comprehensive test coverage fixture
- **`debug_test.rs`** - Debug-specific test scenarios

### `/integration/` - Integration Tests
(Reserved for future Rust integration tests using `#[cfg(test)]`)

## Usage

### Running Tests on Samples

```bash
# Test individual samples
cargo run --bin sniff -- analyze-files tests/samples/test_misalignment.rs
cargo run --bin sniff -- analyze-files tests/samples/test_misalignment.py
cargo run --bin sniff -- analyze-files tests/samples/test_typescript_patterns.ts

# Test all samples at once
cargo run --bin sniff -- analyze-files tests/samples/

# Run the integrated test binary
cargo run --bin test_analyzer
```

### Pattern Development Workflow

1. **Create test cases** in `/samples/` with specific misalignment patterns
2. **Verify detection** using `sniff analyze-files`
3. **Iterate on patterns** in `playbooks/` directory
4. **Test against fixtures** in `/fixtures/` for regression testing

### Test File Conventions

- **Rust files**: `.rs` extension, use `// TODO:`, `unimplemented!()`, etc.
- **Python files**: `.py` extension, use `# TODO:`, `raise NotImplementedError`, etc.
- **TypeScript files**: `.ts` extension, use `// TODO:`, `throw new Error()`, etc.

Each test file should contain:
- Multiple pattern types (critical, high, medium severity)
- Both positive cases (should detect) and edge cases
- Comments explaining the expected misalignment patterns
- Realistic code scenarios where AI might generate these patterns

## Contributing Test Cases

When creating new test files:

1. **Name descriptively**: `test_[language]_[feature].ext`
2. **Document patterns**: Add comments explaining what should be detected
3. **Test thoroughly**: Verify patterns are detected with expected severity
4. **Organize appropriately**: 
   - Active development → `/samples/`
   - Stable regression tests → `/fixtures/`
   - Integration scenarios → `/integration/`

## Pattern Categories Tested

- **Premature returns**: `Ok(())`, `return None`, `return true` without implementation
- **Placeholder implementations**: `unimplemented!()`, `pass`, `throw new Error()`
- **Error suppression**: Empty catch blocks, `except: pass`, `Err(_) => {}`
- **Mock data**: Hardcoded test values, placeholder arrays/objects
- **Security bypasses**: Always-true authentication, skipped validation
- **Timing simulation**: `sleep()`, `setTimeout()` as fake work
- **Generic errors**: Non-descriptive error messages
- **TODO/FIXME comments**: Placeholder comments indicating incomplete work