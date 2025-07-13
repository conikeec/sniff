// Copyright (c) 2025 Chetan Conikee <conikee@gmail.com>
// Licensed under the MIT License

//! Operation extraction and categorization for Claude Code tool usage.
//!
//! This module provides comprehensive analysis of tool operations found in
//! Claude Code sessions, including categorization, dependency analysis,
//! and metadata extraction.

use crate::error::Result;
use crate::types::{ClaudeMessage, ToolUseId, ToolUseOperation};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tracing::{debug, warn};

/// Categories of operations that can be performed by Claude Code tools.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperationType {
    /// File creation operations.
    FileCreate,
    /// File editing operations.
    FileEdit,
    /// File deletion operations.
    FileDelete,
    /// File reading operations.
    FileRead,
    /// File writing operations.
    FileWrite,
    /// Directory listing operations.
    DirectoryList,
    /// Directory creation operations.
    DirectoryCreate,
    /// File globbing/pattern matching operations.
    FileGlob,
    /// Text search and grep operations.
    TextSearch,
    /// Command execution operations.
    CommandExecution,
    /// Web fetching operations.
    WebFetch,
    /// Web search operations.
    WebSearch,
    /// Todo management operations.
    TodoManagement,
    /// Notebook operations.
    NotebookOperation,
    /// Task delegation operations.
    TaskDelegation,
    /// Unknown or unclassified operations.
    Unknown,
}

/// Status of an operation execution.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperationStatus {
    /// Operation completed successfully.
    Success,
    /// Operation failed with an error.
    Failed,
    /// Operation was interrupted.
    Interrupted,
    /// Operation status is unknown.
    Unknown,
}

/// Detailed information about a tool operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Operation {
    /// Unique identifier for the tool use.
    pub tool_use_id: ToolUseId,

    /// Type/category of the operation.
    pub operation_type: OperationType,

    /// Name of the tool that performed the operation.
    pub tool_name: String,

    /// Status of the operation execution.
    pub status: OperationStatus,

    /// Timestamp when the operation was requested.
    pub timestamp: DateTime<Utc>,

    /// Working directory when the operation was performed.
    pub working_directory: PathBuf,

    /// UUID of the message that requested this operation.
    pub message_uuid: String,

    /// File paths involved in the operation.
    pub file_paths: Vec<PathBuf>,

    /// Command executed (for command operations).
    pub command: Option<String>,

    /// Input parameters provided to the tool.
    pub input_parameters: HashMap<String, serde_json::Value>,

    /// Output/result data from the operation.
    pub output_data: Option<OperationOutput>,

    /// Duration of the operation in milliseconds.
    pub duration_ms: Option<u64>,

    /// Whether the operation modified files.
    pub modified_files: bool,

    /// Dependencies on other operations.
    pub dependencies: Vec<ToolUseId>,
}

/// Output data from an operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationOutput {
    /// Standard output content.
    pub stdout: Option<String>,

    /// Standard error content.
    pub stderr: Option<String>,

    /// Whether the output was truncated.
    pub truncated: bool,

    /// Number of files processed.
    pub files_processed: Option<usize>,

    /// File names returned by the operation.
    pub file_names: Vec<String>,

    /// Whether the output contains image data.
    pub is_image: bool,
}

/// Extracts and analyzes operations from Claude Code messages.
#[derive(Debug)]
pub struct OperationExtractor {
    /// Whether to perform dependency analysis.
    analyze_dependencies: bool,
    /// Whether to extract detailed output information.
    extract_output: bool,
}

/// Configuration for operation extraction.
#[derive(Debug, Clone)]
pub struct ExtractionConfig {
    /// Perform dependency analysis between operations.
    pub analyze_dependencies: bool,
    /// Extract detailed output information.
    pub extract_output: bool,
    /// Maximum number of operations to process.
    pub max_operations: Option<usize>,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            analyze_dependencies: true,
            extract_output: true,
            max_operations: None,
        }
    }
}

impl Default for OperationExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl OperationExtractor {
    /// Creates a new operation extractor with default configuration.
    pub fn new() -> Self {
        Self {
            analyze_dependencies: true,
            extract_output: true,
        }
    }

    /// Creates a new operation extractor with custom configuration.
    pub fn with_config(config: ExtractionConfig) -> Self {
        Self {
            analyze_dependencies: config.analyze_dependencies,
            extract_output: config.extract_output,
        }
    }

    /// Extracts operations from a sequence of Claude messages.
    ///
    /// # Errors
    ///
    /// Returns an error if operation extraction fails.
    pub fn extract_operations(&self, messages: &[ClaudeMessage]) -> Result<Vec<Operation>> {
        debug!("Extracting operations from {} messages", messages.len());

        let mut operations = Vec::new();
        let mut tool_results = HashMap::new();

        // First pass: collect tool results
        for message in messages {
            if let ClaudeMessage::User(user_msg) = message {
                if let Some(ref tool_result) = user_msg.tool_use_result {
                    // Extract tool result data for correlation
                    self.collect_tool_results(user_msg, tool_result, &mut tool_results);
                }
            }
        }

        // Second pass: extract operations and correlate with results
        for message in messages {
            let tool_uses = message.extract_tool_uses();
            for tool_use in tool_uses {
                match self.extract_operation(&tool_use, &tool_results) {
                    Ok(operation) => operations.push(operation),
                    Err(e) => {
                        warn!("Failed to extract operation from {}: {}", tool_use.id, e);
                    }
                }
            }
        }

        // Perform dependency analysis if enabled
        if self.analyze_dependencies {
            self.analyze_operation_dependencies(&mut operations)?;
        }

        debug!("Extracted {} operations", operations.len());
        Ok(operations)
    }

    /// Extracts a single operation from a tool use.
    fn extract_operation(
        &self,
        tool_use: &ToolUseOperation,
        tool_results: &HashMap<String, OperationOutput>,
    ) -> Result<Operation> {
        let operation_type = self.classify_operation(tool_use);
        let file_paths = self.extract_file_paths(tool_use);
        let command = self.extract_command(tool_use);
        let output_data = if self.extract_output {
            tool_results.get(&tool_use.id).cloned()
        } else {
            None
        };

        let modified_files = self.determines_file_modification(&operation_type, tool_use);

        Ok(Operation {
            tool_use_id: tool_use.id.clone(),
            operation_type,
            tool_name: tool_use.name.clone(),
            status: self.determine_operation_status(&output_data),
            timestamp: tool_use.timestamp,
            working_directory: tool_use.cwd.clone(),
            message_uuid: tool_use.message_uuid.clone(),
            file_paths,
            command,
            input_parameters: tool_use.input.clone(),
            output_data,
            duration_ms: None, // Will be filled from tool results if available
            modified_files,
            dependencies: Vec::new(), // Will be filled by dependency analysis
        })
    }

    /// Classifies the type of operation based on the tool use.
    fn classify_operation(&self, tool_use: &ToolUseOperation) -> OperationType {
        match tool_use.name.as_str() {
            "Read" => OperationType::FileRead,
            "Write" => OperationType::FileWrite,
            "Edit" | "MultiEdit" => OperationType::FileEdit,
            "LS" => OperationType::DirectoryList,
            "Glob" => OperationType::FileGlob,
            "Grep" => OperationType::TextSearch,
            "Bash" => OperationType::CommandExecution,
            "WebFetch" => OperationType::WebFetch,
            "WebSearch" => OperationType::WebSearch,
            "TodoWrite" => OperationType::TodoManagement,
            "NotebookRead" | "NotebookEdit" => OperationType::NotebookOperation,
            "Task" => OperationType::TaskDelegation,
            _ => OperationType::Unknown,
        }
    }

    /// Extracts file paths involved in the operation.
    fn extract_file_paths(&self, tool_use: &ToolUseOperation) -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // Extract from common file path parameters
        if let Some(file_path) = tool_use.get_string_input("file_path") {
            paths.push(PathBuf::from(file_path));
        }

        if let Some(notebook_path) = tool_use.get_string_input("notebook_path") {
            paths.push(PathBuf::from(notebook_path));
        }

        if let Some(path) = tool_use.get_string_input("path") {
            paths.push(PathBuf::from(path));
        }

        paths
    }

    /// Extracts command from bash operations.
    fn extract_command(&self, tool_use: &ToolUseOperation) -> Option<String> {
        if tool_use.name == "Bash" {
            tool_use.get_string_input("command")
        } else {
            None
        }
    }

    /// Determines if the operation modifies files.
    fn determines_file_modification(
        &self,
        op_type: &OperationType,
        _tool_use: &ToolUseOperation,
    ) -> bool {
        matches!(
            op_type,
            OperationType::FileCreate
                | OperationType::FileEdit
                | OperationType::FileWrite
                | OperationType::FileDelete
                | OperationType::DirectoryCreate
        )
    }

    /// Determines the operation status from output data.
    fn determine_operation_status(&self, output: &Option<OperationOutput>) -> OperationStatus {
        match output {
            Some(output) => {
                if output.stderr.as_ref().is_some_and(|s| !s.trim().is_empty()) {
                    OperationStatus::Failed
                } else {
                    OperationStatus::Success
                }
            }
            None => OperationStatus::Unknown,
        }
    }

    /// Collects tool results for correlation with operations.
    fn collect_tool_results(
        &self,
        user_msg: &crate::types::UserMessage,
        tool_result: &crate::types::ToolUseResultData,
        results: &mut HashMap<String, OperationOutput>,
    ) {
        // Extract tool use ID from the user message content
        let tool_use_ids = self.extract_tool_use_ids_from_user_message(user_msg);

        let output = match tool_result {
            crate::types::ToolUseResultData::Simple(text) => OperationOutput {
                stdout: Some(text.clone()),
                stderr: None,
                truncated: false,
                files_processed: None,
                file_names: vec![],
                is_image: false,
            },
            crate::types::ToolUseResultData::Structured(structured) => OperationOutput {
                stdout: Some(structured.stdout.clone()),
                stderr: Some(structured.stderr.clone()),
                truncated: structured.truncated.unwrap_or(false),
                files_processed: structured.num_files,
                file_names: structured.filenames.clone().unwrap_or_default(),
                is_image: structured.is_image,
            },
        };

        // Store the result for each tool use ID found in the message
        if tool_use_ids.is_empty() {
            // Fallback: if we can't extract tool_use_id, use a timestamp-based key
            let timestamp_key = format!("unknown_{}", user_msg.base.timestamp.timestamp_millis());
            results.insert(timestamp_key, output);
        } else {
            for tool_use_id in tool_use_ids {
                results.insert(tool_use_id, output.clone());
            }
        }
    }

    /// Extracts tool use IDs from user message content.
    fn extract_tool_use_ids_from_user_message(&self, user_msg: &crate::types::UserMessage) -> Vec<String> {
        use crate::types::{UserContentType, ContentBlock};
        
        let mut tool_use_ids = Vec::new();

        match &user_msg.message.content {
            UserContentType::ContentBlocks(blocks) => {
                for block in blocks {
                    if let ContentBlock::ToolResult { tool_use_id, .. } = block {
                        tool_use_ids.push(tool_use_id.clone());
                    }
                }
            }
            UserContentType::ToolResults(results) => {
                for result in results {
                    tool_use_ids.push(result.tool_use_id.clone());
                }
            }
            UserContentType::Text(_) => {
                // Plain text messages don't contain tool use IDs
            }
        }

        tool_use_ids
    }

    /// Analyzes dependencies between operations.
    fn analyze_operation_dependencies(&self, operations: &mut [Operation]) -> Result<()> {
        debug!("Analyzing dependencies for {} operations", operations.len());

        // Build a map of operations by ID for quick lookup
        let _op_map: HashMap<_, _> = operations
            .iter()
            .enumerate()
            .map(|(idx, op)| (op.tool_use_id.clone(), idx))
            .collect();

        // Analyze file-based dependencies
        for i in 0..operations.len() {
            let mut deps = Vec::new();

            // Find operations that this one depends on
            for j in 0..i {
                if self.has_dependency(&operations[j], &operations[i]) {
                    deps.push(operations[j].tool_use_id.clone());
                }
            }

            operations[i].dependencies = deps;
        }

        Ok(())
    }

    /// Determines if operation B depends on operation A.
    fn has_dependency(&self, op_a: &Operation, op_b: &Operation) -> bool {
        // File-based dependencies
        if !op_a.file_paths.is_empty() && !op_b.file_paths.is_empty() {
            let a_files: HashSet<_> = op_a.file_paths.iter().collect();
            let b_files: HashSet<_> = op_b.file_paths.iter().collect();

            // If B reads/edits files that A created/modified
            if op_a.modified_files && !a_files.is_disjoint(&b_files) {
                return true;
            }
        }

        // Directory-based dependencies
        if matches!(op_a.operation_type, OperationType::DirectoryCreate) {
            for b_file in &op_b.file_paths {
                for a_file in &op_a.file_paths {
                    if b_file.starts_with(a_file) {
                        return true;
                    }
                }
            }
        }

        false
    }
}

/// Statistics about extracted operations.
#[derive(Debug, Clone, PartialEq)]
pub struct OperationStats {
    /// Total number of operations.
    pub total_operations: usize,
    /// Operations by type.
    pub operations_by_type: HashMap<OperationType, usize>,
    /// Operations by status.
    pub operations_by_status: HashMap<OperationStatus, usize>,
    /// Number of operations that modified files.
    pub file_modifying_operations: usize,
    /// Most frequently used tools.
    pub tool_usage: HashMap<String, usize>,
    /// Files most frequently operated on.
    pub file_frequency: HashMap<PathBuf, usize>,
}

impl OperationStats {
    /// Computes statistics from a collection of operations.
    pub fn from_operations(operations: &[Operation]) -> Self {
        let mut stats = Self {
            total_operations: operations.len(),
            operations_by_type: HashMap::new(),
            operations_by_status: HashMap::new(),
            file_modifying_operations: 0,
            tool_usage: HashMap::new(),
            file_frequency: HashMap::new(),
        };

        for op in operations {
            // Count by type
            *stats
                .operations_by_type
                .entry(op.operation_type.clone())
                .or_insert(0) += 1;

            // Count by status
            *stats
                .operations_by_status
                .entry(op.status.clone())
                .or_insert(0) += 1;

            // Count file modifications
            if op.modified_files {
                stats.file_modifying_operations += 1;
            }

            // Count tool usage
            *stats.tool_usage.entry(op.tool_name.clone()).or_insert(0) += 1;

            // Count file frequency
            for file_path in &op.file_paths {
                *stats.file_frequency.entry(file_path.clone()).or_insert(0) += 1;
            }
        }

        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use chrono::Utc;

    fn create_test_tool_use(name: &str, id: &str) -> ToolUseOperation {
        let mut input = HashMap::new();
        if name == "Read" {
            input.insert(
                "file_path".to_string(),
                serde_json::Value::String("/test/file.txt".to_string()),
            );
        } else if name == "Bash" {
            input.insert(
                "command".to_string(),
                serde_json::Value::String("ls -la".to_string()),
            );
        }

        ToolUseOperation {
            id: id.to_string(),
            name: name.to_string(),
            input,
            message_uuid: "msg1".to_string(),
            timestamp: Utc::now(),
            cwd: PathBuf::from("/test"),
        }
    }

    #[test]
    fn test_operation_classification() {
        let extractor = OperationExtractor::new();

        let read_op = create_test_tool_use("Read", "tool1");
        assert_eq!(
            extractor.classify_operation(&read_op),
            OperationType::FileRead
        );

        let bash_op = create_test_tool_use("Bash", "tool2");
        assert_eq!(
            extractor.classify_operation(&bash_op),
            OperationType::CommandExecution
        );

        let unknown_op = create_test_tool_use("UnknownTool", "tool3");
        assert_eq!(
            extractor.classify_operation(&unknown_op),
            OperationType::Unknown
        );
    }

    #[test]
    fn test_file_path_extraction() {
        let extractor = OperationExtractor::new();

        let read_op = create_test_tool_use("Read", "tool1");
        let paths = extractor.extract_file_paths(&read_op);

        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], PathBuf::from("/test/file.txt"));
    }

    #[test]
    fn test_command_extraction() {
        let extractor = OperationExtractor::new();

        let bash_op = create_test_tool_use("Bash", "tool1");
        let command = extractor.extract_command(&bash_op);

        assert_eq!(command, Some("ls -la".to_string()));

        let read_op = create_test_tool_use("Read", "tool2");
        let no_command = extractor.extract_command(&read_op);

        assert_eq!(no_command, None);
    }

    #[test]
    fn test_operation_stats() {
        let operations = vec![
            Operation {
                tool_use_id: "1".to_string(),
                operation_type: OperationType::FileRead,
                tool_name: "Read".to_string(),
                status: OperationStatus::Success,
                timestamp: Utc::now(),
                working_directory: PathBuf::from("/test"),
                message_uuid: "msg1".to_string(),
                file_paths: vec![PathBuf::from("/test/file.txt")],
                command: None,
                input_parameters: HashMap::new(),
                output_data: None,
                duration_ms: None,
                modified_files: false,
                dependencies: Vec::new(),
            },
            Operation {
                tool_use_id: "2".to_string(),
                operation_type: OperationType::FileEdit,
                tool_name: "Edit".to_string(),
                status: OperationStatus::Success,
                timestamp: Utc::now(),
                working_directory: PathBuf::from("/test"),
                message_uuid: "msg2".to_string(),
                file_paths: vec![PathBuf::from("/test/file.txt")],
                command: None,
                input_parameters: HashMap::new(),
                output_data: None,
                duration_ms: None,
                modified_files: true,
                dependencies: Vec::new(),
            },
        ];

        let stats = OperationStats::from_operations(&operations);

        assert_eq!(stats.total_operations, 2);
        assert_eq!(stats.file_modifying_operations, 1);
        assert_eq!(
            stats.operations_by_type.get(&OperationType::FileRead),
            Some(&1)
        );
        assert_eq!(
            stats.operations_by_type.get(&OperationType::FileEdit),
            Some(&1)
        );
        assert_eq!(stats.tool_usage.get("Read"), Some(&1));
        assert_eq!(stats.tool_usage.get("Edit"), Some(&1));
    }
}
