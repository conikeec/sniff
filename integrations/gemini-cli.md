# Google Gemini CLI Sniff Integration

## Overview

Google's Gemini CLI is a command-line AI workflow tool that can integrate with external tools and automate development tasks. This integration leverages Gemini CLI's workflow automation capabilities to incorporate sniff verification into AI-driven development processes.

## Integration Method

**Method**: CLI Wrapper + Workflow Automation  
**Status**: ✅ Feasible (Open-source CLI with tool integration)  
**Requirements**: Gemini CLI installed, workflow configuration files

## Implementation Approach

### 1. Workflow Configuration

Gemini CLI can be configured with workflow files that define how it should interact with external tools like sniff.

#### Workflow File Structure
```
.gemini/
├── workflows/
│   ├── todo-workflow.yaml      # TODO management workflow
│   ├── verify-workflow.yaml    # Verification workflow
│   └── completion-workflow.yaml # Completion workflow
├── prompts/
│   ├── create-todo.md         # TODO creation prompts
│   ├── verify-code.md         # Code verification prompts
│   └── quality-check.md       # Quality assessment prompts
└── config.yaml               # Gemini CLI configuration
```

#### Todo Workflow Configuration
```yaml
# .gemini/workflows/todo-workflow.yaml
name: "sniff-todo-workflow"
description: "TODO workflow with sniff verification"

steps:
  - name: "create_todo"
    type: "interactive"
    prompt: |
      I need to create a new TODO with sniff verification tracking.
      
      Please help me:
      1. Create a unique TODO ID
      2. Define the task description
      3. Identify the files that will be modified
      4. Set quality thresholds
    
    tools:
      - name: "sniff-create-todo"
        command: "sniff create-todo"
        args: 
          - "--description"
          - "${TODO_DESCRIPTION}"
          - "--files" 
          - "${TODO_FILES}"
          - "--quality-threshold"
          - "${QUALITY_THRESHOLD:80}"

  - name: "track_progress"
    type: "automated"
    trigger: "file_change"
    prompt: |
      Files have been modified. Should we run a quality check to see how we're progressing?
    
    tools:
      - name: "sniff-analyze"
        command: "sniff"
        args:
          - "analyze-files"
          - "${CHANGED_FILES}"
          - "--format"
          - "table"

  - name: "verify_completion"
    type: "interactive"
    prompt: |
      I'm ready to complete a TODO. Let's verify it meets quality standards first.
      
      TODO ID: ${TODO_ID}
      Files: ${TODO_FILES}
      
      Please run sniff verification and tell me if it's ready for completion.
    
    tools:
      - name: "sniff-verify"
        command: "sniff"
        args:
          - "verify-todo"
          - "--todo-id"
          - "${TODO_ID}"
          - "--files"
          - "${TODO_FILES}"
          - "--format"
          - "json"
    
    post_process: |
      Based on the verification results:
      - If PASSED: Congratulate and mark TODO complete
      - If FAILED: Explain issues and suggest next steps
```

### 2. Custom Sniff Tool Integration

#### Tool Wrapper Script
```bash
#!/bin/bash
# .gemini/tools/sniff-todo-manager.sh

set -e

TODO_FILE=".sniff/todos.json"
GEMINI_CONTEXT=".gemini/context/current-todo.json"

# Initialize TODO tracking
init_todo_tracking() {
    mkdir -p .sniff .gemini/context
    if [ ! -f "$TODO_FILE" ]; then
        echo "[]" > "$TODO_FILE"
    fi
}

# Create TODO with Gemini CLI integration
create_todo() {
    local description="$1"
    local files="$2"
    local quality_threshold="${3:-80}"
    
    init_todo_tracking
    
    local todo_id="todo-$(date +%s)"
    local timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    
    # Create TODO entry
    local todo_entry=$(cat <<EOF
{
    "id": "$todo_id",
    "description": "$description", 
    "files": $(echo "$files" | jq -R 'split(",") | map(gsub("^\\s+|\\s+$"; ""))'),
    "quality_threshold": $quality_threshold,
    "status": "todo",
    "created_at": "$timestamp",
    "sniff_verified": false
}
EOF
    )
    
    # Add to TODO file
    jq ". += [$todo_entry]" "$TODO_FILE" > "$TODO_FILE.tmp" && mv "$TODO_FILE.tmp" "$TODO_FILE"
    
    # Set Gemini context
    echo "$todo_entry" > "$GEMINI_CONTEXT"
    
    echo "✅ Created TODO: $todo_id"
    echo "📝 Description: $description"
    echo "📁 Files: $files"
    echo "🎯 Quality Threshold: $quality_threshold%"
    
    return 0
}

# Verify TODO with sniff
verify_todo() {
    local todo_id="$1"
    
    # Get TODO details
    local todo=$(jq ".[] | select(.id == \"$todo_id\")" "$TODO_FILE")
    if [ -z "$todo" ]; then
        echo "❌ TODO not found: $todo_id"
        return 1
    fi
    
    local files=$(echo "$todo" | jq -r '.files | join(" ")')
    local quality_threshold=$(echo "$todo" | jq -r '.quality_threshold')
    
    echo "🔍 Verifying TODO: $todo_id"
    echo "📁 Files: $files"
    echo "🎯 Required Quality: $quality_threshold%"
    echo ""
    
    # Run sniff verification
    if sniff verify-todo --todo-id "$todo_id" --files $files --min-quality-score "$quality_threshold"; then
        # Update TODO status
        jq "(.[] | select(.id == \"$todo_id\")) |= (. + {\"sniff_verified\": true, \"verified_at\": \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\"})" "$TODO_FILE" > "$TODO_FILE.tmp" && mv "$TODO_FILE.tmp" "$TODO_FILE"
        
        echo ""
        echo "✅ TODO verified successfully!"
        echo "🎉 Ready to mark as complete."
        return 0
    else
        echo ""
        echo "❌ TODO verification failed."
        echo "🔄 Continue working on quality issues."
        return 1
    fi
}

# Complete TODO (only if verified)
complete_todo() {
    local todo_id="$1"
    
    # Check if TODO is verified
    local is_verified=$(jq -r ".[] | select(.id == \"$todo_id\") | .sniff_verified // false" "$TODO_FILE")
    
    if [ "$is_verified" != "true" ]; then
        echo "❌ TODO must be verified before completion"
        echo "🔍 Run verification first: verify_todo $todo_id"
        return 1
    fi
    
    # Mark as complete
    jq "(.[] | select(.id == \"$todo_id\")) |= (. + {\"status\": \"completed\", \"completed_at\": \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\"})" "$TODO_FILE" > "$TODO_FILE.tmp" && mv "$TODO_FILE.tmp" "$TODO_FILE"
    
    echo "🎉 TODO completed: $todo_id"
    return 0
}

# List active TODOs
list_todos() {
    if [ ! -f "$TODO_FILE" ]; then
        echo "📝 No TODOs found"
        return 0
    fi
    
    echo "📋 Active TODOs:"
    echo ""
    
    jq -r '.[] | select(.status != "completed") | "🆔 \(.id)\n📝 \(.description)\n📁 Files: \(.files | join(", "))\n🎯 Quality: \(.quality_threshold)%\n✅ Verified: \(.sniff_verified // false)\n"' "$TODO_FILE"
}

# Main command dispatcher
case "$1" in
    "create")
        create_todo "$2" "$3" "$4"
        ;;
    "verify")
        verify_todo "$2"
        ;;
    "complete")
        complete_todo "$2"
        ;;
    "list")
        list_todos
        ;;
    *)
        echo "Usage: $0 {create|verify|complete|list}"
        echo ""
        echo "Examples:"
        echo "  $0 create 'Implement auth' 'src/auth.ts,src/middleware/auth.ts' 85"
        echo "  $0 verify todo-1699123456"
        echo "  $0 complete todo-1699123456"
        echo "  $0 list"
        exit 1
        ;;
esac
```

### 3. Gemini CLI Prompts

#### Code Verification Prompt
```markdown
<!-- .gemini/prompts/verify-code.md -->
# Code Quality Verification

You are helping with a development workflow that uses sniff for quality verification.

## Context
- We use sniff to detect "bullshit patterns" in code
- TODOs must pass quality verification before completion
- Quality thresholds are configurable per TODO

## Your Role
When asked to verify code quality:

1. **Run sniff analysis** using the verify-todo command
2. **Interpret results** and explain any issues found
3. **Provide guidance** on how to fix issues
4. **Make completion recommendation** based on verification

## Verification Process
1. Run: `sniff verify-todo --todo-id {TODO_ID} --files {FILES} --min-quality-score {THRESHOLD}`
2. Analyze the output for:
   - Quality score vs threshold
   - Critical issues count
   - Specific pattern violations
3. Provide clear next steps

## Response Format
```
🔍 Verification Results for {TODO_ID}

📊 Quality Score: {SCORE}% (required: {THRESHOLD}%)
🚨 Critical Issues: {COUNT}
📈 Total Detections: {TOTAL}

{STATUS: PASSED/FAILED}

{If FAILED, list specific issues and suggestions}
{If PASSED, congratulate and confirm ready for completion}
```

Remember: Never bypass quality gates. Only mark TODOs complete after successful verification.
```

#### TODO Creation Prompt
```markdown
<!-- .gemini/prompts/create-todo.md -->
# TODO Creation with Sniff Integration

You are helping create a new TODO with integrated quality verification.

## Information Needed
1. **Description**: Clear, actionable task description
2. **Files**: List of files that will be modified
3. **Quality Threshold**: Minimum quality score (default: 80%)
4. **Scope**: Estimated complexity and requirements

## Creation Process
1. Generate unique TODO ID
2. Identify files based on task requirements
3. Set appropriate quality threshold
4. Create TODO entry with tracking
5. Provide guidance on implementation approach

## Quality Threshold Guidelines
- **90%+**: Critical production code, APIs, security features
- **85%**: Important features, user-facing components
- **80%**: Standard development tasks (default)
- **75%**: Experimental or prototype code
- **70%**: Documentation, configuration files

## Response Format
When creating a TODO:

```
✅ TODO Created Successfully!

🆔 ID: {TODO_ID}
📝 Task: {DESCRIPTION}
📁 Files: {FILE_LIST}
🎯 Quality Threshold: {THRESHOLD}%

📋 Next Steps:
1. Implement the feature/fix
2. Test your changes
3. Run verification: `sniff verify-todo --todo-id {TODO_ID}`
4. Address any quality issues
5. Mark complete when verified

💡 Tips:
- Avoid TODO comments in code
- Use proper error handling
- Remove debug prints
- Follow established patterns
```
```

### 4. Usage Examples

#### Interactive TODO Workflow
```bash
# Start Gemini CLI with sniff workflow
gemini --workflow=sniff-todo-workflow

# User interaction:
User: "I need to implement user authentication with JWT tokens"

Gemini: "I'll help you create a TODO with quality verification. Let me gather some details..."

[Gemini uses workflow to create TODO]

Gemini: "✅ Created TODO: todo-auth-1699123456
📝 Task: Implement user authentication with JWT tokens  
📁 Files: src/auth/jwt.ts, src/middleware/auth.ts, src/types/auth.ts
🎯 Quality Threshold: 85%

Ready to start implementation?"

User: "Yes, help me implement the JWT authentication"

[Gemini helps with implementation]

User: "I'm done implementing. Can you verify if it's ready?"

Gemini: [Runs sniff verification]
"🔍 Running verification...

📊 Quality Score: 88% (required: 85%) ✅
🚨 Critical Issues: 0 ✅
📈 Total Detections: 2

✅ PASSED - Your TODO is ready for completion!

The 2 minor detections are:
- Line 45: Consider more descriptive variable names
- Line 78: Optional: Add JSDoc comment

Shall I mark this TODO as complete?"
```

#### Automated Quality Monitoring
```bash
# Set up file watching with Gemini
gemini --watch --workflow=sniff-todo-workflow

# When files change:
Gemini: "🔍 I noticed changes to src/auth/jwt.ts. 
Running quality check...

📊 Quality Score: 72%
⚠️ Found 1 issue: TODO comment on line 34

Would you like me to help fix this before continuing?"
```

## Benefits

1. **AI-Guided Quality**: Gemini CLI provides intelligent guidance on quality issues
2. **Workflow Automation**: Automated verification triggers and quality monitoring
3. **Natural Language**: Interact with sniff through conversational interface
4. **Context Awareness**: Gemini maintains context about TODOs and quality requirements
5. **Proactive Monitoring**: File watching with automatic quality checks

## Technical Notes

- **Tool Integration**: Uses Gemini CLI's external tool integration capabilities
- **Workflow Automation**: YAML-based workflow definitions for reproducible processes
- **Context Management**: Maintains TODO context across Gemini CLI sessions
- **File Watching**: Optional file monitoring for proactive quality checks
- **JSON Storage**: Simple JSON-based TODO tracking compatible with other tools

This integration makes sniff verification a natural part of AI-assisted development with Gemini CLI, providing intelligent quality guidance throughout the development process.
