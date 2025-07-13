// Copyright (c) 2025 Chetan Conikee <conikee@gmail.com>
// Licensed under the MIT License

//! Enhanced search interface with reasoning tree visualization.
//!
//! This module provides an interactive search experience that shows conversation
//! context, tool usage, and relationships between messages in a visual tree format.

use crate::error::{SniffError, Result};
use crate::storage::TreeStorage;
use crate::types::{ClaudeMessage, MessageUuid, SessionId};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// Represents a conversation thread with context and relationships.
#[derive(Debug, Clone)]
pub struct ConversationThread {
    /// Root message that started this thread.
    pub root_message: MessageContext,
    /// All messages in this thread, ordered chronologically.
    pub messages: Vec<MessageContext>,
    /// Tools used during this conversation thread.
    pub tools_used: Vec<ToolContext>,
    /// Session this thread belongs to.
    pub session_id: SessionId,
}

/// Extended message context with relationships and surrounding information.
#[derive(Debug, Clone)]
pub struct MessageContext {
    /// The core message.
    pub message: ClaudeMessage,
    /// Direct parent message (if any).
    pub parent: Option<MessageUuid>,
    /// Direct children messages.
    pub children: Vec<MessageUuid>,
    /// Messages that came before in the same thread (for context).
    pub preceding_context: Vec<MessageUuid>,
    /// Messages that came after in the same thread (for context).
    pub following_context: Vec<MessageUuid>,
    /// Match information if this message matched the search query.
    pub search_match: Option<SearchMatch>,
}

/// Information about how a message matched the search query.
#[derive(Debug, Clone)]
pub struct SearchMatch {
    /// The specific text that matched.
    pub matched_text: String,
    /// Context snippet around the match.
    pub snippet: String,
    /// Relevance score (0.0 to 1.0).
    pub relevance: f64,
    /// Type of content that matched (thinking, text, tool_input, etc.).
    pub match_type: MatchType,
}

/// Type of content that matched the search query.
#[derive(Debug, Clone, PartialEq)]
pub enum MatchType {
    /// Regular message text.
    MessageText,
    /// Assistant thinking content.
    Thinking,
    /// Tool input parameters.
    ToolInput,
    /// Tool output/results.
    ToolOutput,
    /// Tool name.
    ToolName,
}

/// Context about tool usage in a conversation.
#[derive(Debug, Clone)]
pub struct ToolContext {
    /// Tool name.
    pub name: String,
    /// Tool use ID.
    pub id: String,
    /// Input parameters.
    pub input: HashMap<String, serde_json::Value>,
    /// Output/result (if available).
    pub output: Option<String>,
    /// Message that requested this tool.
    pub request_message: MessageUuid,
    /// Message that contains the result (if any).
    pub result_message: Option<MessageUuid>,
    /// Timestamp when tool was used.
    pub timestamp: DateTime<Utc>,
}

/// Configuration for search behavior and display.
#[derive(Debug, Clone)]
pub struct SearchConfig {
    /// Maximum number of results to return.
    pub max_results: usize,
    /// Number of context messages to show before/after matches.
    pub context_window: usize,
    /// Whether to include thinking content in search.
    pub include_thinking: bool,
    /// Whether to include tool parameters in search.
    pub include_tool_params: bool,
    /// Minimum relevance score to include in results.
    pub min_relevance: f64,
    /// Terminal width for responsive layout.
    pub terminal_width: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_results: 20,
            context_window: 3,
            include_thinking: true,
            include_tool_params: true,
            min_relevance: 0.1,
            terminal_width: 80,
        }
    }
}

/// Enhanced search engine with conversation context.
pub struct EnhancedSearchEngine {
    storage: TreeStorage,
    config: SearchConfig,
}

impl EnhancedSearchEngine {
    /// Creates a new enhanced search engine.
    pub fn new(storage: TreeStorage, config: SearchConfig) -> Self {
        Self { storage, config }
    }

    /// Performs an enhanced search with conversation context.
    pub fn search(&mut self, query: &str) -> Result<Vec<ConversationThread>> {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }

        // Step 1: Find all sessions with matching content using existing search
        let basic_results = self.storage.search_content(query, self.config.max_results * 2)?;
        
        let mut threads = Vec::new();
        
        // Step 2: Build conversation threads for each matching session
        for (session_id, _snippets) in basic_results {
            match self.build_basic_thread(&session_id, query) {
                Ok(thread) => threads.push(thread),
                Err(_) => {
                    // Session found in basic search but no actual message matches - skip silently
                    // This ensures we only return threads with real matching content
                }
            }
        }
        
        // Step 3: Sort by relevance and limit results
        threads.sort_by(|a, b| {
            let a_score = self.calculate_thread_relevance(a, query);
            let b_score = self.calculate_thread_relevance(b, query);
            b_score.partial_cmp(&a_score).unwrap_or(std::cmp::Ordering::Equal)
        });
        
        threads.truncate(self.config.max_results);
        
        Ok(threads)
    }

    /// Builds a real conversation thread for a session using actual stored data.
    fn build_basic_thread(&mut self, session_id: &SessionId, query: &str) -> Result<ConversationThread> {
        // Get the session root hash
        let session_root_hash = match self.storage.get_session_root(session_id)? {
            Some(hash) => hash,
            None => return Err(SniffError::storage_error(format!("Session {} not found", session_id))),
        };
        
        // Get the session node 
        let session_node = match self.storage.get_node(&session_root_hash)? {
            Some(node) => node,
            None => return Err(SniffError::storage_error(format!("Session node not found for {}", session_id))),
        };
        
        let mut all_messages = Vec::new();
        let mut tools_used = Vec::new();
        let mut matching_messages = Vec::new();
        
        // First pass: collect all messages in chronological order
        for (_child_key, child_hash) in &session_node.children {
            if let Ok(Some(child_node)) = self.storage.get_node(child_hash) {
                match &child_node.node_type {
                    crate::tree::NodeType::Message { message_uuid, timestamp, role: _ } => {
                        if let Some(ref content_data) = child_node.content {
                            if let Ok(message) = serde_json::from_slice::<crate::types::ClaudeMessage>(content_data) {
                                // Extract tool usage from this message before moving
                                let tool_uses = message.extract_tool_uses();
                                
                                all_messages.push((message_uuid.clone(), message, *timestamp));
                                for tool_use in tool_uses {
                                    tools_used.push(ToolContext {
                                        name: tool_use.name.clone(),
                                        id: tool_use.id.clone(),
                                        input: tool_use.input.clone(),
                                        output: self.find_tool_result(&tool_use.id, &session_node),
                                        request_message: message_uuid.clone(),
                                        result_message: self.find_tool_result_message(&tool_use.id, &session_node),
                                        timestamp: *timestamp,
                                    });
                                }
                            }
                        }
                    }
                    crate::tree::NodeType::Operation { tool_use_id, tool_name, timestamp } => {
                        // Extract operation-level tool information
                        if let Some(ref content_data) = child_node.content {
                            if let Ok(operation_data) = serde_json::from_slice::<serde_json::Value>(content_data) {
                                if tool_name.to_lowercase().contains(&query.to_lowercase()) {
                                    tools_used.push(ToolContext {
                                        name: tool_name.clone(),
                                        id: tool_use_id.clone(),
                                        input: operation_data.get("input").cloned().unwrap_or_default().as_object().map(|obj| {
                                            obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                                        }).unwrap_or_default(),
                                        output: operation_data.get("output").and_then(|v| v.as_str()).map(|s| s.to_string()),
                                        request_message: "operation".to_string(),
                                        result_message: None,
                                        timestamp: *timestamp,
                                    });
                                }
                            }
                        }
                    }
                    _ => {} // Skip other node types
                }
            }
        }
        
        // Sort all messages chronologically
        all_messages.sort_by(|a, b| a.2.cmp(&b.2));
        
        // Second pass: find matching messages and build context
        let query_lower = query.to_lowercase();
        for (i, (message_uuid, message, _timestamp)) in all_messages.iter().enumerate() {
            let text_content = message.extract_all_text_content();
            let thinking_content = message.extract_thinking_content();
            
            // Check for matches in all content types
            let mut search_matches = Vec::new();
            
            // Check regular text content
            for text in &text_content {
                if text.to_lowercase().contains(&query_lower) {
                    let snippet = self.extract_debugging_context(text, query);
                    search_matches.push(SearchMatch {
                        matched_text: query.to_string(),
                        snippet,
                        relevance: self.calculate_text_relevance(text, query),
                        match_type: MatchType::MessageText,
                    });
                }
            }
            
            // Check thinking content
            if self.config.include_thinking {
                for thinking in &thinking_content {
                    if thinking.to_lowercase().contains(&query_lower) {
                        let snippet = self.extract_debugging_context(thinking, query);
                        search_matches.push(SearchMatch {
                            matched_text: query.to_string(),
                            snippet,
                            relevance: self.calculate_text_relevance(thinking, query) + 0.1, // Boost thinking content
                            match_type: MatchType::Thinking,
                        });
                    }
                }
            }
            
            // If we found matches, create message context with surrounding conversation
            if !search_matches.is_empty() {
                let best_match = search_matches.into_iter().max_by(|a, b| 
                    a.relevance.partial_cmp(&b.relevance).unwrap_or(std::cmp::Ordering::Equal)
                ).unwrap();
                
                // Extract preceding and following context
                let context_start = i.saturating_sub(self.config.context_window);
                let context_end = (i + self.config.context_window + 1).min(all_messages.len());
                
                let preceding_context: Vec<MessageUuid> = all_messages[context_start..i]
                    .iter()
                    .map(|(uuid, _, _)| uuid.clone())
                    .collect();
                    
                let following_context: Vec<MessageUuid> = all_messages[i + 1..context_end]
                    .iter()
                    .map(|(uuid, _, _)| uuid.clone())
                    .collect();
                
                // Find parent/child relationships
                let parent = message.base().and_then(|base| base.parent_uuid.clone());
                let children = all_messages.iter()
                    .filter(|(_, msg, _)| msg.base().and_then(|base| base.parent_uuid.as_ref()) == Some(message_uuid))
                    .map(|(uuid, _, _)| uuid.clone())
                    .collect();
                
                let message_ctx = MessageContext {
                    message: message.clone(),
                    parent,
                    children,
                    preceding_context,
                    following_context,
                    search_match: Some(best_match),
                };
                
                matching_messages.push(message_ctx);
            }
        }
        
        // Only proceed if we have matching messages - no fake fallbacks
        if matching_messages.is_empty() {
            return Err(SniffError::storage_error(format!(
                "No messages found matching '{}' in session {}", 
                query, 
                session_id
            )));
        }
        
        let root_message = matching_messages[0].clone();
        
        Ok(ConversationThread {
            root_message,
            messages: matching_messages,
            tools_used,
            session_id: session_id.clone(),
        })
    }

    /// Extracts debugging context - what file was changed, what tools were used, what was the thinking.
    fn extract_debugging_context(&self, text: &str, query: &str) -> String {
        let query_lower = query.to_lowercase();
        let text_lower = text.to_lowercase();
        
        // Check if this is a file path
        if text.contains('/') && (text.contains(".rs") || text.contains(".ts") || text.contains(".py") || text.contains(".js") || text.contains(".md")) {
            return format!("📁 {}", text.trim());
        }
        
        // Check if this is tool output with file information
        if text.contains("Tool result:") || text.contains("→") {
            let lines: Vec<&str> = text.lines().take(3).collect();
            return format!("🔧 {}", lines.join(" | "));
        }
        
        // Check if this looks like a command
        if text.trim_start().starts_with("$") || text.contains("command:") {
            return format!("💻 {}", text.trim());
        }
        
        // For thinking content, show more context
        if text.len() > 200 {
            if let Some(match_pos) = text_lower.find(&query_lower) {
                let start = match_pos.saturating_sub(100).max(0);
                let end = (match_pos + query.len() + 100).min(text.len());
                
                // Ensure we don't cut UTF-8 characters
                let safe_start = text.char_indices().find(|(i, _)| *i >= start).map_or(0, |(i, _)| i);
                let safe_end = text.char_indices().rev().find(|(i, _)| *i <= end).map_or(text.len(), |(i, _)| i);
                
                let snippet = &text[safe_start..safe_end].trim();
                format!("💭 ...{}...", snippet.replace('\n', " "))
            } else {
                format!("💭 {}", text.chars().take(150).collect::<String>())
            }
        } else {
            text.trim().to_string()
        }
    }
    
    /// Calculates relevance score based on match quality and context.
    fn calculate_text_relevance(&self, text: &str, query: &str) -> f64 {
        let query_lower = query.to_lowercase();
        let text_lower = text.to_lowercase();
        
        let mut score: f64 = 0.0;
        
        // Exact match bonus
        if text_lower.contains(&query_lower) {
            score += 0.5;
        }
        
        // Word boundary match bonus
        for word in query_lower.split_whitespace() {
            if text_lower.split_whitespace().any(|w| w == word) {
                score += 0.3;
            }
        }
        
        // File path bonus (if it looks like a path)
        if text.contains('/') && (text.contains(".rs") || text.contains(".md") || text.contains(".toml")) {
            score += 0.2;
        }
        
        // Command/tool usage bonus
        if text.contains("tool") || text.contains("command") || text.contains("run") {
            score += 0.1;
        }
        
        // Length penalty for very short or very long texts
        let text_len = text.len();
        if text_len < 20 {
            score *= 0.7; // Prefer longer, more contextual matches
        } else if text_len > 1000 {
            score *= 0.8; // Slight penalty for very long matches
        }
        
        score.min(1.0)
    }
    
    /// Determines the type of match based on content characteristics.
    
    /// Finds the output/result for a specific tool use ID.
    fn find_tool_result(&mut self, tool_use_id: &str, session_node: &crate::tree::MerkleNode) -> Option<String> {
        for (_child_key, child_hash) in &session_node.children {
            if let Ok(Some(child_node)) = self.storage.get_node(child_hash) {
                if let crate::tree::NodeType::Operation { tool_use_id: op_id, .. } = &child_node.node_type {
                    if op_id == tool_use_id {
                        if let Some(ref content_data) = child_node.content {
                            if let Ok(operation_data) = serde_json::from_slice::<serde_json::Value>(content_data) {
                                return operation_data.get("output").and_then(|v| v.as_str()).map(|s| s.to_string());
                            }
                        }
                    }
                }
            }
        }
        None
    }
    
    /// Finds the message UUID that contains the result for a specific tool use.
    fn find_tool_result_message(&mut self, tool_use_id: &str, session_node: &crate::tree::MerkleNode) -> Option<String> {
        for (_child_key, child_hash) in &session_node.children {
            if let Ok(Some(child_node)) = self.storage.get_node(child_hash) {
                if let crate::tree::NodeType::Message { message_uuid, .. } = &child_node.node_type {
                    if let Some(ref content_data) = child_node.content {
                        if let Ok(message) = serde_json::from_slice::<crate::types::ClaudeMessage>(content_data) {
                            // Check if this message contains a tool result for our tool use ID
                            let text_content = message.extract_all_text_content();
                            for text in text_content {
                                if text.contains(tool_use_id) && text.contains("Tool result:") {
                                    return Some(message_uuid.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    }
    
    

    /// Calculates relevance score for a thread with enhanced context awareness.
    fn calculate_thread_relevance(&self, thread: &ConversationThread, query: &str) -> f64 {
        let mut total_score = 0.0;
        let mut count = 0;
        
        // Base relevance from message matches
        for message_ctx in &thread.messages {
            if let Some(search_match) = &message_ctx.search_match {
                total_score += search_match.relevance;
                count += 1;
                
                // Bonus for thinking content matches (more valuable insights)
                if search_match.match_type == MatchType::Thinking {
                    total_score += 0.15;
                }
                
                // Bonus for messages with rich context (preceding/following messages)
                if !message_ctx.preceding_context.is_empty() || !message_ctx.following_context.is_empty() {
                    total_score += 0.1;
                }
            }
        }
        
        // Tool usage relevance bonuses
        if !thread.tools_used.is_empty() {
            total_score += 0.1;
            
            // Bonus for tool workflows (multiple related tools)
            if thread.tools_used.len() > 2 {
                total_score += 0.15;
            }
            
            // Bonus for tools with actual outputs (more actionable)
            let tools_with_output = thread.tools_used.iter().filter(|t| t.output.is_some()).count();
            total_score += (tools_with_output as f64) * 0.05;
        }
        
        // Query matches in tool names and parameters
        let query_lower = query.to_lowercase();
        for tool in &thread.tools_used {
            if tool.name.to_lowercase().contains(&query_lower) {
                total_score += 0.2;
            }
            
            // Check tool input parameters for query matches
            for (param_name, param_value) in &tool.input {
                if param_name.to_lowercase().contains(&query_lower) ||
                   param_value.to_string().to_lowercase().contains(&query_lower) {
                    total_score += 0.1;
                }
            }
        }
        
        // Conversation length bonus (longer conversations often have more context)
        if thread.messages.len() > 5 {
            total_score += 0.05;
        }
        
        if count > 0 {
            total_score / count as f64
        } else {
            // Even threads without direct matches can be relevant due to tools
            if !thread.tools_used.is_empty() {
                0.2 // Base relevance for tool-related sessions
            } else {
                0.0
            }
        }
    }
}

