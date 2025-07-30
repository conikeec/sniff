#!/bin/bash

# Sniff Quality Validation Script for Claude Code Integration
# Licensed under the MIT License
# Copyright (c) 2024 Sniff Contributors

set -euo pipefail

# Configuration with defaults
PLAN_ITEM="${1:-Current Plan Item}"
QUALITY_THRESHOLD="${SNIFF_QUALITY_THRESHOLD:-85.0}"
MAX_CRITICAL="${SNIFF_MAX_CRITICAL:-0}"
ALLOW_TODOS="${SNIFF_ALLOW_TODOS:-false}"
VERBOSE="${SNIFF_VERBOSE:-false}"
OUTPUT_FILE="${SNIFF_OUTPUT_FILE:-validation.json}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

print_status() {
    echo -e "${BLUE}🔍 $1${NC}"
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

print_info() {
    echo -e "${PURPLE}📋 $1${NC}"
}

verbose_log() {
    if [ "$VERBOSE" = "true" ]; then
        echo -e "${BLUE}[DEBUG] $1${NC}"
    fi
}

# Validate prerequisites
check_prerequisites() {
    verbose_log "Checking prerequisites..."
    
    if ! command -v sniff >/dev/null 2>&1; then
        print_error "sniff command not found. Install with: cargo install sniff"
        exit 1
    fi
    
    if ! command -v git >/dev/null 2>&1; then
        print_error "git command not found. This script requires git for change detection."
        exit 1
    fi
    
    if ! command -v jq >/dev/null 2>&1; then
        print_error "jq command not found. Install with: apt-get install jq or brew install jq"
        exit 1
    fi
    
    if ! git rev-parse --git-dir >/dev/null 2>&1; then
        print_error "Not in a git repository. Change detection requires git."
        exit 1
    fi
    
    verbose_log "Prerequisites validated"
}

# Get files changed for current plan
get_changed_files() {
    verbose_log "Detecting changed files..."
    
    # Get staged files
    local staged_files
    staged_files=$(git diff --cached --name-only 2>/dev/null || true)
    
    # Get modified but unstaged files
    local modified_files
    modified_files=$(git diff --name-only 2>/dev/null || true)
    
    # Get untracked files that match code patterns
    local untracked_files
    untracked_files=$(git ls-files --others --exclude-standard 2>/dev/null | \
        grep -E '\.(rs|py|ts|tsx|js|jsx|go|cpp|c|java|kt|swift|rb|php|cs|scala|clj|hs|ml|dart|lua|r|jl)$' || true)
    
    # Combine and deduplicate, filter for code files only
    local all_files
    all_files=$(echo -e "$staged_files\n$modified_files\n$untracked_files" | \
        sort -u | \
        grep -v '^$' | \
        grep -E '\.(rs|py|ts|tsx|js|jsx|go|cpp|c|java|kt|swift|rb|php|cs|scala|clj|hs|ml|dart|lua|r|jl)$' || true)
    
    # Filter out files that don't exist
    local existing_files=""
    for file in $all_files; do
        if [ -f "$file" ]; then
            existing_files="$existing_files $file"
        fi
    done
    
    echo "$existing_files" | xargs -n1 | sort -u
}

# Run sniff analysis
run_sniff_analysis() {
    local files="$1"
    
    if [ -z "$files" ]; then
        print_warning "No code files found to validate"
        echo '{"summary": {"critical_issues": 0, "total_patterns": 0, "average_quality": 100.0}, "files": []}' > "$OUTPUT_FILE"
        return 0
    fi
    
    local file_count
    file_count=$(echo "$files" | wc -w)
    
    print_status "Analyzing $file_count files for plan: $PLAN_ITEM"
    verbose_log "Files: $(echo $files | tr ' ' '\n')"
    
    # Run sniff with detailed output
    if ! sniff analyze-files $files --format json --detailed > "$OUTPUT_FILE" 2>/dev/null; then
        print_error "Sniff analysis failed"
        return 1
    fi
    
    verbose_log "Analysis complete, results saved to $OUTPUT_FILE"
}

# Parse and interpret results
interpret_results() {
    if [ ! -f "$OUTPUT_FILE" ]; then
        print_error "Validation output file not found: $OUTPUT_FILE"
        return 1
    fi
    
    # Extract key metrics
    local critical_issues
    critical_issues=$(jq -r '.summary.critical_issues // 0' "$OUTPUT_FILE")
    
    local total_patterns
    total_patterns=$(jq -r '.summary.total_patterns // 0' "$OUTPUT_FILE")
    
    local average_quality
    average_quality=$(jq -r '.summary.average_quality // 100.0' "$OUTPUT_FILE")
    
    local files_analyzed
    files_analyzed=$(jq -r '.summary.files_analyzed // 0' "$OUTPUT_FILE")
    
    # Display summary
    echo ""
    print_info "Validation Results for: $PLAN_ITEM"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "📊 Summary:"
    echo "   Files Analyzed: $files_analyzed"
    echo "   Total Patterns: $total_patterns"
    echo "   Critical Issues: $critical_issues"
    echo "   Quality Score: $average_quality%"
    echo ""
    
    # Check quality gates
    local quality_passed=false
    local critical_passed=false
    
    if [ "$(echo "$average_quality >= $QUALITY_THRESHOLD" | bc -l 2>/dev/null || echo 0)" -eq 1 ]; then
        quality_passed=true
    fi
    
    if [ "$critical_issues" -le "$MAX_CRITICAL" ]; then
        critical_passed=true
    fi
    
    # Decision logic
    if [ "$quality_passed" = true ] && [ "$critical_passed" = true ]; then
        print_success "VALIDATION PASSED"
        echo "   Quality: $average_quality% (≥$QUALITY_THRESHOLD% required)"
        echo "   Critical: $critical_issues (≤$MAX_CRITICAL allowed)"
        echo ""
        echo "🎉 Plan '$PLAN_ITEM' is ready for completion!"
        return 0
    else
        print_error "VALIDATION FAILED"
        echo "   Quality: $average_quality% (≥$QUALITY_THRESHOLD% required) $([ "$quality_passed" = true ] && echo "✅" || echo "❌")"
        echo "   Critical: $critical_issues (≤$MAX_CRITICAL allowed) $([ "$critical_passed" = true ] && echo "✅" || echo "❌")"
        echo ""
        show_specific_issues
        return 1
    fi
}

# Show specific issues that need fixing
show_specific_issues() {
    echo "🔧 Issues to Address:"
    echo ""
    
    # Group issues by severity
    local critical_count
    critical_count=$(jq -r '[.files[].detections[] | select(.severity == "critical")] | length' "$OUTPUT_FILE")
    
    local high_count
    high_count=$(jq -r '[.files[].detections[] | select(.severity == "high")] | length' "$OUTPUT_FILE")
    
    local medium_count
    medium_count=$(jq -r '[.files[].detections[] | select(.severity == "medium")] | length' "$OUTPUT_FILE")
    
    # Show critical issues first (must fix)
    if [ "$critical_count" -gt 0 ]; then
        echo "🚨 CRITICAL (Must Fix - $critical_count issues):"
        jq -r '.files[] | .file_path as $file | .detections[] | select(.severity == "critical") | 
               "   \($file):\(.line) - \(.pattern_name): \(.description)"' "$OUTPUT_FILE"
        echo ""
    fi
    
    # Show high priority issues
    if [ "$high_count" -gt 0 ]; then
        echo "🟡 HIGH PRIORITY ($high_count issues):"
        jq -r '.files[] | .file_path as $file | .detections[] | select(.severity == "high") | 
               "   \($file):\(.line) - \(.pattern_name): \(.description)"' "$OUTPUT_FILE"
        echo ""
    fi
    
    # Show medium issues (quality improvements)
    if [ "$medium_count" -gt 0 ] && [ "$VERBOSE" = "true" ]; then
        echo "🟢 MEDIUM PRIORITY ($medium_count issues):"
        jq -r '.files[] | .file_path as $file | .detections[] | select(.severity == "medium") | 
               "   \($file):\(.line) - \(.pattern_name): \(.description)"' "$OUTPUT_FILE"
        echo ""
    fi
    
    # Show file-level quality breakdown
    echo "📄 File Quality Breakdown:"
    jq -r '.files[] | select(.detections | length > 0) | 
           "   \(.file_path): \(.detections | length) issues, \(.quality_score)% quality"' "$OUTPUT_FILE"
    
    echo ""
    echo "💡 Next Steps:"
    echo "   1. Address CRITICAL issues first (blocking completion)"
    echo "   2. Improve HIGH PRIORITY issues (affects quality score)"
    echo "   3. Re-run validation: ./scripts/sniff_validation.sh \"$PLAN_ITEM\""
    echo "   4. Continue iteration until validation passes"
}

# Cleanup function
cleanup() {
    if [ -f "$OUTPUT_FILE" ] && [ "$VERBOSE" != "true" ]; then
        rm -f "$OUTPUT_FILE"
    fi
}

# Main execution
main() {
    echo "🎯 Sniff Quality Validation"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    
    # Setup cleanup
    trap cleanup EXIT
    
    # Check prerequisites
    check_prerequisites
    
    # Get changed files
    local changed_files
    changed_files=$(get_changed_files)
    
    if [ -z "$changed_files" ]; then
        print_success "No code files changed - validation passed by default"
        exit 0
    fi
    
    # Run analysis
    if ! run_sniff_analysis "$changed_files"; then
        exit 1
    fi
    
    # Interpret and report results
    if interpret_results; then
        exit 0
    else
        exit 1
    fi
}

# Help function
show_help() {
    cat << EOF
Sniff Quality Validation Script for Claude Code Integration

USAGE:
    $0 [PLAN_ITEM]

ARGUMENTS:
    PLAN_ITEM    Description of the current plan item (optional)

ENVIRONMENT VARIABLES:
    SNIFF_QUALITY_THRESHOLD    Minimum quality score (default: 85.0)
    SNIFF_MAX_CRITICAL        Maximum critical issues (default: 0)  
    SNIFF_ALLOW_TODOS         Allow TODO comments (default: false)
    SNIFF_VERBOSE             Enable verbose output (default: false)
    SNIFF_OUTPUT_FILE         Output file for results (default: validation.json)

EXAMPLES:
    $0 "Implement user authentication"
    SNIFF_QUALITY_THRESHOLD=90.0 $0 "Production release"
    SNIFF_VERBOSE=true $0 "Debug validation"

EXIT CODES:
    0    Validation passed - plan ready for completion
    1    Validation failed - continue iteration
    2    Script error or prerequisites not met

EOF
}

# Handle command line arguments
case "${1:-}" in
    -h|--help|help)
        show_help
        exit 0
        ;;
    *)
        main
        ;;
esac