# Claude Code Sniff Integration

## Overview

Claude Code is Anthropic's agentic coding tool that operates within terminal environments and maintains session context. This integration leverages Claude Code's session file management and MCP (Model Context Protocol) capabilities to incorporate sniff verification into AI-driven development workflows.

## Integration Method

**Method**: MCP Server + Session Hooks + Terminal Integration  
**Status**: ✅ Feasible (MCP support + session file access)  
**Requirements**: Claude Code with MCP, session file access, terminal integration

## Implementation Approach

### 1. MCP Server for Claude Code

Claude Code supports MCP for enhanced capabilities. We create an MCP server that provides sniff verification tools.

#### MCP Server Implementation
```typescript
// claude-code-sniff-server.ts
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { execSync } from 'child_process';
import { readFileSync, writeFileSync, existsSync, mkdirSync } from 'fs';
import { join } from 'path';

class ClaudeCodeSniffServer {
    private server: Server;
    private sessionPath: string;
    
    constructor() {
        this.sessionPath = process.env.CLAUDE_SESSION_PATH || '.claude';
        this.server = new Server(
            {
                name: 'claude-code-sniff-server',
                version: '1.0.0',
            },
            {
                capabilities: {
                    tools: {},
                    resources: {},
                },
            }
        );
        
        this.setupTools();
        this.setupResources();
    }
    
    private setupTools() {
        this.server.setRequestHandler('tools/list', async () => ({
            tools: [
                {
                    name: 'create_quality_gated_task',
                    description: 'Create a task with sniff quality verification requirements',
                    inputSchema: {
                        type: 'object',
                        properties: {
                            description: {
                                type: 'string',
                                description: 'Task description'
                            },
                            files: {
                                type: 'array',
                                items: { type: 'string' },
                                description: 'Files that will be modified for this task'
                            },
                            quality_threshold: {
                                type: 'number',
                                default: 80,
                                description: 'Minimum quality score required (0-100)'
                            },
                            max_critical_issues: {
                                type: 'number',
                                default: 0,
                                description: 'Maximum critical issues allowed'
                            }
                        },
                        required: ['description', 'files']
                    }
                },
                {
                    name: 'verify_task_completion',
                    description: 'Verify task completion using sniff quality analysis',
                    inputSchema: {
                        type: 'object',
                        properties: {
                            task_id: {
                                type: 'string',
                                description: 'Task ID to verify'
                            },
                            session_context: {
                                type: 'boolean',
                                default: true,
                                description: 'Include Claude Code session context in verification'
                            }
                        },
                        required: ['task_id']
                    }
                },
                {
                    name: 'continuous_quality_check',
                    description: 'Run continuous quality checks on modified files',
                    inputSchema: {
                        type: 'object',
                        properties: {
                            files: {
                                type: 'array',
                                items: { type: 'string' },
                                description: 'Files to check for quality issues'
                            },
                            report_format: {
                                type: 'string',
                                enum: ['summary', 'detailed', 'json'],
                                default: 'summary',
                                description: 'Format for quality report'
                            }
                        },
                        required: ['files']
                    }
                },
                {
                    name: 'analyze_session_quality',
                    description: 'Analyze overall quality of changes in current Claude Code session',
                    inputSchema: {
                        type: 'object',
                        properties: {
                            session_file: {
                                type: 'string',
                                description: 'Path to Claude Code session file'
                            },
                            include_history: {
                                type: 'boolean',
                                default: true,
                                description: 'Include historical quality trends'
                            }
                        }
                    }
                }
            ]
        }));
        
        this.server.setRequestHandler('tools/call', async (request) => {
            const { name, arguments: args } = request.params;
            
            switch (name) {
                case 'create_quality_gated_task':
                    return this.createQualityGatedTask(args);
                case 'verify_task_completion':
                    return this.verifyTaskCompletion(args);
                case 'continuous_quality_check':
                    return this.continuousQualityCheck(args);
                case 'analyze_session_quality':
                    return this.analyzeSessionQuality(args);
                default:
                    throw new Error(`Unknown tool: ${name}`);
            }
        });
    }
    
    private setupResources() {
        this.server.setRequestHandler('resources/list', async () => ({
            resources: [
                {
                    uri: 'claude-session://tasks',
                    name: 'Active Tasks',
                    description: 'List of active tasks with quality gates'
                },
                {
                    uri: 'claude-session://quality-report',
                    name: 'Quality Report',
                    description: 'Current session quality analysis'
                }
            ]
        }));
        
        this.server.setRequestHandler('resources/read', async (request) => {
            const { uri } = request.params;
            
            switch (uri) {
                case 'claude-session://tasks':
                    return this.getActiveTasks();
                case 'claude-session://quality-report':
                    return this.getQualityReport();
                default:
                    throw new Error(`Unknown resource: ${uri}`);
            }
        });
    }
    
    private async createQualityGatedTask(args: any) {
        const { description, files, quality_threshold = 80, max_critical_issues = 0 } = args;
        
        // Create task with unique ID
        const taskId = `task-${Date.now()}`;
        const timestamp = new Date().toISOString();
        
        const task = {
            id: taskId,
            description,
            files,
            quality_threshold,
            max_critical_issues,
            status: 'active',
            created_at: timestamp,
            session_id: this.getCurrentSessionId(),
            sniff_verified: false
        };
        
        // Save task to session directory
        this.saveTask(task);
        
        // Create Claude Code command for this task
        const commandFile = join(this.sessionPath, 'commands', `verify-${taskId}.sh`);
        this.createVerificationCommand(taskId, files, quality_threshold, max_critical_issues, commandFile);
        
        return {
            content: [
                {
                    type: 'text',
                    text: `✅ Quality-gated task created successfully!\n\n` +
                          `🆔 Task ID: ${taskId}\n` +
                          `📝 Description: ${description}\n` +
                          `📁 Files: ${files.join(', ')}\n` +
                          `🎯 Quality Threshold: ${quality_threshold}%\n` +
                          `🚨 Max Critical Issues: ${max_critical_issues}\n\n` +
                          `📋 Next Steps:\n` +
                          `1. Implement the feature/fix\n` +
                          `2. Run verification: verify-${taskId}\n` +
                          `3. Address any quality issues\n` +
                          `4. Mark complete when verified\n\n` +
                          `💡 Use "verify_task_completion" tool when ready for verification.`
                }
            ]
        };
    }
    
    private async verifyTaskCompletion(args: any) {
        const { task_id, session_context = true } = args;
        
        // Load task details
        const task = this.loadTask(task_id);
        if (!task) {
            return {
                content: [
                    {
                        type: 'text',
                        text: `❌ Task not found: ${task_id}`
                    }
                ]
            };
        }
        
        try {
            // Run sniff verification
            const command = [
                'sniff',
                'verify-todo',
                '--todo-id', task_id,
                '--files', ...task.files,
                '--min-quality-score', task.quality_threshold.toString(),
                '--max-critical-issues', task.max_critical_issues.toString(),
                '--format', 'json'
            ].join(' ');
            
            const result = execSync(command, { encoding: 'utf-8' });
            const verificationResult = JSON.parse(result);
            
            // Update task status
            task.sniff_verified = verificationResult.verification_passed;
            task.verified_at = new Date().toISOString();
            task.verification_result = verificationResult;
            
            if (verificationResult.verification_passed) {
                task.status = 'completed';
            }
            
            this.saveTask(task);
            
            // Format response
            const statusEmoji = verificationResult.verification_passed ? '✅' : '❌';
            const statusText = verificationResult.verification_passed ? 'PASSED' : 'FAILED';
            
            let response = `${statusEmoji} Task Verification: ${statusText}\n\n` +
                          `🆔 Task: ${task_id}\n` +
                          `📊 Quality Score: ${verificationResult.quality_score}% (required: ${task.quality_threshold}%)\n` +
                          `🚨 Critical Issues: ${verificationResult.critical_issues} (max: ${task.max_critical_issues})\n` +
                          `📈 Total Detections: ${verificationResult.analysis_results.total_detections}\n\n`;
            
            if (verificationResult.verification_passed) {
                response += `🎉 Task is ready for completion!\n` +
                           `✨ All quality gates passed. You can mark this task as done.`;
            } else {
                response += `⚠️ Quality issues need to be addressed:\n\n`;
                
                // Add specific issues
                const issues = verificationResult.analysis_results.file_results
                    .flatMap(f => f.detections)
                    .slice(0, 5); // Show first 5 issues
                
                issues.forEach((issue, index) => {
                    response += `${index + 1}. ${issue.rule_name} (line ${issue.line_number}): ${issue.code_snippet.trim()}\n`;
                });
                
                if (verificationResult.analysis_results.total_detections > 5) {
                    response += `... and ${verificationResult.analysis_results.total_detections - 5} more issues\n`;
                }
                
                response += `\n🔄 Please fix these issues and run verification again.`;
            }
            
            // Add session context if requested
            if (session_context) {
                response += `\n\n📊 Session Quality Summary:\n`;
                response += this.getSessionQualitySummary();
            }
            
            return {
                content: [
                    {
                        type: 'text',
                        text: response
                    }
                ]
            };
            
        } catch (error) {
            return {
                content: [
                    {
                        type: 'text',
                        text: `❌ Verification failed: ${error.message}\n\n` +
                              `Please ensure sniff is installed and the files exist.`
                    }
                ]
            };
        }
    }
    
    private async continuousQualityCheck(args: any) {
        const { files, report_format = 'summary' } = args;
        
        try {
            const command = [
                'sniff',
                'analyze-files',
                ...files,
                '--format', 'json'
            ].join(' ');
            
            const result = execSync(command, { encoding: 'utf-8' });
            const analysisResult = JSON.parse(result);
            
            let response = '';
            
            switch (report_format) {
                case 'summary':
                    response = `📊 Quality Check Summary\n\n` +
                              `📁 Files: ${analysisResult.total_files}\n` +
                              `🎯 Average Quality: ${analysisResult.average_quality_score.toFixed(1)}%\n` +
                              `🚨 Critical Issues: ${analysisResult.critical_issues}\n` +
                              `📈 Total Issues: ${analysisResult.total_detections}`;
                    
                    if (analysisResult.critical_issues > 0) {
                        response += `\n\n⚠️ Attention: ${analysisResult.critical_issues} critical issues need immediate attention.`;
                    }
                    break;
                    
                case 'detailed':
                    response = this.formatDetailedQualityReport(analysisResult);
                    break;
                    
                case 'json':
                    response = JSON.stringify(analysisResult, null, 2);
                    break;
            }
            
            return {
                content: [
                    {
                        type: 'text',
                        text: response
                    }
                ]
            };
            
        } catch (error) {
            return {
                content: [
                    {
                        type: 'text',
                        text: `❌ Quality check failed: ${error.message}`
                    }
                ]
            };
        }
    }
    
    private formatDetailedQualityReport(analysisResult: any): string {
        let report = `📊 Detailed Quality Report\n`;
        report += `${'='.repeat(50)}\n\n`;
        
        report += `📈 Overall Metrics:\n`;
        report += `├─ Files Analyzed: ${analysisResult.total_files}\n`;
        report += `├─ Average Quality: ${analysisResult.average_quality_score.toFixed(1)}%\n`;
        report += `├─ Critical Issues: ${analysisResult.critical_issues}\n`;
        report += `└─ Total Detections: ${analysisResult.total_detections}\n\n`;
        
        if (analysisResult.file_results.length > 0) {
            report += `📄 File Analysis:\n`;
            
            analysisResult.file_results.forEach((fileResult, index) => {
                const isLast = index === analysisResult.file_results.length - 1;
                const prefix = isLast ? '└─' : '├─';
                
                report += `${prefix} ${fileResult.file_path} (${fileResult.quality_score.toFixed(1)}%)\n`;
                
                if (fileResult.detections.length > 0) {
                    fileResult.detections.slice(0, 3).forEach((detection, detIndex) => {
                        const isLastDetection = detIndex === Math.min(2, fileResult.detections.length - 1);
                        const detectionPrefix = isLast 
                            ? (isLastDetection ? '   └─' : '   ├─')
                            : (isLastDetection ? '│  └─' : '│  ├─');
                        
                        report += `${detectionPrefix} ${detection.rule_name} (line ${detection.line_number})\n`;
                    });
                    
                    if (fileResult.detections.length > 3) {
                        const morePrefix = isLast ? '   └─' : '│  └─';
                        report += `${morePrefix} ... and ${fileResult.detections.length - 3} more issues\n`;
                    }
                }
            });
        }
        
        return report;
    }
    
    // Helper methods
    private getCurrentSessionId(): string {
        // Generate or retrieve current Claude Code session ID
        return `session-${Date.now()}`;
    }
    
    private saveTask(task: any): void {
        const tasksDir = join(this.sessionPath, 'tasks');
        mkdirSync(tasksDir, { recursive: true });
        
        const taskFile = join(tasksDir, `${task.id}.json`);
        writeFileSync(taskFile, JSON.stringify(task, null, 2));
    }
    
    private loadTask(taskId: string): any {
        const taskFile = join(this.sessionPath, 'tasks', `${taskId}.json`);
        if (!existsSync(taskFile)) {
            return null;
        }
        
        return JSON.parse(readFileSync(taskFile, 'utf-8'));
    }
    
    private createVerificationCommand(taskId: string, files: string[], qualityThreshold: number, maxCritical: number, commandFile: string): void {
        const commandsDir = join(this.sessionPath, 'commands');
        mkdirSync(commandsDir, { recursive: true });
        
        const command = `#!/bin/bash
# Claude Code Sniff Verification Command for ${taskId}
echo "🔍 Verifying task: ${taskId}"
echo "📁 Files: ${files.join(' ')}"
echo ""

sniff verify-todo \\
    --todo-id "${taskId}" \\
    --files ${files.join(' ')} \\
    --min-quality-score ${qualityThreshold} \\
    --max-critical-issues ${maxCritical}

echo ""
echo "💡 Use 'verify_task_completion' tool in Claude Code for detailed analysis"
`;
        
        writeFileSync(commandFile, command);
        execSync(`chmod +x "${commandFile}"`);
    }
    
    private getSessionQualitySummary(): string {
        // Implement session quality summary logic
        return `Average session quality: 87% | Active tasks: 2 | Completed: 5`;
    }
    
    private async getActiveTasks() {
        const tasksDir = join(this.sessionPath, 'tasks');
        if (!existsSync(tasksDir)) {
            return {
                contents: [
                    {
                        type: 'text',
                        text: 'No active tasks found.'
                    }
                ]
            };
        }
        
        // Implementation for listing active tasks
        return {
            contents: [
                {
                    type: 'text',
                    text: 'Active tasks implementation...'
                }
            ]
        };
    }
    
    private async getQualityReport() {
        // Implementation for quality report
        return {
            contents: [
                {
                    type: 'text',
                    text: 'Quality report implementation...'
                }
            ]
        };
    }
}

// Start the MCP server
const server = new ClaudeCodeSniffServer();
const transport = new StdioServerTransport();
server.connect(transport);
```

### 2. Claude Code Configuration

#### MCP Configuration
```json
{
  "mcpServers": {
    "sniff-verification": {
      "command": "node",
      "args": ["./claude-code-sniff-server.js"],
      "env": {
        "CLAUDE_SESSION_PATH": ".claude"
      }
    }
  }
}
```

#### Session Integration Script
```bash
#!/bin/bash
# .claude/commands/sniff-integration.sh

# Claude Code Sniff Integration
# This script provides sniff integration for Claude Code sessions

CLAUDE_SESSION_PATH="${CLAUDE_SESSION_PATH:-.claude}"
TASKS_DIR="$CLAUDE_SESSION_PATH/tasks"
QUALITY_LOG="$CLAUDE_SESSION_PATH/quality.log"

# Initialize sniff integration
init_sniff_integration() {
    echo "🔧 Initializing sniff integration for Claude Code..."
    
    mkdir -p "$TASKS_DIR"
    mkdir -p "$CLAUDE_SESSION_PATH/commands"
    mkdir -p "$CLAUDE_SESSION_PATH/reports"
    
    # Create quality tracking log
    if [ ! -f "$QUALITY_LOG" ]; then
        echo "# Claude Code Session Quality Log" > "$QUALITY_LOG"
        echo "# Generated on $(date)" >> "$QUALITY_LOG"
        echo "" >> "$QUALITY_LOG"
    fi
    
    echo "✅ Sniff integration initialized"
}

# Monitor file changes and run quality checks
monitor_quality() {
    echo "👀 Starting quality monitoring..."
    
    # Watch for file changes in current directory
    fswatch -o . | while read f; do
        # Get list of modified files (simplified)
        modified_files=$(git diff --name-only HEAD~1 2>/dev/null || echo "")
        
        if [ -n "$modified_files" ]; then
            echo "🔍 Files changed, running quality check..."
            echo "$(date): Quality check triggered by file changes" >> "$QUALITY_LOG"
            
            # Run sniff analysis on changed files
            sniff analyze-files $modified_files --format table >> "$QUALITY_LOG" 2>&1
            echo "" >> "$QUALITY_LOG"
        fi
    done
}

# Generate session quality report
generate_session_report() {
    local output_file="$CLAUDE_SESSION_PATH/reports/session-quality-$(date +%Y%m%d-%H%M%S).md"
    
    echo "📊 Generating session quality report..."
    
    cat > "$output_file" << EOF
# Claude Code Session Quality Report

**Generated**: $(date)
**Session Path**: $CLAUDE_SESSION_PATH

## Active Tasks

$(find "$TASKS_DIR" -name "*.json" -exec echo "- {}" \; | head -10)

## Quality Summary

$(tail -50 "$QUALITY_LOG")

## Recommendations

EOF
    
    # Add automated recommendations based on quality log
    if grep -q "Critical" "$QUALITY_LOG"; then
        echo "⚠️ **Critical Issues Found**: Address critical issues before continuing." >> "$output_file"
    fi
    
    if grep -q "TODO" "$QUALITY_LOG"; then
        echo "📝 **TODO Comments**: Remove TODO comments before task completion." >> "$output_file"
    fi
    
    echo "✅ Report generated: $output_file"
}

# Command dispatcher
case "$1" in
    "init")
        init_sniff_integration
        ;;
    "monitor")
        monitor_quality
        ;;
    "report")
        generate_session_report
        ;;
    *)
        echo "Usage: $0 {init|monitor|report}"
        echo ""
        echo "Commands:"
        echo "  init     - Initialize sniff integration"
        echo "  monitor  - Start quality monitoring"
        echo "  report   - Generate session quality report"
        ;;
esac
```

### 3. Usage Examples

#### Task Creation with Quality Gates
```
User: "I need to implement a REST API for user management with proper authentication"

Claude Code: [Uses create_quality_gated_task tool]

✅ Quality-gated task created successfully!

🆔 Task ID: task-1699123456789
📝 Description: Implement REST API for user management with proper authentication
📁 Files: src/routes/users.ts, src/middleware/auth.ts, src/controllers/user.ts
🎯 Quality Threshold: 85%
🚨 Max Critical Issues: 0

📋 Next Steps:
1. Implement the feature/fix
2. Run verification: verify-task-1699123456789
3. Address any quality issues
4. Mark complete when verified

I'll help you implement this API. Let's start with the authentication middleware...
```

#### Continuous Quality Feedback
```
Claude Code: [Automatically runs continuous_quality_check during development]

📊 Quality Check Summary

📁 Files: 3
🎯 Average Quality: 78%
🚨 Critical Issues: 1
📈 Total Issues: 4

⚠️ Attention: 1 critical issue needs immediate attention.

I notice the quality score is below your threshold. Let me help fix these issues:

1. **Critical**: Line 45 in auth.ts - Using .unwrap() without proper error handling
2. **Medium**: Line 23 in users.ts - TODO comment indicating incomplete implementation
3. **Low**: Line 67 in user.ts - Consider more descriptive variable name

Let me fix these issues for you...
```

#### Task Verification
```
User: "I think I'm done with the API implementation. Can you verify it's ready?"

Claude Code: [Uses verify_task_completion tool]

✅ Task Verification: PASSED

🆔 Task: task-1699123456789
📊 Quality Score: 88% (required: 85%)
🚨 Critical Issues: 0 (max: 0)
📈 Total Detections: 2

🎉 Task is ready for completion!
✨ All quality gates passed. You can mark this task as done.

📊 Session Quality Summary:
Average session quality: 87% | Active tasks: 1 | Completed: 3

Excellent work! Your REST API implementation meets all quality requirements. The remaining 2 minor detections are just suggestions for improvement and don't block completion.
```

## Benefits

1. **Session Integration**: Seamlessly integrates with Claude Code's session management
2. **AI-Guided Quality**: Claude provides intelligent guidance on quality issues
3. **Automated Verification**: Quality checks happen automatically during development
4. **Context Preservation**: Maintains task context across Claude Code sessions
5. **Terminal Native**: Works naturally within Claude Code's terminal environment

## Technical Features

- **MCP Integration**: Native tool integration using Model Context Protocol
- **Session File Management**: Tracks tasks and quality data in Claude session files
- **Command Generation**: Creates custom verification commands for easy terminal use
- **Quality Monitoring**: Continuous file monitoring with automated quality checks
- **Rich Reporting**: Detailed quality reports with actionable recommendations

This integration makes sniff verification a core part of the Claude Code development experience, ensuring quality gates are maintained throughout AI-assisted development workflows.
