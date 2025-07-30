# AmpCode Sniff Integration

## Overview

AmpCode is Sourcegraph's agentic coding tool that provides autonomous reasoning and comprehensive code editing capabilities. This integration leverages AmpCode's extension mechanisms and CLI capabilities to incorporate sniff verification into agentic development workflows.

## Integration Method

**Method**: CLI Integration + Extension Hooks  
**Status**: 🔍 Research Required (Need to verify AmpCode's extension APIs)  
**Requirements**: AmpCode CLI access, extension/plugin capability

## Current Understanding

Based on available information, AmpCode:
- Operates as an agentic coding tool with autonomous reasoning
- Available as CLI, VS Code extension, Cursor integration, and Windsurf platform
- Handles complex tasks like frontend revamps and refactoring
- Provides comprehensive code editing capabilities

**⚠️ Research Note**: This integration design is based on AmpCode's described capabilities. Implementation details may need adjustment based on actual AmpCode API documentation and extension mechanisms.

## Proposed Integration Approach

### 1. CLI Integration Wrapper

Since AmpCode is available as a CLI, we can create a wrapper that integrates sniff verification.

#### AmpCode-Sniff Wrapper
```bash
#!/bin/bash
# ampcode-sniff-wrapper.sh

set -e

AMPCODE_CMD="ampcode"
SNIFF_CONFIG=".ampcode/sniff-config.json" 
TASK_TRACKING=".ampcode/tasks.json"

# Initialize AmpCode-Sniff integration
init_integration() {
    echo "🔧 Initializing AmpCode-Sniff integration..."
    
    mkdir -p .ampcode
    
    # Create default sniff configuration
    if [ ! -f "$SNIFF_CONFIG" ]; then
        cat > "$SNIFF_CONFIG" << EOF
{
    "quality_thresholds": {
        "default": 80,
        "critical_code": 90,
        "experimental": 70
    },
    "max_critical_issues": 0,
    "auto_verify": true,
    "verification_triggers": [
        "before_completion",
        "on_file_change"
    ]
}
EOF
    fi
    
    # Initialize task tracking
    if [ ! -f "$TASK_TRACKING" ]; then
        echo "[]" > "$TASK_TRACKING"
    fi
    
    echo "✅ Integration initialized"
}

# Create task with quality gates
create_task() {
    local description="$1"
    local files="$2"
    local quality_threshold="${3:-80}"
    
    local task_id="ampcode-task-$(date +%s)"
    local timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    
    # Create task entry
    local task_entry=$(cat <<EOF
{
    "id": "$task_id",
    "description": "$description",
    "files": $(echo "$files" | jq -R 'split(",") | map(gsub("^\\s+|\\s+$"; ""))'),
    "quality_threshold": $quality_threshold,
    "status": "created",
    "created_at": "$timestamp",
    "ampcode_session": true,
    "sniff_verified": false
}
EOF
    )
    
    # Add to task tracking
    jq ". += [$task_entry]" "$TASK_TRACKING" > "$TASK_TRACKING.tmp" && mv "$TASK_TRACKING.tmp" "$TASK_TRACKING"
    
    echo "✅ Created task: $task_id"
    echo "📝 Description: $description"
    echo "📁 Files: $files"
    echo "🎯 Quality Threshold: $quality_threshold%"
    
    export AMPCODE_CURRENT_TASK="$task_id"
    echo "$task_id"
}

# Run AmpCode with quality verification
run_ampcode_with_verification() {
    local task_id="$1"
    shift
    local ampcode_args="$@"
    
    echo "🚀 Running AmpCode with quality verification..."
    echo "🆔 Task: $task_id"
    
    # Update task status
    jq "(.[] | select(.id == \"$task_id\")) |= (. + {\"status\": \"in_progress\", \"started_at\": \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\"})" "$TASK_TRACKING" > "$TASK_TRACKING.tmp" && mv "$TASK_TRACKING.tmp" "$TASK_TRACKING"
    
    # Run AmpCode
    echo "🤖 Starting AmpCode..."
    if $AMPCODE_CMD $ampcode_args; then
        echo "✅ AmpCode completed successfully"
        
        # Auto-verify if enabled
        local auto_verify=$(jq -r '.auto_verify // true' "$SNIFF_CONFIG")
        if [ "$auto_verify" = "true" ]; then
            echo "🔍 Running automatic verification..."
            verify_task "$task_id"
        else
            echo "💡 Run 'verify_task $task_id' to check quality before marking complete"
        fi
    else
        echo "❌ AmpCode execution failed"
        jq "(.[] | select(.id == \"$task_id\")) |= (. + {\"status\": \"failed\", \"failed_at\": \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\"})" "$TASK_TRACKING" > "$TASK_TRACKING.tmp" && mv "$TASK_TRACKING.tmp" "$TASK_TRACKING"
        return 1
    fi
}

# Verify task completion with sniff
verify_task() {
    local task_id="$1"
    
    # Get task details
    local task=$(jq ".[] | select(.id == \"$task_id\")" "$TASK_TRACKING")
    if [ -z "$task" ]; then
        echo "❌ Task not found: $task_id"
        return 1
    fi
    
    local files=$(echo "$task" | jq -r '.files | join(" ")')
    local quality_threshold=$(echo "$task" | jq -r '.quality_threshold')
    
    echo "🔍 Verifying task completion with sniff..."
    echo "🆔 Task: $task_id" 
    echo "📁 Files: $files"
    echo "🎯 Required Quality: $quality_threshold%"
    echo ""
    
    # Run sniff verification
    if sniff verify-todo --todo-id "$task_id" --files $files --min-quality-score "$quality_threshold"; then
        # Update task status
        jq "(.[] | select(.id == \"$task_id\")) |= (. + {\"status\": \"completed\", \"sniff_verified\": true, \"verified_at\": \"$(date -u +"%Y-%m-%dT%H:%M:%SZ")\"})" "$TASK_TRACKING" > "$TASK_TRACKING.tmp" && mv "$TASK_TRACKING.tmp" "$TASK_TRACKING"
        
        echo ""
        echo "✅ Task verified and completed successfully!"
        echo "🎉 Quality gates passed - AmpCode task is ready to ship."
        return 0
    else
        echo ""
        echo "❌ Task verification failed"
        echo "🔄 AmpCode may need to iterate on quality issues"
        return 1
    fi
}

# List active tasks
list_tasks() {
    echo "📋 AmpCode Tasks:"
    echo ""
    
    if [ ! -f "$TASK_TRACKING" ]; then
        echo "No tasks found"
        return 0
    fi
    
    jq -r '.[] | "🆔 \(.id)\n📝 \(.description)\n📊 Status: \(.status)\n🎯 Quality: \(.quality_threshold)%\n✅ Verified: \(.sniff_verified // false)\n"' "$TASK_TRACKING"
}

# Main command dispatcher
case "$1" in
    "init")
        init_integration
        ;;
    "create-task")
        create_task "$2" "$3" "$4"
        ;;
    "run")
        task_id="$2"
        shift 2
        run_ampcode_with_verification "$task_id" "$@"
        ;;
    "verify")
        verify_task "$2"
        ;;
    "list")
        list_tasks
        ;;
    *)
        echo "AmpCode-Sniff Integration Wrapper"
        echo ""
        echo "Usage: $0 {init|create-task|run|verify|list}"
        echo ""
        echo "Commands:"
        echo "  init                                    - Initialize integration"
        echo "  create-task DESCRIPTION FILES [QUALITY] - Create task with quality gates"
        echo "  run TASK_ID [AMPCODE_ARGS...]          - Run AmpCode with verification"
        echo "  verify TASK_ID                         - Verify task completion"
        echo "  list                                   - List all tasks"
        echo ""
        echo "Examples:"
        echo "  $0 create-task 'Refactor auth system' 'src/auth.ts,src/middleware/auth.ts' 85"
        echo "  $0 run ampcode-task-1699123456 --prompt 'Implement user authentication'"
        echo "  $0 verify ampcode-task-1699123456"
        ;;
esac
```

### 2. AmpCode Extension Integration

If AmpCode supports extensions similar to VS Code, we could create an extension:

#### Extension Structure (Hypothetical)
```
ampcode-sniff-extension/
├── manifest.json         # Extension manifest
├── src/
│   ├── extension.js     # Main extension entry
│   ├── taskManager.js   # Task and quality management
│   └── sniffIntegration.js # Sniff CLI integration
└── config/
    └── default-settings.json
```

#### Extension Implementation (Conceptual)
```javascript
// extension.js - Hypothetical AmpCode extension
class AmpCodeSniffExtension {
    constructor() {
        this.taskManager = new TaskManager();
        this.sniffIntegration = new SniffIntegration();
    }
    
    onActivate() {
        // Register commands
        AmpCode.commands.register('sniff.createQualityTask', this.createQualityTask.bind(this));
        AmpCode.commands.register('sniff.verifyCompletion', this.verifyCompletion.bind(this));
        
        // Hook into AmpCode events
        AmpCode.events.on('task.beforeComplete', this.onBeforeTaskComplete.bind(this));
        AmpCode.events.on('files.changed', this.onFilesChanged.bind(this));
    }
    
    async createQualityTask(description, files, qualityThreshold = 80) {
        const taskId = await this.taskManager.createTask({
            description,
            files,
            qualityThreshold,
            ampcode: true
        });
        
        AmpCode.ui.showMessage(`Created quality-gated task: ${taskId}`);
        return taskId;
    }
    
    async onBeforeTaskComplete(task) {
        // Automatically verify before completion
        const verificationResult = await this.sniffIntegration.verifyTask(task.id);
        
        if (!verificationResult.passed) {
            AmpCode.ui.showWarning(`Task ${task.id} failed quality verification. Address issues before completion.`);
            return false; // Prevent completion
        }
        
        return true; // Allow completion
    }
    
    async onFilesChanged(files) {
        // Run continuous quality checks
        const analysis = await this.sniffIntegration.analyzeFiles(files);
        
        if (analysis.critical_issues > 0) {
            AmpCode.ui.showWarning(`${analysis.critical_issues} critical quality issues detected`);
        }
    }
}

// Register extension
AmpCode.extensions.register(new AmpCodeSniffExtension());
```

### 3. Usage Workflow Examples

#### Autonomous Development with Quality Gates
```bash
# Initialize integration
./ampcode-sniff-wrapper.sh init

# Create a quality-gated task
TASK_ID=$(./ampcode-sniff-wrapper.sh create-task \
    "Implement OAuth2 authentication system" \
    "src/auth/oauth.ts,src/middleware/auth.ts,src/types/auth.ts" \
    85)

# Run AmpCode with automatic verification
./ampcode-sniff-wrapper.sh run $TASK_ID \
    --prompt "Implement a complete OAuth2 authentication system with JWT tokens, refresh tokens, and proper error handling"

# Output:
# 🚀 Running AmpCode with quality verification...
# 🆔 Task: ampcode-task-1699123456
# 🤖 Starting AmpCode...
# [AmpCode executes autonomously]
# ✅ AmpCode completed successfully
# 🔍 Running automatic verification...
# ✅ Task verified and completed successfully!
# 🎉 Quality gates passed - AmpCode task is ready to ship.
```

#### Manual Verification Workflow
```bash
# Check current tasks
./ampcode-sniff-wrapper.sh list

# Output:
# 📋 AmpCode Tasks:
# 
# 🆔 ampcode-task-1699123456
# 📝 Implement OAuth2 authentication system
# 📊 Status: completed
# 🎯 Quality: 85%
# ✅ Verified: true

# Verify specific task manually
./ampcode-sniff-wrapper.sh verify ampcode-task-1699123456
```

### 4. Configuration Integration

#### AmpCode Settings Integration
```json
{
    "ampcode.sniff.integration": {
        "enabled": true,
        "autoVerify": true,
        "qualityThresholds": {
            "default": 80,
            "production": 90,
            "experimental": 70
        },
        "verificationTriggers": [
            "beforeCompletion",
            "onFileChange",
            "onRequest"
        ],
        "sniffCommand": "sniff",
        "taskTracking": true
    }
}
```

## Benefits

1. **Agentic Quality Assurance**: Sniff verification becomes part of autonomous development
2. **Intelligent Iteration**: AmpCode can automatically address quality issues
3. **Comprehensive Verification**: Works with AmpCode's complex task execution
4. **Quality-First Development**: Prevents completion of substandard code
5. **Autonomous Workflow**: Minimal human intervention while maintaining quality

## Research Required

To complete this integration, we need to research:

1. **AmpCode Extension API**: What extension mechanisms does AmpCode provide?
2. **CLI Integration Points**: How can external tools hook into AmpCode workflows?
3. **Event System**: What events does AmpCode expose for integration?
4. **Task Management**: How does AmpCode handle task creation and completion?
5. **Configuration System**: How are AmpCode settings and preferences managed?

## Next Steps

1. **Contact AmpCode Team**: Reach out to Sourcegraph for API documentation
2. **Review Documentation**: Study available AmpCode integration guides
3. **Test CLI Integration**: Verify the CLI wrapper approach works
4. **Prototype Extension**: Create a minimal extension to test integration points
5. **Validate Workflow**: Test the quality gate workflow with real AmpCode tasks

## Technical Notes

- **CLI Wrapper**: Provides immediate integration capability regardless of extension API
- **Quality Gates**: Prevents AmpCode from completing tasks that don't meet quality standards
- **Task Tracking**: Maintains task-quality associations throughout the development process
- **Autonomous Integration**: Designed to work with AmpCode's autonomous capabilities
- **Fallback Options**: Multiple integration approaches for maximum compatibility

This integration design provides a foundation for incorporating sniff verification into AmpCode workflows. Implementation details will be refined based on actual AmpCode API capabilities and documentation.
