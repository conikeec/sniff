#!/bin/bash
set -euo pipefail

# Sniff Code Formatting Script
echo "🎨 Formatting Sniff codebase..."
echo "==============================="

# Colors for output
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

print_status() {
    echo -e "${BLUE}📋 $1${NC}"
}

print_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

# 1. Format Rust code
print_status "Formatting Rust code..."
cargo fmt --all
print_success "Rust code formatted"

# 2. Format YAML files (if prettier is available)
if command -v prettier >/dev/null 2>&1; then
    print_status "Formatting YAML files..."
    prettier --write "playbooks/*.yaml" "*.yml" ".github/**/*.yml" || true
    print_success "YAML files formatted"
fi

# 3. Format JSON files (if prettier is available) 
if command -v prettier >/dev/null 2>&1; then
    print_status "Formatting JSON files..."
    prettier --write "*.json" ".vscode/*.json" || true
    print_success "JSON files formatted"
fi

# 4. Format Markdown files (if prettier is available)
if command -v prettier >/dev/null 2>&1; then
    print_status "Formatting Markdown files..."
    prettier --write "*.md" "docs/*.md" "tests/*.md" || true
    print_success "Markdown files formatted"
fi

echo ""
print_success "All formatting complete! 🎉"
echo ""
echo "Files have been formatted according to:"
echo "  • rustfmt.toml configuration"
echo "  • Prettier defaults (if available)"
echo ""
echo "Run './scripts/check.sh' to verify formatting."