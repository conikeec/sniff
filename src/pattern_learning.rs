// Copyright (c) 2025 Chetan Conikee <conikee@gmail.com>
// Licensed under the MIT License

//! Pattern learning system for dynamic pattern creation during LLM generative loops.

use crate::analysis::SupportedLanguage;
use crate::error::{Result, SniffError};
use crate::playbook::{DetectionRule, PatternScope, PatternType, Playbook, Severity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Configuration for pattern learning system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfig {
    /// Minimum confidence threshold for pattern acceptance (0.0-1.0)
    pub min_confidence: f64,
    /// Maximum number of learned patterns per language
    pub max_patterns_per_language: usize,
    /// Enable automatic pattern validation
    pub auto_validate: bool,
    /// Pattern expiration time in days (0 = never expire)
    pub pattern_expiry_days: u32,
    /// Learning rate adjustment factor
    pub learning_rate: f64,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.75,
            max_patterns_per_language: 100,
            auto_validate: true,
            pattern_expiry_days: 30,
            learning_rate: 1.0,
        }
    }
}

/// Metadata for a learned pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMetadata {
    /// Unique identifier for this pattern
    pub id: String,
    /// When this pattern was created
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last time this pattern was updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
    /// Confidence score when pattern was created (0.0-1.0)
    pub confidence: f64,
    /// Number of times this pattern has been detected
    pub detection_count: u64,
    /// Number of false positives reported
    pub false_positive_count: u64,
    /// Source of pattern creation (e.g., "claude-code", "manual", "ai-analysis")
    pub source: String,
    /// Language this pattern applies to
    pub language: SupportedLanguage,
    /// Whether this pattern is currently active
    pub active: bool,
    /// User-provided tags for categorization
    pub tags: Vec<String>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// A learned pattern with its metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearnedPattern {
    /// The detection rule
    pub rule: DetectionRule,
    /// Metadata about this learned pattern
    pub metadata: PatternMetadata,
}

/// Request to create a new pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternCreationRequest {
    /// Name for the new pattern
    pub name: String,
    /// Description of what this pattern detects
    pub description: String,
    /// Severity level for detections
    pub severity: Severity,
    /// The regex pattern to match
    pub pattern: String,
    /// Optional regex flags
    pub flags: Option<String>,
    /// Scope where this pattern should be applied
    pub scope: PatternScope,
    /// Language this pattern applies to
    pub language: SupportedLanguage,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Example code that should trigger this pattern
    pub examples: Vec<String>,
    /// Example code that should NOT trigger this pattern
    pub false_positives: Vec<String>,
    /// Confidence in this pattern (0.0-1.0)
    pub confidence: f64,
    /// Source of pattern creation
    pub source: String,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Response from pattern creation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternCreationResponse {
    /// Whether the pattern was created successfully
    pub success: bool,
    /// The ID of the created pattern (if successful)
    pub pattern_id: Option<String>,
    /// Error message (if not successful)
    pub error: Option<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Path where the pattern was stored
    pub storage_path: Option<PathBuf>,
}

/// Statistics about learned patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternStatistics {
    /// Total number of learned patterns
    pub total_patterns: usize,
    /// Patterns by language
    pub patterns_by_language: HashMap<SupportedLanguage, usize>,
    /// Patterns by severity
    pub patterns_by_severity: HashMap<Severity, usize>,
    /// Average confidence score
    pub average_confidence: f64,
    /// Total detections across all patterns
    pub total_detections: u64,
    /// Pattern creation rate (patterns per day)
    pub creation_rate: f64,
    /// Most active patterns (by detection count)
    pub most_active_patterns: Vec<(String, u64)>,
}

/// Manages pattern learning and storage in the .sniff folder.
pub struct PatternLearningManager {
    /// Base path to .sniff folder
    sniff_path: PathBuf,
    /// Learning configuration
    config: LearningConfig,
    /// Cache of loaded learned patterns by language
    learned_patterns: HashMap<SupportedLanguage, Vec<LearnedPattern>>,
}

impl PatternLearningManager {
    /// Creates a new pattern learning manager.
    pub fn new<P: AsRef<Path>>(base_path: P) -> Result<Self> {
        let sniff_path = base_path.as_ref().join(".sniff");
        
        let mut manager = Self {
            sniff_path,
            config: LearningConfig::default(),
            learned_patterns: HashMap::new(),
        };

        // Initialize .sniff folder structure
        manager.initialize_folder_structure()?;
        
        // Load configuration
        manager.load_config()?;
        
        // Load existing learned patterns
        manager.load_learned_patterns()?;

        Ok(manager)
    }

    /// Initializes the .sniff folder structure.
    fn initialize_folder_structure(&self) -> Result<()> {
        let folders = [
            "patterns",
            "patterns/rust",
            "patterns/python", 
            "patterns/typescript",
            "patterns/javascript",
            "patterns/go",
            "patterns/c",
            "patterns/cpp",
            "analysis",
            "analysis/sessions",
            "analysis/reports",
            "analysis/reports/daily",
            "analysis/reports/weekly",
            "database",
            "config",
            "logs",
        ];

        for folder in &folders {
            let path = self.sniff_path.join(folder);
            if !path.exists() {
                std::fs::create_dir_all(&path)
                    .map_err(|e| SniffError::file_system(&path, e))?;
            }
        }

        Ok(())
    }

    /// Loads learning configuration.
    fn load_config(&mut self) -> Result<()> {
        let config_path = self.sniff_path.join("config").join("learning-config.yaml");
        
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)
                .map_err(|e| SniffError::file_system(&config_path, e))?;
            
            self.config = serde_yaml::from_str(&content).map_err(|e| {
                SniffError::invalid_format(
                    "learning config".to_string(),
                    format!("Failed to parse learning config: {e}"),
                )
            })?;
        } else {
            // Create default config
            self.save_config()?;
        }

        Ok(())
    }

    /// Saves learning configuration.
    fn save_config(&self) -> Result<()> {
        let config_path = self.sniff_path.join("config").join("learning-config.yaml");
        
        let content = serde_yaml::to_string(&self.config).map_err(|e| {
            SniffError::invalid_format(
                "learning config serialization".to_string(),
                format!("Failed to serialize learning config: {e}"),
            )
        })?;

        std::fs::write(&config_path, content)
            .map_err(|e| SniffError::file_system(&config_path, e))?;

        Ok(())
    }

    /// Loads all learned patterns from storage.
    fn load_learned_patterns(&mut self) -> Result<()> {
        let languages = [
            SupportedLanguage::Rust,
            SupportedLanguage::Python,
            SupportedLanguage::TypeScript,
            SupportedLanguage::JavaScript,
            SupportedLanguage::Go,
            SupportedLanguage::C,
            SupportedLanguage::Cpp,
        ];

        for language in &languages {
            let patterns = self.load_patterns_for_language(*language)?;
            self.learned_patterns.insert(*language, patterns);
        }

        Ok(())
    }

    /// Loads learned patterns for a specific language.
    fn load_patterns_for_language(&self, language: SupportedLanguage) -> Result<Vec<LearnedPattern>> {
        let patterns_path = self.sniff_path
            .join("patterns")
            .join(language.name())
            .join("learned-patterns.yaml");

        if !patterns_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&patterns_path)
            .map_err(|e| SniffError::file_system(&patterns_path, e))?;

        let patterns: Vec<LearnedPattern> = serde_yaml::from_str(&content).map_err(|e| {
            SniffError::invalid_format(
                "learned patterns".to_string(),
                format!("Failed to parse learned patterns for {}: {e}", language.name()),
            )
        })?;

        Ok(patterns)
    }

    /// Creates a new learned pattern.
    pub fn create_pattern(&mut self, request: PatternCreationRequest) -> Result<PatternCreationResponse> {
        // Validate request
        let warnings = self.validate_pattern_request(&request)?;
        
        if request.confidence < self.config.min_confidence {
            return Ok(PatternCreationResponse {
                success: false,
                pattern_id: None,
                error: Some(format!(
                    "Pattern confidence {} below minimum threshold {}",
                    request.confidence, self.config.min_confidence
                )),
                warnings,
                storage_path: None,
            });
        }

        // Check if we're at the pattern limit for this language
        let current_count = self.learned_patterns
            .get(&request.language)
            .map(|patterns| patterns.len())
            .unwrap_or(0);

        if current_count >= self.config.max_patterns_per_language {
            return Ok(PatternCreationResponse {
                success: false,
                pattern_id: None,
                error: Some(format!(
                    "Maximum patterns reached for {} ({})",
                    request.language.name(), self.config.max_patterns_per_language
                )),
                warnings,
                storage_path: None,
            });
        }

        // Create pattern ID
        let pattern_id = format!("learned_{}", Uuid::new_v4().to_string().replace('-', "_"));
        
        // Create detection rule
        let rule = DetectionRule {
            id: pattern_id.clone(),
            name: request.name,
            description: request.description,
            severity: request.severity,
            pattern_type: PatternType::Regex {
                pattern: request.pattern,
                flags: request.flags,
            },
            scope: request.scope,
            enabled: true,
            tags: request.tags.clone(),
            examples: request.examples,
            false_positives: request.false_positives,
        };

        // Create metadata
        let now = chrono::Utc::now();
        let metadata = PatternMetadata {
            id: pattern_id.clone(),
            created_at: now,
            updated_at: now,
            confidence: request.confidence,
            detection_count: 0,
            false_positive_count: 0,
            source: request.source,
            language: request.language,
            active: true,
            tags: request.tags,
            metadata: request.metadata,
        };

        // Create learned pattern
        let learned_pattern = LearnedPattern { rule, metadata };

        // Add to cache
        self.learned_patterns
            .entry(request.language)
            .or_default()
            .push(learned_pattern.clone());

        // Save to storage
        let storage_path = self.save_patterns_for_language(request.language)?;

        Ok(PatternCreationResponse {
            success: true,
            pattern_id: Some(pattern_id),
            error: None,
            warnings,
            storage_path: Some(storage_path),
        })
    }

    /// Validates a pattern creation request.
    fn validate_pattern_request(&self, request: &PatternCreationRequest) -> Result<Vec<String>> {
        let mut warnings = Vec::new();

        // Validate regex pattern
        if let Err(e) = regex::Regex::new(&request.pattern) {
            return Err(SniffError::invalid_format(
                "pattern validation".to_string(),
                format!("Invalid regex pattern: {e}"),
            ));
        }

        // Check for duplicate patterns
        if let Some(existing_patterns) = self.learned_patterns.get(&request.language) {
            for existing in existing_patterns {
                if let PatternType::Regex { pattern, .. } = &existing.rule.pattern_type {
                    if pattern == &request.pattern {
                        warnings.push("Similar pattern already exists".to_string());
                        break;
                    }
                }
            }
        }

        // Validate examples
        if request.examples.is_empty() {
            warnings.push("No examples provided - pattern may be difficult to validate".to_string());
        }

        Ok(warnings)
    }

    /// Saves learned patterns for a specific language.
    fn save_patterns_for_language(&self, language: SupportedLanguage) -> Result<PathBuf> {
        let patterns_path = self.sniff_path
            .join("patterns")
            .join(language.name())
            .join("learned-patterns.yaml");

        let patterns = self.learned_patterns
            .get(&language)
            .cloned()
            .unwrap_or_default();

        let content = serde_yaml::to_string(&patterns).map_err(|e| {
            SniffError::invalid_format(
                "pattern serialization".to_string(),
                format!("Failed to serialize patterns for {}: {e}", language.name()),
            )
        })?;

        std::fs::write(&patterns_path, content)
            .map_err(|e| SniffError::file_system(&patterns_path, e))?;

        Ok(patterns_path)
    }

    /// Gets learned patterns for a specific language.
    pub fn get_patterns_for_language(&self, language: SupportedLanguage) -> Vec<&LearnedPattern> {
        self.learned_patterns
            .get(&language)
            .map(|patterns| patterns.iter().collect())
            .unwrap_or_default()
    }

    /// Gets pattern statistics.
    pub fn get_statistics(&self) -> PatternStatistics {
        let mut total_patterns = 0;
        let mut patterns_by_language = HashMap::new();
        let mut patterns_by_severity = HashMap::new();
        let mut total_confidence = 0.0;
        let mut total_detections = 0;
        let mut most_active = Vec::new();

        for (language, patterns) in &self.learned_patterns {
            patterns_by_language.insert(*language, patterns.len());
            total_patterns += patterns.len();

            for pattern in patterns {
                // Count by severity
                *patterns_by_severity.entry(pattern.rule.severity).or_insert(0) += 1;
                
                // Sum confidence
                total_confidence += pattern.metadata.confidence;
                
                // Sum detections
                total_detections += pattern.metadata.detection_count;
                
                // Track active patterns
                most_active.push((
                    pattern.rule.name.clone(),
                    pattern.metadata.detection_count,
                ));
            }
        }

        // Sort most active patterns
        most_active.sort_by(|a, b| b.1.cmp(&a.1));
        most_active.truncate(10); // Top 10

        let average_confidence = if total_patterns > 0 {
            total_confidence / total_patterns as f64
        } else {
            0.0
        };

        PatternStatistics {
            total_patterns,
            patterns_by_language,
            patterns_by_severity,
            average_confidence,
            total_detections,
            creation_rate: 0.0, // TODO: Calculate from metadata
            most_active_patterns: most_active,
        }
    }

    /// Converts learned patterns to a playbook for a specific language.
    pub fn to_playbook(&self, language: SupportedLanguage) -> Option<Playbook> {
        let patterns = self.learned_patterns.get(&language)?;
        
        if patterns.is_empty() {
            return None;
        }

        let rules: Vec<DetectionRule> = patterns
            .iter()
            .filter(|p| p.metadata.active)
            .map(|p| p.rule.clone())
            .collect();

        if rules.is_empty() {
            return None;
        }

        Some(Playbook {
            name: format!("Learned {} Patterns", language.name()),
            version: "1.0.0".to_string(),
            language: language.name().to_string(),
            author: "Sniff Learning System".to_string(),
            description: format!("Dynamically learned patterns for {}", language.name()),
            rules,
            metadata: HashMap::new(),
        })
    }

    /// Gets the path to the .sniff folder.
    pub fn sniff_path(&self) -> &Path {
        &self.sniff_path
    }
}