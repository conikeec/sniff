// Copyright (c) 2025 Chetan Conikee <conikee@gmail.com>
// Licensed under the MIT License

//! JSONL parsing for Claude Code session files.
//!
//! This module provides robust parsing of Claude Code JSONL session files,
//! with comprehensive error handling and validation.

use crate::error::{Result, SniffError};
use crate::types::{ClaudeMessage, SessionId};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use tracing::{debug, info, warn};

/// A parser for Claude Code JSONL session files.
#[derive(Debug)]
pub struct JsonlParser {
    /// Whether to validate message consistency during parsing.
    validate_consistency: bool,
    /// Maximum number of lines to parse (0 for unlimited).
    max_lines: usize,
}

/// Configuration for JSONL parsing operations.
#[derive(Debug, Clone)]
pub struct ParseConfig {
    /// Validate message threading and consistency.
    pub validate_consistency: bool,
    /// Maximum number of lines to parse (0 for unlimited).
    pub max_lines: usize,
    /// Skip malformed lines instead of failing.
    pub skip_malformed: bool,
}

impl Default for ParseConfig {
    fn default() -> Self {
        Self {
            validate_consistency: true,
            max_lines: 0,
            skip_malformed: false,
        }
    }
}

/// Result of parsing a JSONL session file.
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// Successfully parsed messages.
    pub messages: Vec<ClaudeMessage>,
    /// Number of lines processed.
    pub lines_processed: usize,
    /// Number of malformed lines encountered.
    pub malformed_lines: usize,
    /// Session ID extracted from messages (if consistent).
    pub session_id: Option<SessionId>,
    /// Validation warnings encountered during parsing.
    pub warnings: Vec<String>,
}

impl Default for JsonlParser {
    fn default() -> Self {
        Self::new()
    }
}

impl JsonlParser {
    /// Creates a new JSONL parser with default configuration.
    #[must_use]
    pub fn new() -> Self {
        Self {
            validate_consistency: true,
            max_lines: 0,
        }
    }

    /// Creates a new JSONL parser with custom configuration.
    #[must_use]
    pub fn with_config(config: ParseConfig) -> Self {
        Self {
            validate_consistency: config.validate_consistency,
            max_lines: config.max_lines,
        }
    }

    /// Parses a JSONL file from the given path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or contains invalid JSONL data.
    pub fn parse_file(&self, path: impl AsRef<Path>) -> Result<ParseResult> {
        let path = path.as_ref();
        debug!("Parsing JSONL file: {}", path.display());

        let file = File::open(path).map_err(|e| SniffError::file_system(path, e))?;

        self.parse_reader(BufReader::new(file))
    }

    /// Parses JSONL data from a buffered reader.
    ///
    /// # Errors
    ///
    /// Returns an error if the data contains invalid JSONL or fails validation.
    pub fn parse_reader<R: BufRead>(&self, reader: R) -> Result<ParseResult> {
        let mut messages = Vec::new();
        let mut lines_processed = 0;
        let mut malformed_lines = 0;
        let mut session_id = None;
        let mut warnings = Vec::new();

        for (line_number, line) in reader.lines().enumerate() {
            let line_num = line_number + 1;

            // Check line limit
            if self.max_lines > 0 && lines_processed >= self.max_lines {
                debug!("Reached maximum line limit: {}", self.max_lines);
                break;
            }

            let line = line.map_err(|e| SniffError::file_system("reader", e))?;
            lines_processed += 1;

            // Skip empty lines
            if line.trim().is_empty() {
                continue;
            }

            // Parse the JSON line
            match self.parse_line(&line, line_num) {
                Ok(message) => {
                    // Validate session consistency (only for messages that have session IDs)
                    if let Some(msg_session_id) = message.session_id() {
                        if let Some(ref existing_session) = session_id {
                            if msg_session_id != existing_session {
                                warnings.push(format!(
                                    "Session ID mismatch at line {line_num}: expected '{existing_session}', found '{msg_session_id}'"
                                ));
                            }
                        } else {
                            session_id = Some(msg_session_id.clone());
                        }
                    }

                    messages.push(message);
                }
                Err(e) => {
                    malformed_lines += 1;
                    warn!("Failed to parse line {}: {}", line_num, e);

                    // If not skipping malformed lines, return the error
                    if !self.should_skip_malformed() {
                        return Err(e);
                    }
                }
            }
        }

        // Perform consistency validation if enabled
        if self.validate_consistency {
            self.validate_message_consistency(&messages, &mut warnings)?;
        }

        info!(
            "Parsed {} messages from {} lines ({} malformed)",
            messages.len(),
            lines_processed,
            malformed_lines
        );

        Ok(ParseResult {
            messages,
            lines_processed,
            malformed_lines,
            session_id,
            warnings,
        })
    }

    /// Parses a single JSONL line into a Claude message.
    fn parse_line(&self, line: &str, line_number: usize) -> Result<ClaudeMessage> {
        serde_json::from_str(line).map_err(|e| SniffError::jsonl_parse(line_number, e))
    }

    /// Validates the consistency of parsed messages.
    fn validate_message_consistency(
        &self,
        messages: &[ClaudeMessage],
        warnings: &mut Vec<String>,
    ) -> Result<()> {
        debug!(
            "Validating message consistency for {} messages",
            messages.len()
        );

        // Check for duplicate UUIDs (only for messages that have UUIDs)
        let mut seen_uuids = std::collections::HashSet::new();
        for message in messages {
            if let Some(uuid) = message.uuid() {
                if !seen_uuids.insert(uuid.clone()) {
                    return Err(SniffError::invalid_session(format!(
                        "Duplicate message UUID: {uuid}"
                    )));
                }
            }
        }

        // Validate parent-child relationships (only for messages with UUIDs)
        let uuid_set: std::collections::HashSet<_> =
            messages.iter().filter_map(|m| m.uuid()).cloned().collect();

        for message in messages {
            if let Some(parent_uuid) = message.parent_uuid() {
                if !uuid_set.contains(parent_uuid) {
                    if let Some(msg_uuid) = message.uuid() {
                        warnings.push(format!(
                            "Message {msg_uuid} references non-existent parent: {parent_uuid}"
                        ));
                    }
                }
            }
        }

        // Validate chronological ordering (only for messages with timestamps)
        for window in messages.windows(2) {
            if let (Some(ts1), Some(ts2)) = (window[0].timestamp(), window[1].timestamp()) {
                if ts1 > ts2 {
                    warnings.push(format!(
                        "Messages not in chronological order: {} ({}) after {} ({})",
                        window[0].uuid().unwrap_or(&"<summary>".to_string()),
                        ts1,
                        window[1].uuid().unwrap_or(&"<summary>".to_string()),
                        ts2
                    ));
                }
            }
        }

        Ok(())
    }

    /// Returns whether malformed lines should be skipped.
    fn should_skip_malformed(&self) -> bool {
        // For now, this is hardcoded, but could be part of configuration
        false
    }
}

/// Utility functions for working with JSONL files.
pub mod utils {
    use super::{
        BufRead, BufReader, ClaudeMessage, File, JsonlParser, Path, Result, SessionId, SniffError,
    };
    use std::fs;

    /// Counts the number of lines in a JSONL file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn count_lines(path: impl AsRef<Path>) -> Result<usize> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| SniffError::file_system(path.as_ref(), e))?;

        Ok(content.lines().count())
    }

    /// Validates that a file appears to be a valid JSONL file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file is not valid JSONL.
    pub fn validate_jsonl_file(path: impl AsRef<Path>) -> Result<bool> {
        let parser = JsonlParser::new();
        let result = parser.parse_file(path)?;

        Ok(result.malformed_lines == 0)
    }

    /// Extracts the session ID from a JSONL file without full parsing.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or doesn't contain valid messages.
    pub fn extract_session_id(path: impl AsRef<Path>) -> Result<Option<SessionId>> {
        let file =
            File::open(path.as_ref()).map_err(|e| SniffError::file_system(path.as_ref(), e))?;

        let reader = BufReader::new(file);

        // Read just the first few lines to find session ID
        for line in reader.lines().take(5) {
            let line = line.map_err(|e| SniffError::file_system(path.as_ref(), e))?;

            if line.trim().is_empty() {
                continue;
            }

            if let Ok(message) = serde_json::from_str::<ClaudeMessage>(&line) {
                if let Some(session_id) = message.session_id() {
                    return Ok(Some(session_id.clone()));
                }
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_jsonl() -> String {
        r#"{"parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/Users/test","sessionId":"test-session","version":"1.0.0","type":"user","message":{"role":"user","content":"Hello"},"uuid":"msg1","timestamp":"2025-01-01T00:00:00Z"}
{"parentUuid":"msg1","isSidechain":false,"userType":"external","cwd":"/Users/test","sessionId":"test-session","version":"1.0.0","type":"assistant","message":{"id":"response1","type":"message","role":"assistant","model":"claude-3","content":[{"type":"text","text":"Hi there!"}],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":5,"output_tokens":3,"service_tier":"standard"}},"requestId":"req1","uuid":"msg2","timestamp":"2025-01-01T00:01:00Z"}"#.to_string()
    }

    #[test]
    fn test_parse_valid_jsonl() {
        let jsonl_data = create_test_jsonl();
        let cursor = Cursor::new(jsonl_data);

        let parser = JsonlParser::new();
        let result = parser.parse_reader(cursor).unwrap();

        assert_eq!(result.messages.len(), 2);
        assert_eq!(result.lines_processed, 2);
        assert_eq!(result.malformed_lines, 0);
        assert_eq!(result.session_id, Some("test-session".to_string()));
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn test_parse_with_malformed_line() {
        let jsonl_data = format!(
            "{}\n{}\n{}",
            r#"{"valid":"json","type":"user","uuid":"1","parentUuid":null,"isSidechain":false,"userType":"external","cwd":"/test","sessionId":"test","version":"1.0","message":{"role":"user","content":"test"},"timestamp":"2025-01-01T00:00:00Z"}"#,
            "invalid json line",
            r#"{"valid":"json","type":"user","uuid":"2","parentUuid":"1","isSidechain":false,"userType":"external","cwd":"/test","sessionId":"test","version":"1.0","message":{"role":"user","content":"test2"},"timestamp":"2025-01-01T00:01:00Z"}"#
        );

        let cursor = Cursor::new(jsonl_data);
        let parser = JsonlParser::new();

        // Should fail because we don't skip malformed lines by default
        assert!(parser.parse_reader(cursor).is_err());
    }

    #[test]
    fn test_parse_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "{}", create_test_jsonl()).unwrap();

        let parser = JsonlParser::new();
        let result = parser.parse_file(temp_file.path()).unwrap();

        assert_eq!(result.messages.len(), 2);
        assert_eq!(result.session_id, Some("test-session".to_string()));
    }

    #[test]
    fn test_max_lines_limit() {
        let jsonl_data = create_test_jsonl();
        let cursor = Cursor::new(jsonl_data);

        let config = ParseConfig {
            max_lines: 1,
            ..Default::default()
        };
        let parser = JsonlParser::with_config(config);
        let result = parser.parse_reader(cursor).unwrap();

        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.lines_processed, 1);
    }

    #[test]
    fn test_utils_count_lines() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "{}", create_test_jsonl()).unwrap();

        let count = utils::count_lines(temp_file.path()).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_utils_extract_session_id() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "{}", create_test_jsonl().lines().next().unwrap()).unwrap();

        let session_id = utils::extract_session_id(temp_file.path()).unwrap();
        assert_eq!(session_id, Some("test-session".to_string()));
    }
}
