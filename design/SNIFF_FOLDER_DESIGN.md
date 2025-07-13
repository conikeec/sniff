# .sniff Folder Structure Design

## Overview
The `.sniff` folder serves as a local workspace for pattern learning, analysis storage, and pattern management during LLM generative loops.

## Folder Structure
```
.sniff/
├── patterns/                    # Learned patterns (separate from baseline)
│   ├── rust/
│   │   ├── learned-patterns.yaml
│   │   └── pattern-metadata.json
│   ├── python/
│   │   ├── learned-patterns.yaml
│   │   └── pattern-metadata.json
│   ├── typescript/
│   │   ├── learned-patterns.yaml
│   │   └── pattern-metadata.json
│   └── javascript/
│       ├── learned-patterns.yaml
│       └── pattern-metadata.json
├── analysis/                    # Analysis results and session data
│   ├── sessions/
│   │   └── {session-id}/
│   │       ├── analysis.json
│   │       ├── patterns-detected.json
│   │       └── learned-patterns.json
│   └── reports/
│       ├── daily/
│       ├── weekly/
│       └── pattern-evolution.json
├── database/                    # ReDB storage
│   ├── patterns.redb           # Pattern storage and metadata
│   ├── analysis.redb           # Analysis results
│   └── learning.redb           # Learning history and statistics
├── config/                     # Configuration
│   ├── learning-config.yaml    # Learning parameters and thresholds
│   ├── pattern-rules.yaml      # Pattern creation rules and validation
│   └── api-config.yaml         # API configuration for pattern creation
└── logs/                       # Logging
    ├── pattern-creation.log
    ├── analysis.log
    └── api.log
```

## Key Components

### 1. Pattern Storage (`patterns/`)
- **Language-specific**: Each language gets its own subdirectory
- **Learned patterns**: Separate from baseline playbooks
- **Metadata tracking**: Pattern creation date, confidence, usage statistics
- **YAML format**: Compatible with existing playbook system

### 2. Analysis Storage (`analysis/`)
- **Session-based**: Each Claude Code session gets analysis results
- **Pattern detection history**: Track what patterns were found when
- **Learning opportunities**: Identify new patterns from analysis

### 3. Database Storage (`database/`)
- **ReDB integration**: Fast, embedded database storage
- **Pattern persistence**: Survive between sessions
- **Learning history**: Track pattern evolution and effectiveness

### 4. Configuration (`config/`)
- **Learning parameters**: Confidence thresholds, pattern validation rules
- **API configuration**: Settings for pattern creation API
- **Customization**: Allow users to configure learning behavior

## Pattern Learning Workflow

1. **Detection**: Analyze code with existing patterns
2. **Learning**: Identify potential new patterns from analysis
3. **Validation**: Validate pattern quality and uniqueness
4. **Storage**: Store learned pattern in language-specific files
5. **Integration**: Automatically load learned patterns in future analysis
6. **Evolution**: Track pattern effectiveness and refine over time

## API Integration Points

- **Pattern Creation**: `POST /api/patterns/{language}`
- **Pattern Validation**: `POST /api/patterns/validate`
- **Pattern Statistics**: `GET /api/patterns/{language}/stats`
- **Learning History**: `GET /api/learning/history`
- **Pattern Export**: `GET /api/patterns/{language}/export`