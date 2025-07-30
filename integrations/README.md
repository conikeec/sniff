# Sniff Integrations

This directory contains integration guides for incorporating sniff verification into various AI coding platforms and editors.

## Overview

Sniff provides quality gate verification that prevents completion of TODOs unless code meets specified quality standards. Each integration follows the core pattern:

1. **Task Creation**: TODO/task is created with associated files
2. **Implementation**: Code changes are made
3. **Verification**: Sniff analyzes modified files for quality issues
4. **Gate Check**: Only pass if quality thresholds are met
5. **Completion**: Mark task complete only after verification passes

## Available Integrations

- [AmpCode Integration](./ampcode.md) - Sourcegraph's AI coding platform
- [Claude Code Integration](./claude-code.md) - Anthropic's Claude in coding environments
- [Gemini CLI Integration](./gemini-cli.md) - Google's Gemini command line interface
- [VS Code Integration](./vscode.md) - Visual Studio Code with AI extensions
- [Cursor Integration](./cursor.md) - AI-powered code editor

## Common Requirements

All integrations require:

- Sniff binary installed and accessible in PATH
- Project-specific quality thresholds configured
- File tracking for TODO items
- Integration with the platform's task/TODO system

## Integration Status

| Platform                        | Status          | Method                     | Complexity | Notes                             |
| ------------------------------- | --------------- | -------------------------- | ---------- | --------------------------------- |
| [VS Code](./vscode.md)          | ✅ **Feasible** | Extension + Tasks API      | Medium     | Well-documented extension API     |
| [Cursor](./cursor.md)           | ✅ **Feasible** | MCP Server + Extension     | Medium     | VS Code-based + MCP support       |
| [Gemini CLI](./gemini-cli.md)   | ✅ **Feasible** | Workflow + CLI Tools       | Low        | Open-source with tool integration |
| [Claude Code](./claude-code.md) | ✅ **Feasible** | MCP Server + Session Files | High       | MCP support + session management  |
| [AmpCode](./ampcode.md)         | 🔍 **Research** | CLI Wrapper + Extension    | Unknown    | Need API documentation            |

### Implementation Priority

1. **VS Code** - Most straightforward with mature APIs
2. **Gemini CLI** - Simple CLI-based integration
3. **Cursor** - VS Code foundation + AI enhancements
4. **Claude Code** - Complex but powerful session integration
5. **AmpCode** - Pending API research and documentation

### Research Status

✅ **Completed Research**: VS Code, Cursor, Gemini CLI, Claude Code  
🔍 **Requires Research**: AmpCode extension APIs and integration points
