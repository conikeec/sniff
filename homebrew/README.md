# Homebrew Formula for Sniff

This directory contains the Homebrew formula for the Sniff AI misalignment detection tool.

## Setup Instructions

### 1. Create Homebrew Tap Repository

Create a new GitHub repository named `homebrew-tap` under your account:

```bash
# Clone your new tap repository
git clone https://github.com/conikeec/homebrew-tap.git
cd homebrew-tap

# Create Formula directory
mkdir -p Formula

# Copy the formula
cp ../sniff/homebrew/sniff.rb Formula/

# Commit and push
git add .
git commit -m "Add sniff formula"
git push origin main
```

### 2. Configure GitHub Secrets

In your main `sniff` repository, add these secrets in Settings > Secrets and variables > Actions:

1. **HOMEBREW_TAP_TOKEN**: Personal Access Token with repo permissions for the homebrew-tap repository
2. **CARGO_REGISTRY_TOKEN**: Token for publishing to crates.io (optional)

### 3. Set up Tap Repository Automation

In your `homebrew-tap` repository, create `.github/workflows/update-formula.yml`:

```yaml
name: Update Formula

on:
  repository_dispatch:
    types: [update-formula]

jobs:
  update:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4
        with:
          token: ${{ secrets.GITHUB_TOKEN }}

      - name: Update Formula
        run: |
          FORMULA="${{ github.event.client_payload.formula }}"
          VERSION="${{ github.event.client_payload.version }}"
          URL="${{ github.event.client_payload.url }}"
          SHA256="${{ github.event.client_payload.sha256 }}"
          DESCRIPTION="${{ github.event.client_payload.description }}"
          
          # Update the formula file
          sed -i "s|url \".*\"|url \"$URL\"|" "Formula/${FORMULA}.rb"
          sed -i "s|sha256 \".*\"|sha256 \"$SHA256\"|" "Formula/${FORMULA}.rb"
          sed -i "s|version \".*\"|version \"${VERSION#v}\"|" "Formula/${FORMULA}.rb"
          sed -i "s|desc \".*\"|desc \"$DESCRIPTION\"|" "Formula/${FORMULA}.rb"

      - name: Commit changes
        run: |
          git config --local user.email "action@github.com"
          git config --local user.name "GitHub Action"
          git add .
          git commit -m "Update ${{ github.event.client_payload.formula }} to ${{ github.event.client_payload.version }}" || exit 0
          git push
```

### 4. Test Installation

After setting up the tap:

```bash
# Add your tap
brew tap conikeec/tap

# Install sniff
brew install sniff

# Test installation
sniff --version
```

### 5. Publishing Releases

When you push a new version tag to the main repository:

1. GitHub Actions will build binaries for all platforms
2. Create a GitHub release with the binaries
3. Automatically update the Homebrew formula
4. Optionally publish to crates.io

Example release workflow:

```bash
# Make sure you're on main branch with clean working directory
git checkout main
git pull origin main

# Create and push a new release
./scripts/release.sh v1.0.0
git push origin main
git push origin v1.0.0
```

## Formula Details

The formula:
- Installs the `sniff` binary to `/usr/local/bin/`
- Includes shell completions if available
- Includes man page if available
- Tests basic functionality during installation
- Supports both stable and development versions

## Maintenance

The formula is automatically updated when new releases are published through GitHub Actions. Manual updates can be made by:

1. Updating the URL, SHA256, and version in `Formula/sniff.rb`
2. Testing locally: `brew install --build-from-source Formula/sniff.rb`
3. Committing and pushing changes

For issues with the Homebrew formula, please open an issue in the main `sniff` repository.