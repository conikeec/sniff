// Copyright (c) 2025 Chetan Conikee <conikee@gmail.com>
// Licensed under the MIT License

//! Core types for Claude Code session data structures.
//!
//! This module defines the fundamental data types used throughout
//! the sniff application for representing Claude Code sessions,
//! messages, and operations.

#![allow(clippy::match_same_arms)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Unique identifier for a message within a session.
pub type MessageUuid = String;

/// Unique identifier for a Claude Code session.
pub type SessionId = String;

/// Unique identifier for a tool use operation.
pub type ToolUseId = String;

/// Unique identifier for a request.
pub type RequestId = String;

/// Claude Code version string.
pub type Version = String;

/// A complete Claude Code message as stored in JSONL files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ClaudeMessage {
    /// A message from the user.
    #[serde(rename = "user")]
    User(UserMessage),

    /// A message from the assistant.
    #[serde(rename = "assistant")]
    Assistant(AssistantMessage),

    /// A session summary message.
    #[serde(rename = "summary")]
    Summary(SummaryMessage),
}

/// Base fields common to all message types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MessageBase {
    /// Unique identifier for this message.
    pub uuid: MessageUuid,

    /// UUID of the parent message (null for session start).
    #[serde(rename = "parentUuid")]
    pub parent_uuid: Option<MessageUuid>,

    /// Whether this message is part of a sidechain.
    #[serde(rename = "isSidechain")]
    pub is_sidechain: bool,

    /// Whether this message is meta (optional field).
    #[serde(rename = "isMeta", skip_serializing_if = "Option::is_none")]
    pub is_meta: Option<bool>,

    /// Type of user (typically "external").
    #[serde(rename = "userType")]
    pub user_type: String,

    /// Current working directory when the message was created.
    pub cwd: PathBuf,

    /// Session identifier this message belongs to.
    #[serde(rename = "sessionId")]
    pub session_id: SessionId,

    /// Claude Code version that created this message.
    pub version: Version,

    /// Timestamp when the message was created.
    pub timestamp: DateTime<Utc>,
}

/// A user message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserMessage {
    /// Base message fields.
    #[serde(flatten)]
    pub base: MessageBase,

    /// The message content.
    pub message: UserMessageContent,

    /// Optional tool use result data.
    #[serde(rename = "toolUseResult", skip_serializing_if = "Option::is_none")]
    pub tool_use_result: Option<ToolUseResultData>,
}

/// Content of a user message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserMessageContent {
    /// Role is always "user" for user messages.
    pub role: String,

    /// The content, either text or tool result array.
    pub content: UserContentType,
}

/// Content type for user messages.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum UserContentType {
    /// Array of content blocks (newer format).
    ContentBlocks(Vec<ContentBlock>),

    /// Array of tool results (legacy format).
    ToolResults(Vec<ToolResult>),

    /// Simple text content (legacy format).
    Text(String),
}

/// A content block in a message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ContentBlock {
    /// Text content block.
    #[serde(rename = "text")]
    Text {
        /// The text content.
        text: String,
    },

    /// Tool result content block.
    #[serde(rename = "tool_result")]
    ToolResult {
        /// ID of the tool use this result corresponds to.
        tool_use_id: ToolUseId,
        /// The result content (can be string or array of content blocks).
        content: ToolResultContent,
        /// Whether this result represents an error.
        #[serde(default)]
        is_error: bool,
    },

    /// Tool use content block.
    #[serde(rename = "tool_use")]
    ToolUse {
        /// Unique identifier for this tool use.
        id: ToolUseId,
        /// Name of the tool being used.
        name: String,
        /// Input parameters for the tool.
        input: serde_json::Value,
    },
}

/// Content type for tool results which can be either a simple string or array of content blocks.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ToolResultContent {
    /// Array of nested content blocks (newer format).
    Nested(Vec<NestedContentBlock>),
    /// Simple string content (legacy format).
    Simple(String),
}

/// A nested content block within a tool result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum NestedContentBlock {
    /// Text content within a tool result.
    #[serde(rename = "text")]
    Text {
        /// The text content.
        text: String,
    },
}

/// A tool result within user message content.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolResult {
    /// ID of the tool use this result corresponds to.
    pub tool_use_id: ToolUseId,

    /// Type is always "`tool_result`".
    #[serde(rename = "type")]
    pub result_type: String,

    /// The actual result content.
    pub content: String,
}

/// Tool use result data that can be either a simple string or structured data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum ToolUseResultData {
    /// Simple string result (newer format).
    Simple(String),
    /// Structured result data (older format).
    Structured(ToolUseResult),
}

/// Extended tool use result information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolUseResult {
    /// Standard output from the tool.
    #[serde(default)]
    pub stdout: String,

    /// Standard error from the tool.
    #[serde(default)]
    pub stderr: String,

    /// Whether the tool execution was interrupted.
    #[serde(default)]
    pub interrupted: bool,

    /// Whether the result contains image data.
    #[serde(rename = "isImage", default)]
    pub is_image: bool,

    /// Old todos state (for `TodoWrite` tool).
    #[serde(rename = "oldTodos", skip_serializing_if = "Option::is_none")]
    pub old_todos: Option<Vec<Todo>>,

    /// New todos state (for `TodoWrite` tool).
    #[serde(rename = "newTodos", skip_serializing_if = "Option::is_none")]
    pub new_todos: Option<Vec<Todo>>,

    /// File names returned by tools like Glob.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filenames: Option<Vec<String>>,

    /// Duration in milliseconds for tool execution.
    #[serde(rename = "durationMs", skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,

    /// Number of files processed.
    #[serde(rename = "numFiles", skip_serializing_if = "Option::is_none")]
    pub num_files: Option<usize>,

    /// Whether results were truncated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
}

/// A todo item for the `TodoWrite` tool.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Todo {
    /// Unique identifier for the todo.
    pub id: String,

    /// Content/description of the todo.
    pub content: String,

    /// Current status of the todo.
    pub status: String,

    /// Priority level of the todo.
    pub priority: String,
}

/// An assistant message in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantMessage {
    /// Base message fields.
    #[serde(flatten)]
    pub base: MessageBase,

    /// The message content.
    pub message: AssistantMessageContent,

    /// Request ID for this assistant response.
    #[serde(rename = "requestId", skip_serializing_if = "Option::is_none")]
    pub request_id: Option<RequestId>,
}

/// Content of an assistant message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssistantMessageContent {
    /// Unique message ID from Claude.
    pub id: String,

    /// Type is always "message".
    #[serde(rename = "type")]
    pub message_type: String,

    /// Role is always "assistant".
    pub role: String,

    /// Model used for this response.
    pub model: String,

    /// Content array containing text and/or tool uses.
    pub content: Vec<AssistantContentItem>,

    /// Reason the response stopped.
    pub stop_reason: Option<String>,

    /// Stop sequence that triggered the stop.
    pub stop_sequence: Option<String>,

    /// Token usage information.
    pub usage: TokenUsage,
}

/// Individual content item in an assistant message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum AssistantContentItem {
    /// Text content.
    #[serde(rename = "text")]
    Text {
        /// The text content.
        text: String,
    },

    /// Tool use request.
    #[serde(rename = "tool_use")]
    ToolUse {
        /// Unique ID for this tool use.
        id: ToolUseId,
        /// Name of the tool to use.
        name: String,
        /// Input parameters for the tool.
        input: HashMap<String, serde_json::Value>,
    },

    /// Thinking content (internal reasoning).
    #[serde(rename = "thinking")]
    Thinking {
        /// The thinking content.
        thinking: String,
    },
}

/// Token usage information for an assistant message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenUsage {
    /// Number of input tokens used.
    pub input_tokens: u32,

    /// Number of cache creation input tokens.
    #[serde(default)]
    pub cache_creation_input_tokens: u32,

    /// Number of cache read input tokens.
    #[serde(default)]
    pub cache_read_input_tokens: u32,

    /// Number of output tokens generated.
    pub output_tokens: u32,

    /// Service tier used.
    pub service_tier: String,
}

/// A session summary message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SummaryMessage {
    /// Summary text describing the session or conversation.
    pub summary: String,

    /// Unique identifier for the leaf node in the conversation tree.
    #[serde(rename = "leafUuid")]
    pub leaf_uuid: String,
}

impl ClaudeMessage {
    /// Returns the base message information (if available).
    #[must_use]
    pub fn base(&self) -> Option<&MessageBase> {
        match self {
            ClaudeMessage::User(msg) => Some(&msg.base),
            ClaudeMessage::Assistant(msg) => Some(&msg.base),
            ClaudeMessage::Summary(_) => None, // Summary messages don't have base fields
        }
    }

    /// Returns the message UUID (if available).
    #[must_use]
    pub fn uuid(&self) -> Option<&MessageUuid> {
        self.base().map(|base| &base.uuid)
    }

    /// Returns the parent message UUID, if any.
    #[must_use]
    pub fn parent_uuid(&self) -> Option<&MessageUuid> {
        self.base()?.parent_uuid.as_ref()
    }

    /// Returns the session ID this message belongs to (if available).
    #[must_use]
    pub fn session_id(&self) -> Option<&SessionId> {
        self.base().map(|base| &base.session_id)
    }

    /// Returns the timestamp when this message was created (if available).
    #[must_use]
    pub fn timestamp(&self) -> Option<DateTime<Utc>> {
        self.base().map(|base| base.timestamp)
    }

    /// Returns the current working directory for this message (if available).
    #[must_use]
    pub fn cwd(&self) -> Option<&PathBuf> {
        self.base().map(|base| &base.cwd)
    }

    /// Returns true if this is a user message.
    #[must_use]
    pub fn is_user_message(&self) -> bool {
        matches!(self, ClaudeMessage::User(_))
    }

    /// Returns true if this is an assistant message.
    #[must_use]
    pub fn is_assistant_message(&self) -> bool {
        matches!(self, ClaudeMessage::Assistant(_))
    }

    /// Extracts tool use operations from this message.
    #[must_use]
    pub fn extract_tool_uses(&self) -> Vec<ToolUseOperation> {
        match self {
            ClaudeMessage::User(_) => Vec::new(),
            ClaudeMessage::Assistant(msg) => msg
                .message
                .content
                .iter()
                .filter_map(|item| match item {
                    AssistantContentItem::ToolUse { id, name, input } => Some(ToolUseOperation {
                        id: id.clone(),
                        name: name.clone(),
                        input: input.clone(),
                        message_uuid: msg.base.uuid.clone(),
                        timestamp: msg.base.timestamp,
                        cwd: msg.base.cwd.clone(),
                    }),
                    AssistantContentItem::Text { .. } | AssistantContentItem::Thinking { .. } => {
                        None
                    }
                })
                .collect(),
            ClaudeMessage::Summary(_) => Vec::new(), // Summary messages don't contain tool uses
        }
    }

    /// Extracts thinking content from this message for indexing and analysis.
    #[must_use]
    pub fn extract_thinking_content(&self) -> Vec<String> {
        match self {
            ClaudeMessage::User(_) => Vec::new(),
            ClaudeMessage::Assistant(msg) => {
                msg.message
                    .content
                    .iter()
                    .filter_map(|item| match item {
                        AssistantContentItem::Thinking { thinking } => Some(thinking.clone()),
                        AssistantContentItem::Text { .. }
                        | AssistantContentItem::ToolUse { .. } => None,
                    })
                    .collect()
            }
            ClaudeMessage::Summary(_) => Vec::new(), // Summary messages don't contain thinking content
        }
    }

    /// Extracts all text content (including thinking) from this message.
    #[must_use]
    pub fn extract_all_text_content(&self) -> Vec<String> {
        let mut content = Vec::new();

        match self {
            ClaudeMessage::User(msg) => {
                match &msg.message.content {
                    UserContentType::Text(text) => content.push(text.clone()),
                    UserContentType::ContentBlocks(blocks) => {
                        for block in blocks {
                            match block {
                                ContentBlock::Text { text } => content.push(text.clone()),
                                ContentBlock::ToolResult {
                                    content: tool_content,
                                    ..
                                } => {
                                    // Tool results contain valuable textual output that should be indexed
                                    match tool_content {
                                        ToolResultContent::Simple(text) => {
                                            content.push(text.clone());
                                        }
                                        ToolResultContent::Nested(blocks) => {
                                            for block in blocks {
                                                let NestedContentBlock::Text { text } = block;
                                                content.push(text.clone());
                                            }
                                        }
                                    }
                                }
                                ContentBlock::ToolUse { name, input, .. } => {
                                    // Include tool name for searchability
                                    content.push(format!("Tool: {name}"));
                                    // Include string parameters for searchability
                                    if let Some(obj) = input.as_object() {
                                        for (key, value) in obj {
                                            if let Some(str_value) = value.as_str() {
                                                content.push(format!("{key}: {str_value}"));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    UserContentType::ToolResults(results) => {
                        // Extract textual content from tool results
                        for result in results {
                            content.push(result.content.clone());
                        }
                    }
                }
            }
            ClaudeMessage::Assistant(msg) => {
                for item in &msg.message.content {
                    match item {
                        AssistantContentItem::Text { text } => content.push(text.clone()),
                        AssistantContentItem::Thinking { thinking } => {
                            content.push(thinking.clone());
                        }
                        AssistantContentItem::ToolUse { name, input, .. } => {
                            // Include tool name and parameters for searchability
                            content.push(format!("Tool: {name}"));
                            // Include string parameters for searchability
                            for (key, value) in input {
                                if let Some(str_value) = value.as_str() {
                                    content.push(format!("{key}: {str_value}"));
                                }
                            }
                        }
                    }
                }
            }
            ClaudeMessage::Summary(msg) => {
                // Include summary text for searchability
                content.push(msg.summary.clone());
            }
        }

        content
    }
}

/// Represents a tool use operation extracted from a message.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolUseOperation {
    /// Unique identifier for this tool use.
    pub id: ToolUseId,

    /// Name of the tool used.
    pub name: String,

    /// Input parameters passed to the tool.
    pub input: HashMap<String, serde_json::Value>,

    /// UUID of the message containing this tool use.
    pub message_uuid: MessageUuid,

    /// Timestamp when the tool use was requested.
    pub timestamp: DateTime<Utc>,

    /// Working directory when the tool was used.
    pub cwd: PathBuf,
}

impl ToolUseOperation {
    /// Returns the tool name.
    #[must_use]
    pub fn tool_name(&self) -> &str {
        &self.name
    }

    /// Returns the tool use ID.
    #[must_use]
    pub fn tool_id(&self) -> &str {
        &self.id
    }

    /// Gets a string input parameter by name.
    pub fn get_string_input(&self, key: &str) -> Option<String> {
        self.input
            .get(key)?
            .as_str()
            .map(std::string::ToString::to_string)
    }

    /// Gets a boolean input parameter by name.
    #[must_use]
    pub fn get_bool_input(&self, key: &str) -> Option<bool> {
        self.input.get(key)?.as_bool()
    }

    /// Gets a numeric input parameter by name.
    #[must_use]
    pub fn get_number_input(&self, key: &str) -> Option<f64> {
        self.input.get(key)?.as_f64()
    }

    /// Returns true if this is a file operation tool.
    #[must_use]
    pub fn is_file_operation(&self) -> bool {
        matches!(
            self.name.as_str(),
            "Read" | "Write" | "Edit" | "MultiEdit" | "NotebookRead" | "NotebookEdit"
        )
    }

    /// Returns true if this is a directory operation tool.
    #[must_use]
    pub fn is_directory_operation(&self) -> bool {
        matches!(self.name.as_str(), "LS" | "Glob")
    }

    /// Returns true if this is a command execution tool.
    #[must_use]
    pub fn is_command_operation(&self) -> bool {
        self.name == "Bash"
    }

    /// Returns true if this is a network operation tool.
    #[must_use]
    pub fn is_network_operation(&self) -> bool {
        matches!(self.name.as_str(), "WebFetch" | "WebSearch")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_user_message_parsing() {
        let json = r#"{
            "parentUuid": null,
            "isSidechain": false,
            "userType": "external",
            "cwd": "/Users/test",
            "sessionId": "test-session",
            "version": "1.0.0",
            "type": "user",
            "message": {
                "role": "user",
                "content": "Hello world"
            },
            "uuid": "test-uuid",
            "timestamp": "2025-01-01T00:00:00Z"
        }"#;

        let message: ClaudeMessage = serde_json::from_str(json).unwrap();
        assert!(message.is_user_message());
        assert_eq!(message.uuid(), Some(&"test-uuid".to_string()));
        assert_eq!(message.session_id(), Some(&"test-session".to_string()));
    }

    #[test]
    fn test_assistant_message_with_tool_use() {
        let json = r#"{
            "parentUuid": "parent-uuid",
            "isSidechain": false,
            "userType": "external",
            "cwd": "/Users/test",
            "sessionId": "test-session",
            "version": "1.0.0",
            "type": "assistant",
            "message": {
                "id": "msg_123",
                "type": "message",
                "role": "assistant",
                "model": "claude-3",
                "content": [
                    {
                        "type": "tool_use",
                        "id": "tool_123",
                        "name": "Read",
                        "input": {
                            "file_path": "/path/to/file"
                        }
                    }
                ],
                "stop_reason": null,
                "stop_sequence": null,
                "usage": {
                    "input_tokens": 10,
                    "output_tokens": 20,
                    "service_tier": "standard"
                }
            },
            "requestId": "req_123",
            "uuid": "test-uuid",
            "timestamp": "2025-01-01T00:00:00Z"
        }"#;

        let message: ClaudeMessage = serde_json::from_str(json).unwrap();
        assert!(message.is_assistant_message());

        let tool_uses = message.extract_tool_uses();
        assert_eq!(tool_uses.len(), 1);
        assert_eq!(tool_uses[0].name, "Read");
        assert_eq!(
            tool_uses[0].get_string_input("file_path"),
            Some("/path/to/file".to_string())
        );
    }

    #[test]
    fn test_tool_use_operation_classification() {
        let op = ToolUseOperation {
            id: "test".to_string(),
            name: "Read".to_string(),
            input: HashMap::new(),
            message_uuid: "msg".to_string(),
            timestamp: Utc::now(),
            cwd: PathBuf::from("/test"),
        };

        assert!(op.is_file_operation());
        assert!(!op.is_directory_operation());
        assert!(!op.is_command_operation());
        assert!(!op.is_network_operation());
    }
}

// Bincode-compatible storage types (for database serialization)
// These types convert untagged enums to tagged enums that bincode can handle

/// Bincode-compatible version of `UserContentType` for storage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StorageUserContentType {
    /// Array of content blocks (newer format).
    ContentBlocks(Vec<StorageContentBlock>),
    /// Array of tool results (legacy format).
    ToolResults(Vec<ToolResult>),
    /// Simple text content (legacy format).
    Text(String),
}

/// Bincode-compatible version of `ToolResultContent` for storage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StorageToolResultContent {
    /// Array of nested content blocks (newer format).
    Nested(Vec<NestedContentBlock>),
    /// Simple string content (legacy format).
    Simple(String),
}

/// Bincode-compatible version of `ClaudeMessage` for storage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StorageClaudeMessage {
    /// A message from the user.
    User(StorageUserMessage),
    /// A message from the assistant.
    Assistant(AssistantMessage),
    /// A session summary message.
    Summary(SummaryMessage),
}

/// Bincode-compatible version of `UserMessage` for storage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StorageUserMessage {
    /// Base message fields.
    pub base: MessageBase,
    /// The message content.
    pub message: StorageUserMessageContent,
    /// Optional tool use result data.
    pub tool_use_result: Option<ToolUseResultData>,
}

/// Bincode-compatible version of `UserMessageContent` for storage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StorageUserMessageContent {
    /// Role is always "user" for user messages.
    pub role: String,
    /// The content, converted to storage-compatible format.
    pub content: StorageUserContentType,
}

impl From<UserContentType> for StorageUserContentType {
    fn from(content: UserContentType) -> Self {
        match content {
            UserContentType::ContentBlocks(blocks) => StorageUserContentType::ContentBlocks(
                blocks.into_iter().map(std::convert::Into::into).collect(),
            ),
            UserContentType::ToolResults(results) => StorageUserContentType::ToolResults(results),
            UserContentType::Text(text) => StorageUserContentType::Text(text),
        }
    }
}

impl From<StorageUserContentType> for UserContentType {
    fn from(content: StorageUserContentType) -> Self {
        match content {
            StorageUserContentType::ContentBlocks(blocks) => UserContentType::ContentBlocks(
                blocks.into_iter().map(std::convert::Into::into).collect(),
            ),
            StorageUserContentType::ToolResults(results) => UserContentType::ToolResults(results),
            StorageUserContentType::Text(text) => UserContentType::Text(text),
        }
    }
}

impl From<ToolResultContent> for StorageToolResultContent {
    fn from(content: ToolResultContent) -> Self {
        match content {
            ToolResultContent::Nested(blocks) => StorageToolResultContent::Nested(blocks),
            ToolResultContent::Simple(text) => StorageToolResultContent::Simple(text),
        }
    }
}

impl From<StorageToolResultContent> for ToolResultContent {
    fn from(content: StorageToolResultContent) -> Self {
        match content {
            StorageToolResultContent::Nested(blocks) => ToolResultContent::Nested(blocks),
            StorageToolResultContent::Simple(text) => ToolResultContent::Simple(text),
        }
    }
}

/// Bincode-compatible version of `ContentBlock` for storage.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StorageContentBlock {
    /// Text content block.
    Text {
        /// The text content.
        text: String,
    },
    /// Tool result content block.
    ToolResult {
        /// ID of the tool use this result corresponds to.
        tool_use_id: ToolUseId,
        /// The result content (converted to storage format).
        content: StorageToolResultContent,
        /// Whether this result represents an error.
        is_error: bool,
    },
    /// Tool use content block.
    ToolUse {
        /// Unique identifier for this tool use.
        id: ToolUseId,
        /// Name of the tool being used.
        name: String,
        /// Input parameters for the tool.
        input: std::collections::BTreeMap<String, serde_json::Value>,
    },
}

impl From<ContentBlock> for StorageContentBlock {
    fn from(block: ContentBlock) -> Self {
        match block {
            ContentBlock::Text { text } => StorageContentBlock::Text { text },
            ContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
            } => StorageContentBlock::ToolResult {
                tool_use_id,
                content: content.into(),
                is_error,
            },
            ContentBlock::ToolUse { id, name, input } => StorageContentBlock::ToolUse {
                id,
                name,
                input: input
                    .as_object()
                    .map(|obj| obj.clone().into_iter().collect())
                    .unwrap_or_default(),
            },
        }
    }
}

impl From<StorageContentBlock> for ContentBlock {
    fn from(block: StorageContentBlock) -> Self {
        match block {
            StorageContentBlock::Text { text } => ContentBlock::Text { text },
            StorageContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
            } => ContentBlock::ToolResult {
                tool_use_id,
                content: content.into(),
                is_error,
            },
            StorageContentBlock::ToolUse { id, name, input } => ContentBlock::ToolUse {
                id,
                name,
                input: serde_json::Value::Object(input.into_iter().collect()),
            },
        }
    }
}

impl From<ClaudeMessage> for StorageClaudeMessage {
    fn from(message: ClaudeMessage) -> Self {
        match message {
            ClaudeMessage::User(user_msg) => StorageClaudeMessage::User(StorageUserMessage {
                base: user_msg.base,
                message: StorageUserMessageContent {
                    role: user_msg.message.role,
                    content: user_msg.message.content.into(),
                },
                tool_use_result: user_msg.tool_use_result,
            }),
            ClaudeMessage::Assistant(assistant_msg) => {
                StorageClaudeMessage::Assistant(assistant_msg)
            }
            ClaudeMessage::Summary(summary_msg) => StorageClaudeMessage::Summary(summary_msg),
        }
    }
}

impl From<StorageClaudeMessage> for ClaudeMessage {
    fn from(message: StorageClaudeMessage) -> Self {
        match message {
            StorageClaudeMessage::User(user_msg) => ClaudeMessage::User(UserMessage {
                base: user_msg.base,
                message: UserMessageContent {
                    role: user_msg.message.role,
                    content: user_msg.message.content.into(),
                },
                tool_use_result: user_msg.tool_use_result,
            }),
            StorageClaudeMessage::Assistant(assistant_msg) => {
                ClaudeMessage::Assistant(assistant_msg)
            }
            StorageClaudeMessage::Summary(summary_msg) => ClaudeMessage::Summary(summary_msg),
        }
    }
}

impl StorageClaudeMessage {
    /// Extracts all text content (including thinking) from this message.
    #[must_use]
    pub fn extract_all_text_content(&self) -> Vec<String> {
        // Convert to regular ClaudeMessage and use its method
        let regular_message: ClaudeMessage = self.clone().into();
        regular_message.extract_all_text_content()
    }
}
