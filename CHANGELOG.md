# Changelog

All notable changes to the Sniff project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- TODO verification with quality gates (`verify-todo` command)
- Git discovery for preventing AI agent deception during verification
- Comprehensive CI/CD pipeline with GitHub Actions
- Automated binary releases for multiple platforms
- Homebrew formula and tap integration
- Code formatting and quality check scripts
- Cross-platform release automation
- Multi-platform editor integrations (VS Code, Cursor, Gemini CLI)
- Checkpoint system for tracking code changes over time

### Changed
- Improved release workflow with automated testing
- Enhanced standalone file analysis independent of Claude Code
- Streamlined CLI interface with focus on quality verification

### Removed
- Claude Code specific session management (tree.rs, session.rs, storage.rs)
- Merkle tree storage system (hash.rs, removed blake3/redb dependencies)
- Session indexing and search commands (search.rs, operations.rs)
- Legacy JSONL parsing (jsonl.rs)
- Progress indicators and file watchers (progress.rs, watcher.rs)
- Complex pattern learning system APIs (simplified)
- Database maintenance commands

### Deprecated
- Legacy types.rs (partially cleaned, can be further reduced)

## [0.1.0] - 2024-XX-XX

### Added
- AI misalignment pattern detection engine
- Multi-language support (Rust, Python, TypeScript)
- Claude Code session integration and monitoring
- Standalone file analysis capabilities
- Extensible pattern library system with YAML-based rules
- TreeSitter-based syntax parsing for accurate code analysis
- Merkle tree storage for efficient session management
- Real-time monitoring of AI code generation
- Community pattern learning and contribution system
- Comprehensive playbook system for custom detection rules

### Features
- **Core Detection Engine**: MisalignmentAnalyzer with pattern matching
- **Multi-Language Support**: 
  - Rust: 15+ deception patterns including premature returns, fake authentication, silent errors
  - Python: 15+ patterns covering placeholder implementations, mock data, generic errors
  - TypeScript: 14+ patterns for interface misuse, async shortcuts, type assertions
- **Session Management**: Integration with Claude Code sessions via Merkle tree storage
- **Standalone Analysis**: File-by-file analysis independent of Claude sessions
- **Pattern Learning**: Community-driven pattern discovery and sharing
- **Real-time Monitoring**: Live detection during AI code generation sessions

### Technical Details
- **Architecture**: Rust-based CLI tool with TreeSitter parsing
- **Storage**: Efficient Merkle tree structure for session data
- **Extensibility**: YAML-based pattern definitions for easy customization
- **Performance**: Optimized for large codebases with concurrent analysis
- **Integration**: Designed for seamless Claude Code workflow integration

### Documentation
- Comprehensive README with usage examples
- Pattern library documentation
- API documentation for programmatic usage
- Integration guides for various editors and workflows

### Security
- Safe pattern matching without code execution
- Secure session data handling
- Privacy-focused analysis (no code sent to external services)

---

## Release Types

- **Major (X.0.0)**: Breaking changes, major new features, architecture changes
- **Minor (0.X.0)**: New features, improvements, backwards-compatible changes  
- **Patch (0.0.X)**: Bug fixes, documentation updates, minor improvements

## Contributing

When contributing changes, please:

1. Add an entry to the `[Unreleased]` section
2. Use the appropriate category: Added, Changed, Deprecated, Removed, Fixed, Security
3. Include brief, user-focused descriptions
4. Reference issue numbers where applicable

Example:
```markdown
### Added
- New pattern detection for async/await misuse (#42)

### Fixed  
- File extension detection for temporary files (#38)
- Session timeout handling in monitor mode (#45)
```

## Links

- [Repository](https://github.com/conikeec/sniff)
- [Issues](https://github.com/conikeec/sniff/issues)
- [Releases](https://github.com/conikeec/sniff/releases)