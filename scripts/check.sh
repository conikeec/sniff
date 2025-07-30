#!/bin/bash
set -euo pipefail

# Sniff Code Quality Check Script
echo "🕵️  Running Sniff Code Quality Checks"
echo "====================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}📋 $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

print_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Track overall success
OVERALL_SUCCESS=true

# 1. Format check
print_status "Checking code formatting..."
if cargo fmt --all -- --check; then
    print_success "Code formatting is correct"
else
    print_error "Code formatting issues found. Run 'cargo fmt' to fix."
    OVERALL_SUCCESS=false
fi

echo ""

# 2. Clippy check
print_status "Running Clippy lints..."
if cargo clippy --all-targets --all-features -- -D warnings; then
    print_success "Clippy checks passed"
else
    print_error "Clippy found issues. Fix them before proceeding."
    OVERALL_SUCCESS=false
fi

echo ""

# 3. Build check
print_status "Building project..."
if cargo build --all-targets; then
    print_success "Build successful"
else
    print_error "Build failed"
    OVERALL_SUCCESS=false
fi

echo ""

# 4. Test check
print_status "Running tests..."
if cargo test; then
    print_success "All tests passed"
else
    print_error "Some tests failed"
    OVERALL_SUCCESS=false
fi

echo ""

# 5. Documentation check
print_status "Checking documentation..."
if cargo doc --no-deps --document-private-items; then
    print_success "Documentation builds successfully"
else
    print_warning "Documentation issues found"
    # Don't fail overall for docs issues
fi

echo ""

# 6. Security audit (if cargo-audit is installed)
if command -v cargo-audit >/dev/null 2>&1; then
    print_status "Running security audit..."
    if cargo audit; then
        print_success "Security audit passed"
    else
        print_warning "Security audit found issues"
        # Don't fail overall for audit issues in CI
    fi
    echo ""
fi

# 7. License check
print_status "Checking license headers..."
missing_license=0
for file in $(find src -name "*.rs"); do
    if ! head -3 "$file" | grep -q "Licensed under the MIT License"; then
        print_warning "Missing license header in $file"
        missing_license=$((missing_license + 1))
    fi
done

if [ $missing_license -eq 0 ]; then
    print_success "All source files have license headers"
else
    print_warning "$missing_license files missing license headers"
fi

echo ""

# Final result
if [ "$OVERALL_SUCCESS" = true ]; then
    print_success "All critical checks passed! 🎉"
    echo ""
    echo "Project is ready for:"
    echo "  • Pull requests"
    echo "  • Releases"
    echo "  • Public distribution"
    exit 0
else
    print_error "Some critical checks failed!"
    echo ""
    echo "Please fix the issues above before:"
    echo "  • Creating pull requests"
    echo "  • Publishing releases"
    echo "  • Making the repository public"
    exit 1
fi