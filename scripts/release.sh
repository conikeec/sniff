#!/bin/bash
set -euo pipefail

# Sniff Release Script
echo "🚀 Preparing Sniff Release"
echo "=========================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

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

# Check if version is provided
if [ $# -eq 0 ]; then
    print_error "Usage: $0 <version>"
    echo "Example: $0 v1.0.0"
    exit 1
fi

VERSION="$1"

# Validate version format
if [[ ! "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9]+)?$ ]]; then
    print_error "Invalid version format. Use semver format like v1.0.0 or v1.0.0-beta"
    exit 1
fi

# Check if we're on main branch
CURRENT_BRANCH=$(git branch --show-current)
if [ "$CURRENT_BRANCH" != "main" ] && [ "$CURRENT_BRANCH" != "master" ]; then
    print_warning "Not on main/master branch. Current branch: $CURRENT_BRANCH"
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Check if working directory is clean
if [ -n "$(git status --porcelain)" ]; then
    print_error "Working directory is not clean. Commit or stash changes first."
    git status --short
    exit 1
fi

# Run quality checks
print_status "Running quality checks..."
if ! ./scripts/check.sh; then
    print_error "Quality checks failed. Fix issues before release."
    exit 1
fi

# Update version in Cargo.toml
print_status "Updating version in Cargo.toml..."
VERSION_NUMBER=${VERSION#v}  # Remove 'v' prefix
sed -i.bak "s/^version = \".*\"/version = \"$VERSION_NUMBER\"/" Cargo.toml
rm Cargo.toml.bak
print_success "Updated version to $VERSION_NUMBER"

# Update Cargo.lock
print_status "Updating Cargo.lock..."
cargo update --workspace
print_success "Updated Cargo.lock"

# Build release binaries locally for testing
print_status "Building release binaries..."
cargo build --release
print_success "Release build successful"

# Run tests one more time with release build
print_status "Running tests with release build..."
cargo test --release
print_success "Release tests passed"

# Create release commit
print_status "Creating release commit..."
git add Cargo.toml Cargo.lock
git commit -m "chore: release $VERSION

- Update version to $VERSION_NUMBER
- Update dependencies
- Ready for public release"
print_success "Release commit created"

# Create and push tag
print_status "Creating release tag..."
git tag -a "$VERSION" -m "Release $VERSION

## Features
- AI misalignment pattern detection for Rust, Python, TypeScript
- Claude Code session analysis and real-time monitoring  
- Standalone file analysis for any editor
- Pattern learning and community contribution system
- Comprehensive playbook system for custom rules

## Installation
\`\`\`bash
# From GitHub releases
curl -L https://github.com/your-username/sniff/releases/download/$VERSION/sniff-\$(uname -s)-\$(uname -m).tar.gz | tar xz
sudo mv sniff /usr/local/bin/

# From Homebrew (coming soon)
brew install your-username/tap/sniff
\`\`\`

See CHANGELOG.md for detailed changes."

print_success "Tag $VERSION created"

# Show summary
echo ""
print_success "Release $VERSION prepared successfully! 🎉"
echo ""
echo "Next steps:"
echo "  1. Review the changes: git show $VERSION"
echo "  2. Push the release: git push origin main && git push origin $VERSION"
echo "  3. GitHub Actions will automatically:"
echo "     • Build binaries for all platforms"
echo "     • Create GitHub release with artifacts"
echo "     • Update Homebrew formula"
echo ""
echo "Or run: git push origin main && git push origin $VERSION"