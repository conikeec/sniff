// Copyright (c) 2025 Chetan Conikee <conikee@gmail.com>
// Licensed under the MIT License

//! Playbook system for defining and managing bullshit detection patterns.

use crate::analysis::SupportedLanguage;
use crate::error::{SniffError, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Severity level for detected bullshit patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Severity {
    /// Low severity issues - minor code quality concerns
    Low,
    /// Medium severity issues - moderate code quality problems
    Medium,
    /// High severity issues - significant code quality problems
    High,
    /// Critical severity issues - serious problems that need immediate attention
    Critical,
}

impl Severity {
    /// Gets the numeric score for this severity level.
    pub fn score(&self) -> u8 {
        match self {
            Severity::Low => 1,
            Severity::Medium => 2,
            Severity::High => 3,
            Severity::Critical => 4,
        }
    }
    
    /// Gets the emoji representation for this severity level.
    pub fn emoji(&self) -> &'static str {
        match self {
            Severity::Low => "🟢",
            Severity::Medium => "🟡",
            Severity::High => "🔴",
            Severity::Critical => "🚨",
        }
    }
    
    /// Gets the string name for this severity level.
    pub fn name(&self) -> &'static str {
        match self {
            Severity::Low => "Low",
            Severity::Medium => "Medium",
            Severity::High => "High",
            Severity::Critical => "Critical",
        }
    }
}

/// Scope where a pattern should be applied.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PatternScope {
    /// Apply to entire file.
    File,
    /// Apply only within function bodies.
    FunctionBody,
    /// Apply only within class bodies.
    ClassBody,
    /// Apply only within comments.
    Comments,
    /// Apply only within method signatures.
    MethodSignature,
}

/// Type of pattern matching to perform.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatternType {
    /// Regular expression pattern.
    Regex {
        /// The regex pattern string
        pattern: String,
        /// Optional regex flags (i, m, s, etc.)
        flags: Option<String>,
    },
    /// TreeSitter AST query pattern.
    AstQuery {
        /// The TreeSitter query string
        query: String,
    },
    /// Structural analysis pattern.
    Structural {
        /// Type of structural analysis to perform
        analysis_type: String,
        /// Additional parameters for the analysis
        parameters: HashMap<String, String>,
    },
}

/// A single detection rule within a playbook.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DetectionRule {
    /// Unique identifier for this rule.
    pub id: String,
    /// Human-readable name for this rule.
    pub name: String,
    /// Detailed description of what this rule detects.
    pub description: String,
    /// Severity level of this detection.
    pub severity: Severity,
    /// Type of pattern matching to perform.
    pub pattern_type: PatternType,
    /// Scope where this rule should be applied.
    pub scope: PatternScope,
    /// Whether this rule is enabled.
    pub enabled: bool,
    /// Tags for categorizing this rule.
    pub tags: Vec<String>,
    /// Examples of code that triggers this rule.
    pub examples: Vec<String>,
    /// False positive examples that should NOT trigger this rule.
    pub false_positives: Vec<String>,
}

/// A collection of detection rules for a specific language.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Playbook {
    /// Name of this playbook.
    pub name: String,
    /// Version of this playbook.
    pub version: String,
    /// Programming language this playbook targets.
    pub language: String,
    /// Author of this playbook.
    pub author: String,
    /// Description of this playbook.
    pub description: String,
    /// Detection rules in this playbook.
    pub rules: Vec<DetectionRule>,
    /// Metadata about this playbook.
    pub metadata: HashMap<String, String>,
}

/// Manages loading and organizing playbooks.
pub struct PlaybookManager {
    /// Loaded playbooks organized by language.
    playbooks: HashMap<SupportedLanguage, Vec<Playbook>>,
    /// Compiled regex patterns for performance.
    compiled_patterns: HashMap<String, Regex>,
}

impl PlaybookManager {
    /// Creates a new playbook manager.
    pub fn new() -> Self {
        Self {
            playbooks: HashMap::new(),
            compiled_patterns: HashMap::new(),
        }
    }
    
    /// Loads a playbook from a YAML file.
    pub fn load_playbook(&mut self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            SniffError::file_system(path, e)
        })?;
        
        let playbook: Playbook = serde_yaml::from_str(&content).map_err(|e| {
            SniffError::invalid_format(
                "playbook parsing".to_string(),
                format!("Failed to parse playbook YAML: {}", e),
            )
        })?;
        
        // Validate the playbook
        self.validate_playbook(&playbook)?;
        
        // Pre-compile regex patterns for performance
        for rule in &playbook.rules {
            if let PatternType::Regex { pattern, .. } = &rule.pattern_type {
                if !self.compiled_patterns.contains_key(&rule.id) {
                    let regex = Regex::new(pattern).map_err(|e| {
                        SniffError::invalid_format(
                            "regex pattern".to_string(),
                            format!("Invalid regex in rule '{}': {}", rule.id, e),
                        )
                    })?;
                    self.compiled_patterns.insert(rule.id.clone(), regex);
                }
            }
        }
        
        // Convert language name to SupportedLanguage
        let supported_language = match playbook.language.as_str() {
            "rust" => SupportedLanguage::Rust,
            "python" => SupportedLanguage::Python,
            "javascript" => SupportedLanguage::JavaScript,
            "typescript" => SupportedLanguage::TypeScript,
            "go" => SupportedLanguage::Go,
            "c" => SupportedLanguage::C,
            "cpp" => SupportedLanguage::Cpp,
            _ => return Err(SniffError::invalid_format(
                "unsupported language".to_string(),
                format!("Unsupported language: {}", playbook.language),
            )),
        };
        
        // Add to playbooks
        self.playbooks
            .entry(supported_language)
            .or_insert_with(Vec::new)
            .push(playbook);
        
        Ok(())
    }
    
    /// Loads all playbooks from a directory.
    pub fn load_playbooks_from_dir(&mut self, dir: &Path) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }
        
        let entries = std::fs::read_dir(dir).map_err(|e| {
            SniffError::file_system(dir, e)
        })?;
        
        for entry in entries {
            let entry = entry.map_err(|e| {
                SniffError::file_system(dir, e)
            })?;
            
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("yaml") ||
               path.extension().and_then(|s| s.to_str()) == Some("yml") {
                if let Err(e) = self.load_playbook(&path) {
                    eprintln!("Warning: Failed to load playbook {}: {}", path.display(), e);
                }
            }
        }
        
        Ok(())
    }
    
    /// Gets all playbooks for a specific language.
    pub fn get_playbooks_for_language(&self, language: SupportedLanguage) -> Vec<&Playbook> {
        self.playbooks
            .get(&language)
            .map(|playbooks| playbooks.iter().collect())
            .unwrap_or_default()
    }
    
    /// Gets all playbooks for a specific language by name.
    pub fn get_playbooks_for_language_name(&self, language_name: &str) -> Vec<&Playbook> {
        self.playbooks
            .values()
            .flat_map(|playbooks| playbooks.iter())
            .filter(|playbook| playbook.language == language_name)
            .collect()
    }
    
    /// Gets all active rules for a specific language.
    pub fn get_active_rules_for_language(&self, language: SupportedLanguage) -> Vec<&DetectionRule> {
        self.get_playbooks_for_language(language)
            .iter()
            .flat_map(|playbook| playbook.rules.iter())
            .filter(|rule| rule.enabled)
            .collect()
    }
    
    /// Gets a compiled regex pattern for a rule.
    pub fn get_compiled_pattern(&self, rule_id: &str) -> Option<&Regex> {
        self.compiled_patterns.get(rule_id)
    }

    /// Adds a playbook directly to the manager.
    pub fn add_playbook(&mut self, language: SupportedLanguage, playbook: Playbook) {
        self.playbooks
            .entry(language)
            .or_insert_with(Vec::new)
            .push(playbook);
    }
    
    /// Validates a playbook for correctness.
    fn validate_playbook(&self, playbook: &Playbook) -> Result<()> {
        // Check for duplicate rule IDs
        let mut rule_ids = std::collections::HashSet::new();
        for rule in &playbook.rules {
            if !rule_ids.insert(&rule.id) {
                return Err(SniffError::invalid_format(
                    "playbook validation".to_string(),
                    format!("Duplicate rule ID '{}' in playbook '{}'", rule.id, playbook.name),
                ));
            }
        }
        
        // Validate regex patterns
        for rule in &playbook.rules {
            if let PatternType::Regex { pattern, .. } = &rule.pattern_type {
                Regex::new(pattern).map_err(|e| {
                    SniffError::invalid_format(
                        "regex validation".to_string(),
                        format!("Invalid regex in rule '{}': {}", rule.id, e),
                    )
                })?;
            }
        }
        
        Ok(())
    }
    
    /// Creates a default playbook for a language.
    pub fn create_default_playbook(language: SupportedLanguage) -> Playbook {
        let rules = match language {
            SupportedLanguage::Rust => Self::create_rust_default_rules(),
            SupportedLanguage::Python => Self::create_python_default_rules(),
            SupportedLanguage::JavaScript => Self::create_javascript_default_rules(),
            SupportedLanguage::TypeScript => Self::create_typescript_default_rules(),
            SupportedLanguage::Go => Self::create_go_default_rules(),
            SupportedLanguage::C => Self::create_c_default_rules(),
            SupportedLanguage::Cpp => Self::create_cpp_default_rules(),
        };
        
        Playbook {
            name: format!("{} Default Patterns", language.name()),
            version: "1.0.0".to_string(),
            language: language.name().to_string(),
            author: "Sniff Core Team".to_string(),
            description: format!("Default bullshit detection patterns for {}", language.name()),
            rules,
            metadata: HashMap::new(),
        }
    }
    
    /// Creates default Rust detection rules.
    fn create_rust_default_rules() -> Vec<DetectionRule> {
        vec![
            DetectionRule {
                id: "rust_unimplemented_macro".to_string(),
                name: "Unimplemented Macro".to_string(),
                description: "Function uses unimplemented!() macro".to_string(),
                severity: Severity::Critical,
                pattern_type: PatternType::Regex {
                    pattern: r"unimplemented!\(\)".to_string(),
                    flags: None,
                },
                scope: PatternScope::FunctionBody,
                enabled: true,
                tags: vec!["placeholder".to_string(), "incomplete".to_string()],
                examples: vec![
                    "fn do_something() { unimplemented!() }".to_string(),
                ],
                false_positives: vec![],
            },
            DetectionRule {
                id: "rust_todo_comment".to_string(),
                name: "TODO Comment".to_string(),
                description: "TODO, FIXME, or XXX comment in code".to_string(),
                severity: Severity::Medium,
                pattern_type: PatternType::Regex {
                    pattern: r"(?i)//\s*(TODO|FIXME|XXX|HACK):".to_string(),
                    flags: Some("i".to_string()),
                },
                scope: PatternScope::Comments,
                enabled: true,
                tags: vec!["todo".to_string(), "incomplete".to_string()],
                examples: vec![
                    "// TODO: implement this".to_string(),
                    "// FIXME: handle errors".to_string(),
                ],
                false_positives: vec![],
            },
            DetectionRule {
                id: "rust_panic_with_todo".to_string(),
                name: "Panic with TODO".to_string(),
                description: "Function panics with TODO-related message".to_string(),
                severity: Severity::High,
                pattern_type: PatternType::Regex {
                    pattern: r#"panic!\s*\(\s*"[^"]*(?:TODO|FIXME|XXX|placeholder|not implemented)[^"]*"\s*\)"#.to_string(),
                    flags: Some("i".to_string()),
                },
                scope: PatternScope::FunctionBody,
                enabled: true,
                tags: vec!["panic".to_string(), "placeholder".to_string()],
                examples: vec![
                    r#"panic!("TODO: implement this")"#.to_string(),
                ],
                false_positives: vec![],
            },
            DetectionRule {
                id: "rust_unwrap_without_context".to_string(),
                name: "Unwrap Without Context".to_string(),
                description: "Using unwrap() without proper error handling".to_string(),
                severity: Severity::Medium,
                pattern_type: PatternType::Regex {
                    pattern: r"\.unwrap\(\)".to_string(),
                    flags: None,
                },
                scope: PatternScope::FunctionBody,
                enabled: true,
                tags: vec!["error_handling".to_string(), "unwrap".to_string()],
                examples: vec![
                    "let value = result.unwrap();".to_string(),
                ],
                false_positives: vec![
                    "let value = result.unwrap(); // Safe: checked above".to_string(),
                ],
            },
        ]
    }
    
    /// Creates default Python detection rules.
    fn create_python_default_rules() -> Vec<DetectionRule> {
        vec![
            DetectionRule {
                id: "python_pass_only_function".to_string(),
                name: "Pass-Only Function".to_string(),
                description: "Function contains only 'pass' statement".to_string(),
                severity: Severity::High,
                pattern_type: PatternType::Regex {
                    pattern: r"def\s+\w+\([^)]*\):\s*pass".to_string(),
                    flags: Some("m".to_string()),
                },
                scope: PatternScope::FunctionBody,
                enabled: true,
                tags: vec!["placeholder".to_string(), "incomplete".to_string()],
                examples: vec![
                    "def do_something():\n    pass".to_string(),
                ],
                false_positives: vec![],
            },
            DetectionRule {
                id: "python_not_implemented_error".to_string(),
                name: "NotImplementedError".to_string(),
                description: "Function raises NotImplementedError".to_string(),
                severity: Severity::Critical,
                pattern_type: PatternType::Regex {
                    pattern: r"raise\s+NotImplementedError".to_string(),
                    flags: None,
                },
                scope: PatternScope::FunctionBody,
                enabled: true,
                tags: vec!["placeholder".to_string(), "incomplete".to_string()],
                examples: vec![
                    "raise NotImplementedError()".to_string(),
                ],
                false_positives: vec![],
            },
            DetectionRule {
                id: "python_todo_comment".to_string(),
                name: "TODO Comment".to_string(),
                description: "TODO, FIXME, or XXX comment in code".to_string(),
                severity: Severity::Medium,
                pattern_type: PatternType::Regex {
                    pattern: r"(?i)#\s*(TODO|FIXME|XXX|HACK):".to_string(),
                    flags: Some("i".to_string()),
                },
                scope: PatternScope::Comments,
                enabled: true,
                tags: vec!["todo".to_string(), "incomplete".to_string()],
                examples: vec![
                    "# TODO: implement this".to_string(),
                    "# FIXME: handle errors".to_string(),
                ],
                false_positives: vec![],
            },
        ]
    }
    
    /// Placeholder for other language default rules.
    fn create_javascript_default_rules() -> Vec<DetectionRule> {
        vec![
            DetectionRule {
                id: "js_empty_function".to_string(),
                name: "Empty Function".to_string(),
                description: "Function has empty body".to_string(),
                severity: Severity::High,
                pattern_type: PatternType::Regex {
                    pattern: r"function\s+\w+\s*\([^)]*\)\s*\{\s*\}".to_string(),
                    flags: None,
                },
                scope: PatternScope::FunctionBody,
                enabled: true,
                tags: vec!["placeholder".to_string(), "incomplete".to_string()],
                examples: vec![
                    "function doSomething() {}".to_string(),
                ],
                false_positives: vec![],
            },
        ]
    }
    
    fn create_typescript_default_rules() -> Vec<DetectionRule> { Self::create_javascript_default_rules() }
    fn create_go_default_rules() -> Vec<DetectionRule> { vec![] }
    fn create_c_default_rules() -> Vec<DetectionRule> { vec![] }
    fn create_cpp_default_rules() -> Vec<DetectionRule> { vec![] }
}

impl Default for PlaybookManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_playbook_creation() {
        let playbook = PlaybookManager::create_default_playbook(SupportedLanguage::Rust);
        assert_eq!(playbook.language, "rust");
        assert!(!playbook.rules.is_empty());
    }
    
    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical.score() > Severity::High.score());
        assert!(Severity::High.score() > Severity::Medium.score());
        assert!(Severity::Medium.score() > Severity::Low.score());
    }
    
    #[test]
    fn test_playbook_manager() {
        let mut manager = PlaybookManager::new();
        let playbook = PlaybookManager::create_default_playbook(SupportedLanguage::Rust);
        
        // Simulate loading a playbook
        manager.playbooks.insert(SupportedLanguage::Rust, vec![playbook]);
        
        let rules = manager.get_active_rules_for_language(SupportedLanguage::Rust);
        assert!(!rules.is_empty());
    }
}