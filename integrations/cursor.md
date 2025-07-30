# Cursor AI Editor Sniff Integration

## Overview

Cursor is an AI-powered code editor built on VS Code that provides enhanced AI capabilities. Since Cursor is based on VS Code, it inherits the task system and extension capabilities, making sniff integration feasible through similar mechanisms.

## Integration Method

**Method**: Extension + MCP Server (Model Context Protocol)  
**Status**: ✅ Feasible (VS Code-based + MCP support)  
**Requirements**: Cursor extension capabilities, MCP server integration

## Implementation Approach

### 1. MCP Server Integration

Cursor supports MCP (Model Context Protocol) for enhanced AI capabilities. We can create an MCP server that provides sniff verification as a tool for the AI to use.

#### MCP Server Structure
```
sniff-mcp-server/
├── package.json
├── src/
│   ├── server.ts        # MCP server implementation
│   ├── tools.ts         # Sniff tool definitions
│   └── types.ts         # Type definitions
└── mcp.json            # MCP configuration
```

#### MCP Server Implementation
```typescript
// server.ts
import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { Tool, CallToolRequest } from '@modelcontextprotocol/sdk/types.js';
import { execSync } from 'child_process';

class SniffMCPServer {
    private server: Server;
    
    constructor() {
        this.server = new Server(
            {
                name: 'sniff-verification-server',
                version: '1.0.0',
            },
            {
                capabilities: {
                    tools: {},
                },
            }
        );
        
        this.setupTools();
    }
    
    private setupTools() {
        // Tool: Verify TODO with sniff
        this.server.setRequestHandler('tools/list', async () => ({
            tools: [
                {
                    name: 'verify_todo_with_sniff',
                    description: 'Verify TODO completion using sniff quality analysis',
                    inputSchema: {
                        type: 'object',
                        properties: {
                            todoId: {
                                type: 'string',
                                description: 'Unique identifier for the TODO'
                            },
                            files: {
                                type: 'array',
                                items: { type: 'string' },
                                description: 'List of files to analyze'
                            },
                            minQualityScore: {
                                type: 'number',
                                default: 80,
                                description: 'Minimum quality score required (0-100)'
                            },
                            maxCriticalIssues: {
                                type: 'number', 
                                default: 0,
                                description: 'Maximum critical issues allowed'
                            }
                        },
                        required: ['todoId', 'files']
                    }
                },
                {
                    name: 'analyze_files_with_sniff',
                    description: 'Analyze files for code quality issues using sniff',
                    inputSchema: {
                        type: 'object',
                        properties: {
                            files: {
                                type: 'array',
                                items: { type: 'string' },
                                description: 'List of files to analyze'
                            },
                            format: {
                                type: 'string',
                                enum: ['table', 'json', 'markdown'],
                                default: 'json',
                                description: 'Output format'
                            }
                        },
                        required: ['files']
                    }
                },
                {
                    name: 'create_todo_with_tracking',
                    description: 'Create a new TODO with file tracking for sniff verification',
                    inputSchema: {
                        type: 'object',
                        properties: {
                            description: {
                                type: 'string',
                                description: 'TODO description'
                            },
                            files: {
                                type: 'array',
                                items: { type: 'string' },
                                description: 'Files associated with this TODO'
                            },
                            minQualityScore: {
                                type: 'number',
                                default: 80,
                                description: 'Minimum quality score required'
                            }
                        },
                        required: ['description', 'files']
                    }
                }
            ]
        }));
        
        this.server.setRequestHandler('tools/call', async (request) => {
            const { name, arguments: args } = request.params;
            
            switch (name) {
                case 'verify_todo_with_sniff':
                    return this.verifyTodo(args);
                case 'analyze_files_with_sniff':
                    return this.analyzeFiles(args);
                case 'create_todo_with_tracking':
                    return this.createTodo(args);
                default:
                    throw new Error(`Unknown tool: ${name}`);
            }
        });
    }
    
    private async verifyTodo(args: any) {
        const { todoId, files, minQualityScore = 80, maxCriticalIssues = 0 } = args;
        
        try {
            const command = [
                'sniff',
                'verify-todo',
                '--todo-id', todoId,
                '--files', ...files,
                '--min-quality-score', minQualityScore.toString(),
                '--max-critical-issues', maxCriticalIssues.toString(),
                '--format', 'json'
            ].join(' ');
            
            const result = execSync(command, { encoding: 'utf-8' });
            const verificationResult = JSON.parse(result);
            
            return {
                content: [
                    {
                        type: 'text',
                        text: `TODO Verification Results for ${todoId}:\n\n` +
                              `✅ Status: ${verificationResult.verification_passed ? 'PASSED' : 'FAILED'}\n` +
                              `📊 Quality Score: ${verificationResult.quality_score}% (required: ${minQualityScore}%)\n` +
                              `🚨 Critical Issues: ${verificationResult.critical_issues} (max: ${maxCriticalIssues})\n\n` +
                              (verificationResult.verification_passed 
                                ? '🎉 TODO is ready to be marked complete!' 
                                : '⚠️ Please address the quality issues before completing this TODO.')
                    }
                ]
            };
        } catch (error) {
            return {
                content: [
                    {
                        type: 'text',
                        text: `❌ Verification failed: ${error.message}`
                    }
                ]
            };
        }
    }
    
    private async analyzeFiles(args: any) {
        const { files, format = 'json' } = args;
        
        try {
            const command = [
                'sniff',
                'analyze-files',
                ...files,
                '--format', format
            ].join(' ');
            
            const result = execSync(command, { encoding: 'utf-8' });
            
            return {
                content: [
                    {
                        type: 'text',
                        text: `Sniff Analysis Results:\n\n${result}`
                    }
                ]
            };
        } catch (error) {
            return {
                content: [
                    {
                        type: 'text',
                        text: `❌ Analysis failed: ${error.message}`
                    }
                ]
            };
        }
    }
    
    private async createTodo(args: any) {
        const { description, files, minQualityScore = 80 } = args;
        const todoId = `todo-${Date.now()}`;
        
        // Save TODO to a tracking file
        const todoData = {
            id: todoId,
            description,
            files,
            minQualityScore,
            status: 'todo',
            createdAt: new Date().toISOString()
        };
        
        // In a real implementation, this would save to a proper storage
        const fs = require('fs');
        const todosFile = '.sniff/todos.json';
        
        try {
            let todos = [];
            if (fs.existsSync(todosFile)) {
                todos = JSON.parse(fs.readFileSync(todosFile, 'utf-8'));
            }
            todos.push(todoData);
            
            fs.mkdirSync('.sniff', { recursive: true });
            fs.writeFileSync(todosFile, JSON.stringify(todos, null, 2));
            
            return {
                content: [
                    {
                        type: 'text',
                        text: `✅ TODO created successfully!\n\n` +
                              `🆔 ID: ${todoId}\n` +
                              `📝 Description: ${description}\n` +
                              `📁 Files: ${files.join(', ')}\n` +
                              `🎯 Quality Threshold: ${minQualityScore}%\n\n` +
                              `Use "verify_todo_with_sniff" to check quality before marking complete.`
                    }
                ]
            };
        } catch (error) {
            return {
                content: [
                    {
                        type: 'text',
                        text: `❌ Failed to create TODO: ${error.message}`
                    }
                ]
            };
        }
    }
}

// Start the MCP server
const server = new SniffMCPServer();
const transport = new StdioServerTransport();
server.connect(transport);
```

### 2. MCP Configuration

#### mcp.json (for Cursor)
```json
{
    "mcpServers": {
        "sniff-verification": {
            "command": "node",
            "args": ["./sniff-mcp-server/dist/server.js"]
        }
    }
}
```

### 3. VS Code Extension Integration

Since Cursor is VS Code-based, the VS Code extension approach also works:

```typescript
// Cursor-specific enhancements
export function activate(context: vscode.ExtensionContext) {
    // Standard VS Code extension functionality
    // ... (same as VS Code integration)
    
    // Cursor-specific AI integration
    const cursorAICommand = vscode.commands.registerCommand('sniff.askCursorAI', async () => {
        const editor = vscode.window.activeTextEditor;
        if (!editor) return;
        
        const selection = editor.selection;
        const selectedText = editor.document.getText(selection);
        
        // Use Cursor's AI to analyze code with sniff context
        const prompt = `Please analyze this code for potential quality issues that sniff might detect:\n\n${selectedText}\n\nConsider patterns like:\n- TODO comments\n- Unwrap without context\n- Debug prints\n- Poor error handling`;
        
        vscode.window.showInformationMessage('Asking Cursor AI to analyze code quality...');
        // This would integrate with Cursor's AI system
    });
    
    context.subscriptions.push(cursorAICommand);
}
```

## Usage Workflow

### 1. AI-Assisted TODO Creation
```
User: "I need to implement user authentication. Create a TODO and track the files I'll modify."

Cursor AI: [Uses create_todo_with_tracking tool]
✅ TODO created: todo-1699123456
📝 Description: Implement user authentication
📁 Files: src/auth.ts, src/middleware/auth.ts, src/types/user.ts
🎯 Quality Threshold: 80%
```

### 2. Implementation with AI Assistance
```
User: "Help me implement the authentication system"

Cursor AI: [Generates code for the tracked files]
[User reviews and refines the implementation]
```

### 3. AI-Triggered Verification
```
User: "I'm done implementing. Can you verify the TODO is ready for completion?"

Cursor AI: [Uses verify_todo_with_sniff tool]
📊 Quality Score: 85% (required: 80%) ✅
🚨 Critical Issues: 0 (max: 0) ✅
🎉 TODO is ready to be marked complete!
```

### 4. Continuous Quality Feedback
```
User: "Check the quality of my current changes"

Cursor AI: [Uses analyze_files_with_sniff tool]
Found 2 issues in src/auth.ts:
- Line 45: TODO comment indicating incomplete implementation
- Line 78: Unwrap without proper error handling context

Would you like me to help fix these issues?
```

## Cursor-Specific Benefits

1. **AI Integration**: Sniff verification becomes part of AI-assisted development
2. **Natural Language**: Users can ask for verification in natural language
3. **Proactive Analysis**: AI can suggest running sniff verification
4. **Code Generation**: AI can generate code that passes sniff verification
5. **Context Awareness**: AI maintains context about TODOs and quality requirements

## MCP Server Benefits

1. **Native AI Tool**: Sniff becomes a native tool for Cursor's AI
2. **Consistent Interface**: Standardized way to interact with sniff
3. **Rich Responses**: Structured responses with formatting
4. **Extensible**: Easy to add new sniff-related tools

## Installation

### 1. Install MCP Server
```bash
npm install -g sniff-mcp-server
```

### 2. Configure Cursor
Add to `mcp.json`:
```json
{
    "mcpServers": {
        "sniff-verification": {
            "command": "sniff-mcp-server"
        }
    }
}
```

### 3. Install Extension (Optional)
Install the Cursor extension for additional UI integration.

## Technical Notes

- **MCP Compatibility**: Uses standard MCP protocol for tool integration
- **Error Handling**: Proper error handling for sniff command failures
- **File Tracking**: Maintains TODO-file associations in `.sniff/todos.json`
- **AI Context**: Provides rich context to AI about verification results
- **Cross-Platform**: Works on all platforms supported by Cursor

This integration makes sniff verification a natural part of the AI-assisted development workflow in Cursor, providing quality gates without disrupting the conversational interface.
