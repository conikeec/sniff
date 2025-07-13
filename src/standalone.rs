// Copyright (c) 2025 Chetan Conikee <conikee@gmail.com>
// Licensed under the MIT License

//! Standalone file analysis capabilities independent of Claude Code sessions.
//!
//! This module provides the ability to analyze arbitrary files for bullshit patterns,
//! create checkpoints for change tracking, and integrate with editors like Cursor,
//! Windsurf, and VS Code.

use crate::analysis::{BullshitAnalyzer, BullshitDetection};
use crate::error::{Result, SniffError};
use crate::SupportedLanguage;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info, warn};

/// Configuration for standalone file analysis.
#[derive(Debug, Clone)]
pub struct AnalysisConfig {
    /// File filtering configuration.
    pub filter: FileFilter,
    /// Force a specific language for all files (overrides detection).
    pub force_language: Option<SupportedLanguage>,
    /// Enable detailed analysis with additional context.
    pub detailed_analysis: bool,
}

/// File filtering configuration.
#[derive(Debug, Clone)]
pub struct FileFilter {
    /// Include hidden files and directories.
    pub include_hidden: bool,
    /// Allowed file extensions (e.g., ["rs", "py", "ts"]).
    pub allowed_extensions: Option<Vec<String>>,
    /// Pattern to exclude files (glob pattern).
    pub exclude_pattern: Option<String>,
    /// Maximum file size to analyze (in bytes).
    pub max_file_size_bytes: u64,
}

impl Default for FileFilter {
    fn default() -> Self {
        Self {
            include_hidden: false,
            allowed_extensions: None,
            exclude_pattern: None,
            max_file_size_bytes: 10 * 1024 * 1024, // 10MB
        }
    }
}

/// Standalone analyzer for arbitrary files.
pub struct StandaloneAnalyzer {
    bullshit_analyzer: BullshitAnalyzer,
    config: AnalysisConfig,
    language_detector: LanguageDetector,
}

impl StandaloneAnalyzer {
    /// Creates a new standalone analyzer.
    pub fn new(bullshit_analyzer: BullshitAnalyzer, config: AnalysisConfig) -> Self {
        Self {
            bullshit_analyzer,
            config,
            language_detector: LanguageDetector::new(),
        }
    }

    /// Analyzes the specified files and directories.
    pub async fn analyze_files(&mut self, paths: &[PathBuf]) -> Result<AnalysisResults> {
        let mut discovered_files = Vec::new();

        // Discover all files to analyze
        for path in paths {
            if path.is_file() {
                if self.should_analyze_file(path).await? {
                    discovered_files.push(path.clone());
                }
            } else if path.is_dir() {
                let dir_files = self.discover_files_in_directory(path).await?;
                discovered_files.extend(dir_files);
            } else {
                warn!("Path does not exist or is not accessible: {}", path.display());
            }
        }

        if discovered_files.is_empty() {
            return Ok(AnalysisResults::empty());
        }

        info!("Analyzing {} files", discovered_files.len());

        // Analyze each file
        let mut file_results = Vec::new();
        let mut total_detections = 0;
        let mut critical_issues = 0;
        let mut quality_scores = Vec::new();

        for file_path in discovered_files {
            match self.analyze_single_file(&file_path).await {
                Ok(result) => {
                    total_detections += result.detections.len();
                    critical_issues += result.detections.iter()
                        .filter(|d| matches!(d.severity, crate::playbook::Severity::Critical))
                        .count();
                    quality_scores.push(result.quality_score);
                    file_results.push(result);
                }
                Err(e) => {
                    warn!("Failed to analyze {}: {}", file_path.display(), e);
                }
            }
        }

        let average_quality_score = if quality_scores.is_empty() {
            100.0
        } else {
            quality_scores.iter().sum::<f64>() / quality_scores.len() as f64
        };

        Ok(AnalysisResults {
            total_files: file_results.len(),
            total_detections,
            critical_issues,
            average_quality_score,
            file_results,
        })
    }

    /// Analyzes a single file.
    async fn analyze_single_file(&mut self, file_path: &Path) -> Result<FileAnalysisResult> {
        debug!("Analyzing file: {}", file_path.display());

        // Read file content
        let content = fs::read_to_string(file_path).await
            .map_err(|e| SniffError::file_system(file_path, e))?;

        // Detect or use forced language
        let language = if let Some(forced) = self.config.force_language {
            Some(forced)
        } else {
            self.language_detector.detect_from_path(file_path)
        };

        if language.is_none() {
            debug!("Unknown language for file: {}, skipping", file_path.display());
            return Ok(FileAnalysisResult {
                file_path: file_path.to_path_buf(),
                language: None,
                detections: Vec::new(),
                quality_score: 100.0,
                analysis_metadata: AnalysisMetadata::default(),
            });
        }

        let lang = language.unwrap();

        // Analyze content for bullshit patterns
        let detections = self.bullshit_analyzer.analyze_content_for_language(
            &content,
            lang,
            file_path.to_str().unwrap_or("unknown"),
        )?;

        // Calculate quality score
        let quality_score = self.calculate_quality_score(&detections);

        // Gather analysis metadata
        let metadata = if self.config.detailed_analysis {
            AnalysisMetadata {
                line_count: content.lines().count(),
                char_count: content.chars().count(),
                file_size_bytes: content.len(),
                complexity_metrics: self.calculate_complexity_metrics(&content, lang),
            }
        } else {
            AnalysisMetadata::default()
        };

        Ok(FileAnalysisResult {
            file_path: file_path.to_path_buf(),
            language: Some(lang),
            detections,
            quality_score,
            analysis_metadata: metadata,
        })
    }

    /// Discovers files in a directory recursively.
    async fn discover_files_in_directory(&self, dir_path: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let mut stack = vec![dir_path.to_path_buf()];

        while let Some(current_dir) = stack.pop() {
            let mut entries = fs::read_dir(&current_dir).await
                .map_err(|e| SniffError::file_system(&current_dir, e))?;

            while let Some(entry) = entries.next_entry().await
                .map_err(|e| SniffError::file_system(&current_dir, e))? {
                
                let path = entry.path();
                
                // Skip hidden files/directories unless configured to include them
                if !self.config.filter.include_hidden {
                    if let Some(file_name) = path.file_name() {
                        if file_name.to_string_lossy().starts_with('.') {
                            continue;
                        }
                    }
                }

                if path.is_dir() {
                    stack.push(path);
                } else if self.should_analyze_file(&path).await? {
                    files.push(path);
                }
            }
        }

        Ok(files)
    }

    /// Checks if a file should be analyzed based on the filter configuration.
    async fn should_analyze_file(&self, file_path: &Path) -> Result<bool> {
        // Check file size
        if let Ok(metadata) = fs::metadata(file_path).await {
            if metadata.len() > self.config.filter.max_file_size_bytes {
                debug!("Skipping large file: {} ({} bytes)", 
                    file_path.display(), metadata.len());
                return Ok(false);
            }
        }

        // Check file extension
        if let Some(ref allowed_extensions) = self.config.filter.allowed_extensions {
            if let Some(extension) = file_path.extension() {
                let ext_str = extension.to_string_lossy().to_lowercase();
                if !allowed_extensions.iter().any(|allowed| allowed.to_lowercase() == ext_str) {
                    return Ok(false);
                }
            } else {
                // No extension, skip if we have extension filters
                return Ok(false);
            }
        }

        // Check exclude pattern (simplified - would use proper glob matching in production)
        if let Some(ref exclude_pattern) = self.config.filter.exclude_pattern {
            let path_str = file_path.to_string_lossy();
            if path_str.contains(exclude_pattern) {
                debug!("Excluding file matching pattern '{}': {}", 
                    exclude_pattern, file_path.display());
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Calculates a quality score based on detected patterns.
    fn calculate_quality_score(&self, detections: &[BullshitDetection]) -> f64 {
        if detections.is_empty() {
            return 100.0;
        }

        let mut penalty = 0.0;
        for detection in detections {
            penalty += match detection.severity {
                crate::playbook::Severity::Critical => 25.0,
                crate::playbook::Severity::High => 15.0,
                crate::playbook::Severity::Medium => 8.0,
                crate::playbook::Severity::Low => 3.0,
                crate::playbook::Severity::Info => 1.0,
            };
        }

        (100.0 - penalty).max(0.0)
    }

    /// Calculates basic complexity metrics for a file.
    fn calculate_complexity_metrics(&self, content: &str, _language: SupportedLanguage) -> ComplexityMetrics {
        // Simple metrics - could be enhanced with proper AST analysis
        let lines = content.lines().collect::<Vec<_>>();
        let non_empty_lines = lines.iter().filter(|line| !line.trim().is_empty()).count();
        let comment_lines = lines.iter().filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*")
        }).count();
        
        ComplexityMetrics {
            cyclomatic_complexity: 1, // Placeholder
            nesting_depth: self.calculate_max_nesting_depth(content),
            function_count: self.count_functions(content),
            comment_ratio: if non_empty_lines > 0 { 
                comment_lines as f64 / non_empty_lines as f64 
            } else { 
                0.0 
            },
        }
    }

    fn calculate_max_nesting_depth(&self, content: &str) -> usize {
        let mut max_depth = 0;
        let mut current_depth = 0;
        
        for line in content.lines() {
            let trimmed = line.trim();
            // Simple heuristic - count braces (works for C-like languages)
            for ch in trimmed.chars() {
                match ch {
                    '{' | '(' | '[' => current_depth += 1,
                    '}' | ')' | ']' => current_depth = current_depth.saturating_sub(1),
                    _ => {}
                }
            }
            max_depth = max_depth.max(current_depth);
        }
        
        max_depth
    }

    fn count_functions(&self, content: &str) -> usize {
        // Simple heuristic - count function-like patterns
        content.lines().filter(|line| {
            let trimmed = line.trim();
            trimmed.contains("fn ") || 
            trimmed.contains("def ") || 
            trimmed.contains("function ") ||
            (trimmed.contains("(") && trimmed.contains(")") && trimmed.contains("{"))
        }).count()
    }
}

/// Language detection utility.
struct LanguageDetector {
    extension_map: HashMap<String, SupportedLanguage>,
}

impl LanguageDetector {
    fn new() -> Self {
        let mut extension_map = HashMap::new();
        
        extension_map.insert("rs".to_string(), SupportedLanguage::Rust);
        extension_map.insert("py".to_string(), SupportedLanguage::Python);
        extension_map.insert("pyw".to_string(), SupportedLanguage::Python);
        extension_map.insert("ts".to_string(), SupportedLanguage::TypeScript);
        extension_map.insert("tsx".to_string(), SupportedLanguage::TypeScript);
        extension_map.insert("js".to_string(), SupportedLanguage::JavaScript);
        extension_map.insert("jsx".to_string(), SupportedLanguage::JavaScript);
        extension_map.insert("go".to_string(), SupportedLanguage::Go);
        extension_map.insert("c".to_string(), SupportedLanguage::C);
        extension_map.insert("h".to_string(), SupportedLanguage::C);
        extension_map.insert("cpp".to_string(), SupportedLanguage::Cpp);
        extension_map.insert("cxx".to_string(), SupportedLanguage::Cpp);
        extension_map.insert("cc".to_string(), SupportedLanguage::Cpp);
        extension_map.insert("hpp".to_string(), SupportedLanguage::Cpp);

        Self { extension_map }
    }

    fn detect_from_path(&self, path: &Path) -> Option<SupportedLanguage> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_lowercase())
            .and_then(|ext| self.extension_map.get(&ext))
            .copied()
    }
}

/// Results of analyzing multiple files.
#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisResults {
    /// Total number of files analyzed.
    pub total_files: usize,
    /// Total number of pattern detections across all files.
    pub total_detections: usize,
    /// Number of critical issues found.
    pub critical_issues: usize,
    /// Average quality score across all files.
    pub average_quality_score: f64,
    /// Individual file analysis results.
    pub file_results: Vec<FileAnalysisResult>,
}

impl AnalysisResults {
    fn empty() -> Self {
        Self {
            total_files: 0,
            total_detections: 0,
            critical_issues: 0,
            average_quality_score: 100.0,
            file_results: Vec::new(),
        }
    }
}

/// Results of analyzing a single file.
#[derive(Debug, Serialize, Deserialize)]
pub struct FileAnalysisResult {
    /// Path to the analyzed file.
    pub file_path: PathBuf,
    /// Detected language (if any).
    pub language: Option<SupportedLanguage>,
    /// Bullshit patterns detected in the file.
    pub detections: Vec<BullshitDetection>,
    /// Overall quality score for the file (0-100).
    pub quality_score: f64,
    /// Additional analysis metadata.
    pub analysis_metadata: AnalysisMetadata,
}

/// Additional metadata about the analysis.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct AnalysisMetadata {
    /// Number of lines in the file.
    pub line_count: usize,
    /// Number of characters in the file.
    pub char_count: usize,
    /// File size in bytes.
    pub file_size_bytes: usize,
    /// Complexity metrics.
    pub complexity_metrics: ComplexityMetrics,
}

/// Code complexity metrics.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    /// Cyclomatic complexity (simplified calculation).
    pub cyclomatic_complexity: usize,
    /// Maximum nesting depth.
    pub nesting_depth: usize,
    /// Number of functions/methods.
    pub function_count: usize,
    /// Ratio of comment lines to total lines.
    pub comment_ratio: f64,
}

/// Checkpoint management for tracking file changes over time.
pub struct CheckpointManager {
    project_dir: PathBuf,
    checkpoint_dir: PathBuf,
}

impl CheckpointManager {
    /// Creates a new checkpoint manager for the given project directory.
    pub fn new(project_dir: &Path) -> Result<Self> {
        let checkpoint_dir = project_dir.join(".sniff/checkpoints");
        
        Ok(Self {
            project_dir: project_dir.to_path_buf(),
            checkpoint_dir,
        })
    }

    /// Creates a new checkpoint with the current state of specified files.
    pub async fn create_checkpoint(
        &self,
        name: &str,
        paths: &[PathBuf],
        description: Option<String>,
    ) -> Result<()> {
        // Ensure checkpoint directory exists
        fs::create_dir_all(&self.checkpoint_dir).await
            .map_err(|e| SniffError::file_system(&self.checkpoint_dir, e))?;

        let checkpoint = Checkpoint {
            name: name.to_string(),
            description,
            timestamp: Utc::now(),
            file_count: 0, // Will be updated below
            files: HashMap::new(),
        };

        let checkpoint_file = self.checkpoint_dir.join(format!("{}.json", name));
        let mut file_snapshots = HashMap::new();
        let mut total_files = 0;

        // Capture file states
        for path in paths {
            let snapshots = self.capture_file_states(path).await?;
            total_files += snapshots.len();
            file_snapshots.extend(snapshots);
        }

        let final_checkpoint = Checkpoint {
            file_count: total_files,
            files: file_snapshots,
            ..checkpoint
        };

        // Save checkpoint to file
        let checkpoint_json = serde_json::to_string_pretty(&final_checkpoint)
            .map_err(|e| SniffError::invalid_format("checkpoint".to_string(), e.to_string()))?;
        
        fs::write(&checkpoint_file, checkpoint_json).await
            .map_err(|e| SniffError::file_system(&checkpoint_file, e))?;

        info!("Created checkpoint '{}' with {} files", name, total_files);
        Ok(())
    }

    /// Lists all available checkpoints.
    pub async fn list_checkpoints(&self) -> Result<Vec<CheckpointInfo>> {
        if !self.checkpoint_dir.exists() {
            return Ok(Vec::new());
        }

        let mut checkpoints = Vec::new();
        let mut entries = fs::read_dir(&self.checkpoint_dir).await
            .map_err(|e| SniffError::file_system(&self.checkpoint_dir, e))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| SniffError::file_system(&self.checkpoint_dir, e))? {
            
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                if let Some(name) = path.file_stem() {
                    if let Ok(checkpoint) = self.load_checkpoint(&name.to_string_lossy()).await {
                        checkpoints.push(CheckpointInfo {
                            name: checkpoint.name,
                            description: checkpoint.description,
                            timestamp: checkpoint.timestamp,
                            file_count: checkpoint.file_count,
                        });
                    }
                }
            }
        }

        // Sort by timestamp (newest first)
        checkpoints.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(checkpoints)
    }

    /// Gets detailed information about a specific checkpoint.
    pub async fn get_checkpoint(&self, name: &str) -> Result<Option<CheckpointInfo>> {
        match self.load_checkpoint(name).await {
            Ok(checkpoint) => Ok(Some(CheckpointInfo {
                name: checkpoint.name,
                description: checkpoint.description,
                timestamp: checkpoint.timestamp,
                file_count: checkpoint.file_count,
            })),
            Err(_) => Ok(None),
        }
    }

    /// Gets file details for a checkpoint.
    pub async fn get_checkpoint_files(&self, name: &str) -> Result<Vec<FileInfo>> {
        let checkpoint = self.load_checkpoint(name).await?;
        let mut file_infos = Vec::new();

        for (path_str, snapshot) in checkpoint.files {
            file_infos.push(FileInfo {
                path: PathBuf::from(path_str),
                file_size: snapshot.size,
                modified_time: snapshot.modified_time,
                content_hash: snapshot.content_hash,
            });
        }

        file_infos.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(file_infos)
    }

    /// Compares current file state against a checkpoint.
    pub async fn compare_files(&self, checkpoint_name: &str, paths: &[PathBuf]) -> Result<FileComparison> {
        let checkpoint = self.load_checkpoint(checkpoint_name).await?;
        let current_files = self.capture_file_states_flat(paths).await?;

        let checkpoint_paths: HashSet<_> = checkpoint.files.keys().cloned().collect();
        let current_paths: HashSet<_> = current_files.keys().cloned().collect();

        let new_files: Vec<PathBuf> = current_paths.difference(&checkpoint_paths)
            .map(|p| PathBuf::from(p))
            .collect();
        
        let deleted_files: Vec<PathBuf> = checkpoint_paths.difference(&current_paths)
            .map(|p| PathBuf::from(p))
            .collect();

        let mut changed_files = Vec::new();
        for path_str in checkpoint_paths.intersection(&current_paths) {
            if let (Some(checkpoint_snapshot), Some(current_snapshot)) = 
                (checkpoint.files.get(path_str), current_files.get(path_str)) {
                if checkpoint_snapshot.content_hash != current_snapshot.content_hash {
                    changed_files.push(PathBuf::from(path_str));
                }
            }
        }

        Ok(FileComparison {
            new_files,
            changed_files,
            deleted_files,
        })
    }

    /// Deletes a checkpoint.
    pub async fn delete_checkpoint(&self, name: &str) -> Result<()> {
        let checkpoint_file = self.checkpoint_dir.join(format!("{}.json", name));
        if checkpoint_file.exists() {
            fs::remove_file(&checkpoint_file).await
                .map_err(|e| SniffError::file_system(&checkpoint_file, e))?;
            info!("Deleted checkpoint '{}'", name);
        }
        Ok(())
    }

    /// Captures the state of all files in the given paths.
    async fn capture_file_states(&self, path: &Path) -> Result<HashMap<String, FileSnapshot>> {
        let mut snapshots = HashMap::new();
        
        if path.is_file() {
            if let Some(snapshot) = self.capture_single_file_state(path).await? {
                snapshots.insert(path.to_string_lossy().to_string(), snapshot);
            }
        } else if path.is_dir() {
            let files = self.discover_all_files(path).await?;
            for file_path in files {
                if let Some(snapshot) = self.capture_single_file_state(&file_path).await? {
                    snapshots.insert(file_path.to_string_lossy().to_string(), snapshot);
                }
            }
        }

        Ok(snapshots)
    }

    /// Captures file states and returns a flat map.
    async fn capture_file_states_flat(&self, paths: &[PathBuf]) -> Result<HashMap<String, FileSnapshot>> {
        let mut all_snapshots = HashMap::new();
        
        for path in paths {
            let snapshots = self.capture_file_states(path).await?;
            all_snapshots.extend(snapshots);
        }
        
        Ok(all_snapshots)
    }

    /// Captures the state of a single file.
    async fn capture_single_file_state(&self, file_path: &Path) -> Result<Option<FileSnapshot>> {
        if !file_path.is_file() {
            return Ok(None);
        }

        let metadata = fs::metadata(file_path).await
            .map_err(|e| SniffError::file_system(file_path, e))?;

        let content = fs::read(file_path).await
            .map_err(|e| SniffError::file_system(file_path, e))?;

        let content_hash = blake3::hash(&content);

        Ok(Some(FileSnapshot {
            size: metadata.len(),
            modified_time: metadata.modified()
                .map_err(|e| SniffError::file_system(file_path, e))?
                .into(),
            content_hash: hex::encode(content_hash.as_bytes()),
        }))
    }

    /// Discovers all files in a directory recursively.
    async fn discover_all_files(&self, dir_path: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        let mut stack = vec![dir_path.to_path_buf()];

        while let Some(current_dir) = stack.pop() {
            let mut entries = fs::read_dir(&current_dir).await
                .map_err(|e| SniffError::file_system(&current_dir, e))?;

            while let Some(entry) = entries.next_entry().await
                .map_err(|e| SniffError::file_system(&current_dir, e))? {
                
                let path = entry.path();
                
                // Skip .sniff directory to avoid recursion
                if path.file_name().map_or(false, |name| name == ".sniff") {
                    continue;
                }

                if path.is_dir() {
                    stack.push(path);
                } else {
                    files.push(path);
                }
            }
        }

        Ok(files)
    }

    /// Loads a checkpoint from disk.
    async fn load_checkpoint(&self, name: &str) -> Result<Checkpoint> {
        let checkpoint_file = self.checkpoint_dir.join(format!("{}.json", name));
        
        if !checkpoint_file.exists() {
            return Err(SniffError::not_found(format!("Checkpoint '{}' not found", name)));
        }

        let content = fs::read_to_string(&checkpoint_file).await
            .map_err(|e| SniffError::file_system(&checkpoint_file, e))?;

        let checkpoint: Checkpoint = serde_json::from_str(&content)
            .map_err(|e| SniffError::invalid_format("checkpoint".to_string(), e.to_string()))?;

        Ok(checkpoint)
    }
}

/// Information about a checkpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct CheckpointInfo {
    /// Checkpoint name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// When the checkpoint was created.
    pub timestamp: DateTime<Utc>,
    /// Number of files in the checkpoint.
    pub file_count: usize,
}

/// Complete checkpoint data.
#[derive(Debug, Serialize, Deserialize)]
struct Checkpoint {
    /// Checkpoint name.
    name: String,
    /// Optional description.
    description: Option<String>,
    /// When the checkpoint was created.
    timestamp: DateTime<Utc>,
    /// Number of files in the checkpoint.
    file_count: usize,
    /// File snapshots keyed by file path.
    files: HashMap<String, FileSnapshot>,
}

/// Snapshot of a file's state at a point in time.
#[derive(Debug, Serialize, Deserialize)]
struct FileSnapshot {
    /// File size in bytes.
    size: u64,
    /// Last modified time.
    modified_time: DateTime<Utc>,
    /// Hash of file content.
    content_hash: String,
}

/// Information about a file.
#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfo {
    /// File path.
    pub path: PathBuf,
    /// File size in bytes.
    pub file_size: u64,
    /// Last modified time.
    pub modified_time: DateTime<Utc>,
    /// Content hash.
    pub content_hash: String,
}

/// Result of comparing current state against a checkpoint.
#[derive(Debug, Serialize, Deserialize)]
pub struct FileComparison {
    /// Files that exist now but not in the checkpoint.
    pub new_files: Vec<PathBuf>,
    /// Files that exist in both but have different content.
    pub changed_files: Vec<PathBuf>,
    /// Files that existed in the checkpoint but not now.
    pub deleted_files: Vec<PathBuf>,
}