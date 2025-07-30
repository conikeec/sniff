# VS Code Sniff Integration

## Overview

VS Code provides robust task integration through its Tasks API, making it ideal for integrating sniff verification into TODO workflows. This integration uses VS Code's native task system to run sniff verification before marking TODOs complete.

## Integration Method

**Method**: VS Code Extension + Task Integration  
**Status**: ✅ Feasible (Well-documented APIs)  
**Requirements**: VS Code Extension API, tasks.json configuration

## Implementation Approach

### 1. VS Code Extension Structure

```
sniff-vscode-extension/
├── package.json          # Extension manifest
├── src/
│   ├── extension.ts     # Main extension entry point
│   ├── taskProvider.ts  # Task provider for sniff verification
│   └── todoManager.ts   # TODO tracking and management
└── .vscode/
    └── tasks.json       # Task configuration template
```

### 2. Core Extension Features

#### A. TODO Tracking
```typescript
// todoManager.ts
interface SniffTodo {
    id: string;
    description: string;
    files: string[];
    minQualityScore: number;
    maxCriticalIssues: number;
    status: 'todo' | 'in-progress' | 'completed';
    sniffVerified: boolean;
}

class TodoManager {
    private todos: SniffTodo[] = [];
    
    createTodo(description: string, files: string[], quality: number = 80): string {
        const todo: SniffTodo = {
            id: generateId(),
            description,
            files,
            minQualityScore: quality,
            maxCriticalIssues: 0,
            status: 'todo',
            sniffVerified: false
        };
        this.todos.push(todo);
        return todo.id;
    }
    
    async completeTodo(todoId: string): Promise<boolean> {
        const todo = this.todos.find(t => t.id === todoId);
        if (!todo) return false;
        
        // Run sniff verification before completion
        const verificationResult = await this.runSniffVerification(todo);
        
        if (verificationResult.passed) {
            todo.status = 'completed';
            todo.sniffVerified = true;
            vscode.window.showInformationMessage(`✅ TODO '${todo.description}' completed with sniff verification`);
            return true;
        } else {
            vscode.window.showWarningMessage(`❌ TODO '${todo.description}' failed verification. Continue working.`);
            return false;
        }
    }
}
```

#### B. Task Provider Integration
```typescript
// taskProvider.ts
export class SniffTaskProvider implements vscode.TaskProvider {
    provideTasks(): vscode.Task[] {
        const tasks: vscode.Task[] = [];
        
        // Add sniff verification task
        const sniffTask = new vscode.Task(
            { type: 'sniff-verify' },
            vscode.TaskScope.Workspace,
            'Verify TODO',
            'sniff',
            new vscode.ShellExecution('sniff', ['verify-todo', '--todo-id', '${input:todoId}', '--files', '${input:files}']),
            '$sniff-verify'
        );
        
        tasks.push(sniffTask);
        return tasks;
    }
}
```

### 3. Task Configuration (tasks.json)

```json
{
    "version": "2.0.0",
    "tasks": [
        {
            "label": "Sniff: Verify TODO",
            "type": "shell",
            "command": "sniff",
            "args": [
                "verify-todo",
                "--todo-id", "${input:todoId}",
                "--files", "${input:files}",
                "--min-quality-score", "${input:qualityScore}",
                "--format", "json"
            ],
            "group": "test",
            "presentation": {
                "echo": true,
                "reveal": "always",
                "focus": false,
                "panel": "shared"
            },
            "problemMatcher": {
                "base": "$tsc",
                "pattern": {
                    "regexp": "^(.*):(\\d+):(\\d+):\\s+(warning|error):\\s+(.*)$",
                    "file": 1,
                    "line": 2,
                    "column": 3,
                    "severity": 4,
                    "message": 5
                }
            }
        },
        {
            "label": "Sniff: Analyze Files",
            "type": "shell",
            "command": "sniff",
            "args": [
                "analyze-files",
                "${input:filesToAnalyze}",
                "--format", "table"
            ],
            "group": "test"
        }
    ],
    "inputs": [
        {
            "id": "todoId",
            "description": "TODO ID to verify",
            "type": "promptString"
        },
        {
            "id": "files",
            "description": "Files to verify (space-separated)",
            "type": "promptString"
        },
        {
            "id": "qualityScore",
            "description": "Minimum quality score (0-100)",
            "type": "promptString",
            "default": "80"
        },
        {
            "id": "filesToAnalyze",
            "description": "Files to analyze",
            "type": "promptString",
            "default": "${workspaceFolder}"
        }
    ]
}
```

### 4. Extension Commands

```typescript
// extension.ts
export function activate(context: vscode.ExtensionContext) {
    const todoManager = new TodoManager();
    
    // Register commands
    const createTodoCommand = vscode.commands.registerCommand('sniff.createTodo', async () => {
        const description = await vscode.window.showInputBox({
            prompt: 'Enter TODO description',
            placeHolder: 'Implement user authentication'
        });
        
        if (description) {
            const files = await vscode.window.showInputBox({
                prompt: 'Enter files to track (comma-separated)',
                placeHolder: 'src/auth.ts, src/middleware/auth.ts'
            });
            
            const fileList = files ? files.split(',').map(f => f.trim()) : [];
            const todoId = todoManager.createTodo(description, fileList);
            
            vscode.window.showInformationMessage(`Created TODO: ${todoId}`);
        }
    });
    
    const completeTodoCommand = vscode.commands.registerCommand('sniff.completeTodo', async () => {
        const todoId = await vscode.window.showInputBox({
            prompt: 'Enter TODO ID to complete'
        });
        
        if (todoId) {
            await todoManager.completeTodo(todoId);
        }
    });
    
    const verifyTodoCommand = vscode.commands.registerCommand('sniff.verifyTodo', async () => {
        const todoId = await vscode.window.showInputBox({
            prompt: 'Enter TODO ID to verify'
        });
        
        if (todoId) {
            const todo = todoManager.getTodo(todoId);
            if (todo) {
                const terminal = vscode.window.createTerminal('Sniff Verification');
                terminal.sendText(`sniff verify-todo --todo-id "${todoId}" --files ${todo.files.join(' ')} --min-quality-score ${todo.minQualityScore}`);
                terminal.show();
            }
        }
    });
    
    context.subscriptions.push(createTodoCommand, completeTodoCommand, verifyTodoCommand);
    
    // Register task provider
    const taskProvider = new SniffTaskProvider();
    context.subscriptions.push(vscode.tasks.registerTaskProvider('sniff-verify', taskProvider));
}
```

### 5. Package.json Configuration

```json
{
    "name": "sniff-vscode",
    "displayName": "Sniff Quality Gate",
    "description": "Integrate sniff verification into VS Code TODO workflows",
    "version": "1.0.0",
    "engines": {
        "vscode": "^1.70.0"
    },
    "categories": ["Other"],
    "activationEvents": [
        "onCommand:sniff.createTodo"
    ],
    "contributes": {
        "commands": [
            {
                "command": "sniff.createTodo",
                "title": "Create TODO with Sniff Verification",
                "category": "Sniff"
            },
            {
                "command": "sniff.completeTodo", 
                "title": "Complete TODO (with verification)",
                "category": "Sniff"
            },
            {
                "command": "sniff.verifyTodo",
                "title": "Verify TODO Quality",
                "category": "Sniff"
            }
        ],
        "taskDefinitions": [
            {
                "type": "sniff-verify",
                "required": ["todoId"],
                "properties": {
                    "todoId": {
                        "type": "string",
                        "description": "TODO ID to verify"
                    }
                }
            }
        ]
    }
}
```

## Usage Workflow

### 1. Create TODO
```
Command Palette → "Sniff: Create TODO with Sniff Verification"
→ Enter description: "Implement user authentication"
→ Enter files: "src/auth.ts, src/middleware/auth.ts"
→ TODO created with ID: auth-001
```

### 2. Work on Implementation
```
// Developer implements the feature
// Makes changes to src/auth.ts and src/middleware/auth.ts
```

### 3. Verify Before Completion
```
Command Palette → "Sniff: Verify TODO Quality"
→ Enter TODO ID: auth-001
→ Sniff runs analysis on specified files
→ Shows verification results in terminal
```

### 4. Complete TODO (Only if Verification Passes)
```
Command Palette → "Sniff: Complete TODO (with verification)"
→ Enter TODO ID: auth-001
→ Automatic verification runs
→ If passed: TODO marked complete
→ If failed: Shows issues to fix
```

## Benefits

1. **Native Integration**: Uses VS Code's built-in task system
2. **Quality Gates**: Prevents TODO completion without quality verification
3. **Visual Feedback**: Shows verification results in Problems panel
4. **Configurable**: Adjustable quality thresholds per TODO
5. **Terminal Integration**: Can run sniff commands directly in terminal

## Technical Notes

- **Problem Matcher**: Custom problem matcher parses sniff output for VS Code Problems panel
- **Task Execution**: Uses VS Code's shell execution for cross-platform compatibility  
- **Extension API**: Leverages VS Code Extension API for commands and UI
- **File Tracking**: Associates specific files with each TODO for targeted analysis

This integration provides a robust, native VS Code experience for quality-gated TODO workflows using sniff verification.
