// Copyright (c) 2025 Chetan Conikee <conikee@gmail.com>
// Licensed under the MIT License

//! File system watcher for Claude Code project directories.
//!
//! This module provides real-time monitoring of Claude Code project
//! directories, detecting changes to session files and triggering
//! appropriate processing workflows.

use crate::error::{SniffError, Result};
use crate::types::SessionId;
use notify::{
    event::{CreateKind, ModifyKind, RemoveKind},
    Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc as tokio_mpsc;
use tracing::{debug, error, info, warn};

/// Events emitted by the file watcher.
#[derive(Debug, Clone, PartialEq)]
pub enum WatchEvent {
    /// A new project directory was created.
    ProjectCreated {
        /// Path to the project directory.
        project_path: PathBuf,
        /// Inferred project name.
        project_name: String,
    },

    /// A project directory was removed.
    ProjectRemoved {
        /// Path to the project directory.
        project_path: PathBuf,
        /// Project name.
        project_name: String,
    },

    /// A session file was created.
    SessionCreated {
        /// Path to the session file.
        file_path: PathBuf,
        /// Extracted session ID.
        session_id: SessionId,
        /// Project path containing the session.
        project_path: PathBuf,
    },

    /// A session file was modified.
    SessionModified {
        /// Path to the session file.
        file_path: PathBuf,
        /// Extracted session ID.
        session_id: SessionId,
        /// Project path containing the session.
        project_path: PathBuf,
    },

    /// A session file was removed.
    SessionRemoved {
        /// Path to the session file.
        file_path: PathBuf,
        /// Extracted session ID.
        session_id: SessionId,
        /// Project path containing the session.
        project_path: PathBuf,
    },
}

/// Configuration for the file watcher.
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Path to the Claude projects directory.
    pub claude_projects_path: PathBuf,
    /// Debounce interval to avoid duplicate events.
    pub debounce_duration: Duration,
    /// Whether to process existing files on startup.
    pub process_existing: bool,
    /// File extensions to watch (typically ".jsonl").
    pub watched_extensions: Vec<String>,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

        Self {
            claude_projects_path: home_dir.join(".claude").join("projects"),
            debounce_duration: Duration::from_millis(500),
            process_existing: true,
            watched_extensions: vec!["jsonl".to_string()],
        }
    }
}

/// File system watcher for Claude Code projects.
pub struct ClaudeWatcher {
    /// Configuration for the watcher.
    config: WatcherConfig,
    /// Channel sender for emitting watch events.
    event_sender: tokio_mpsc::UnboundedSender<WatchEvent>,
    /// Debounce map to avoid duplicate events.
    debounce_map: HashMap<PathBuf, Instant>,
}

impl ClaudeWatcher {
    /// Creates a new Claude watcher with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the watcher cannot be initialized.
    pub fn new(config: WatcherConfig) -> Result<(Self, tokio_mpsc::UnboundedReceiver<WatchEvent>)> {
        let (event_sender, event_receiver) = tokio_mpsc::unbounded_channel();

        let watcher = Self {
            config,
            event_sender,
            debounce_map: HashMap::new(),
        };

        Ok((watcher, event_receiver))
    }

    /// Starts watching the Claude projects directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the watcher cannot be started or the directory doesn't exist.
    pub async fn start_watching(mut self) -> Result<()> {
        info!(
            "Starting Claude watcher for: {}",
            self.config.claude_projects_path.display()
        );

        // Ensure the projects directory exists
        if !self.config.claude_projects_path.exists() {
            return Err(SniffError::project_discovery(
                &self.config.claude_projects_path,
                "Claude projects directory does not exist",
            ));
        }

        // Process existing files if configured
        if self.config.process_existing {
            self.process_existing_files().await?;
        }

        // Set up the file system watcher
        let (tx, rx) = mpsc::channel();
        let mut watcher = RecommendedWatcher::new(
            tx,
            notify::Config::default().with_poll_interval(Duration::from_millis(100)),
        )
        .map_err(SniffError::file_watcher)?;

        // Watch the projects directory recursively
        watcher
            .watch(&self.config.claude_projects_path, RecursiveMode::Recursive)
            .map_err(SniffError::file_watcher)?;

        info!("File watcher started successfully");

        // Process file system events
        loop {
            match rx.recv() {
                Ok(event) => {
                    if let Err(e) = self.handle_fs_event(event).await {
                        error!("Error handling file system event: {}", e);
                    }
                }
                Err(e) => {
                    error!("File watcher channel error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    /// Processes existing files in the projects directory.
    async fn process_existing_files(&mut self) -> Result<()> {
        debug!(
            "Processing existing files in: {}",
            self.config.claude_projects_path.display()
        );

        let walker = walkdir::WalkDir::new(&self.config.claude_projects_path)
            .follow_links(false)
            .max_depth(3); // projects/<project>/<session>.jsonl

        for entry in walker {
            let entry = entry.map_err(|e| {
                SniffError::file_system(&self.config.claude_projects_path, e.into())
            })?;

            if entry.file_type().is_file() {
                let path = entry.path();
                if self.is_session_file(path) {
                    if let Some(event) = self
                        .create_session_event(path, EventKind::Create(CreateKind::File))
                        .await
                    {
                        self.emit_event(event).await;
                    }
                }
            }
        }

        Ok(())
    }

    /// Handles a file system event from the notify library.
    async fn handle_fs_event(&mut self, event: notify::Result<Event>) -> Result<()> {
        let event = event.map_err(SniffError::file_watcher)?;

        debug!("Received FS event: {:?}", event);

        for path in event.paths {
            // Check if this event should be debounced
            if self.should_debounce(&path) {
                continue;
            }

            // Update debounce timestamp
            self.debounce_map.insert(path.clone(), Instant::now());

            // Handle different event types
            match event.kind {
                EventKind::Create(CreateKind::Folder) => {
                    if self.is_project_directory(&path) {
                        self.emit_project_created(&path).await;
                    }
                }
                EventKind::Create(CreateKind::File) | EventKind::Modify(ModifyKind::Data(_)) => {
                    if self.is_session_file(&path) {
                        if let Some(watch_event) =
                            self.create_session_event(&path, event.kind).await
                        {
                            self.emit_event(watch_event).await;
                        }
                    }
                }
                EventKind::Remove(RemoveKind::File) => {
                    if self.is_session_file(&path) {
                        if let Some(watch_event) =
                            self.create_session_event(&path, event.kind).await
                        {
                            self.emit_event(watch_event).await;
                        }
                    }
                }
                EventKind::Remove(RemoveKind::Folder) => {
                    if self.is_project_directory(&path) {
                        self.emit_project_removed(&path).await;
                    }
                }
                _ => {
                    // Ignore other event types
                }
            }
        }

        Ok(())
    }

    /// Checks if an event should be debounced.
    fn should_debounce(&self, path: &Path) -> bool {
        if let Some(&last_time) = self.debounce_map.get(path) {
            last_time.elapsed() < self.config.debounce_duration
        } else {
            false
        }
    }

    /// Checks if a path is a project directory.
    fn is_project_directory(&self, path: &Path) -> bool {
        path.parent() == Some(&self.config.claude_projects_path) && path.is_dir()
    }

    /// Checks if a path is a session file we should watch.
    fn is_session_file(&self, path: &Path) -> bool {
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            self.config
                .watched_extensions
                .contains(&extension.to_string())
        } else {
            false
        }
    }

    /// Creates a session event from a file system event.
    async fn create_session_event(&self, path: &Path, kind: EventKind) -> Option<WatchEvent> {
        // Extract session ID from filename
        let session_id = path.file_stem()?.to_str()?.to_string();

        // Find project path
        let project_path = self.find_project_path(path)?;

        match kind {
            EventKind::Create(CreateKind::File) => Some(WatchEvent::SessionCreated {
                file_path: path.to_path_buf(),
                session_id,
                project_path,
            }),
            EventKind::Modify(ModifyKind::Data(_)) => Some(WatchEvent::SessionModified {
                file_path: path.to_path_buf(),
                session_id,
                project_path,
            }),
            EventKind::Remove(RemoveKind::File) => Some(WatchEvent::SessionRemoved {
                file_path: path.to_path_buf(),
                session_id,
                project_path,
            }),
            _ => None,
        }
    }

    /// Finds the project path for a given session file.
    fn find_project_path(&self, session_path: &Path) -> Option<PathBuf> {
        let mut current = session_path.parent()?;

        // Look for the parent directory within the projects directory
        while let Some(parent) = current.parent() {
            if parent == self.config.claude_projects_path {
                return Some(current.to_path_buf());
            }
            current = parent;
        }

        None
    }

    /// Emits a project created event.
    async fn emit_project_created(&self, project_path: &Path) {
        let project_name = project_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();

        let event = WatchEvent::ProjectCreated {
            project_path: project_path.to_path_buf(),
            project_name,
        };

        self.emit_event(event).await;
    }

    /// Emits a project removed event.
    async fn emit_project_removed(&self, project_path: &Path) {
        let project_name = project_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();

        let event = WatchEvent::ProjectRemoved {
            project_path: project_path.to_path_buf(),
            project_name,
        };

        self.emit_event(event).await;
    }

    /// Emits a watch event.
    async fn emit_event(&self, event: WatchEvent) {
        debug!("Emitting watch event: {:?}", event);

        if let Err(e) = self.event_sender.send(event) {
            warn!("Failed to send watch event: {}", e);
        }
    }
}

/// Utility functions for working with Claude project directories.
pub mod utils {
    use super::*;

    /// Discovers all existing project directories.
    ///
    /// # Errors
    ///
    /// Returns an error if the projects directory cannot be read.
    pub fn discover_projects(projects_path: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
        let projects_path = projects_path.as_ref();

        if !projects_path.exists() {
            return Ok(Vec::new());
        }

        let mut projects = Vec::new();

        for entry in std::fs::read_dir(projects_path)
            .map_err(|e| SniffError::file_system(projects_path, e))?
        {
            let entry = entry.map_err(|e| SniffError::file_system(projects_path, e))?;

            if entry
                .file_type()
                .map_err(|e| SniffError::file_system(projects_path, e))?
                .is_dir()
            {
                projects.push(entry.path());
            }
        }

        Ok(projects)
    }

    /// Discovers all session files in a project directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the project directory cannot be read.
    pub fn discover_sessions(project_path: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
        let project_path = project_path.as_ref();

        let mut sessions = Vec::new();

        for entry in std::fs::read_dir(project_path)
            .map_err(|e| SniffError::file_system(project_path, e))?
        {
            let entry = entry.map_err(|e| SniffError::file_system(project_path, e))?;
            let path = entry.path();

            if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
                sessions.push(path);
            }
        }

        Ok(sessions)
    }

    /// Extracts the project name from a project path.
    pub fn extract_project_name(project_path: impl AsRef<Path>) -> String {
        project_path
            .as_ref()
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_watcher_config_default() {
        let config = WatcherConfig::default();
        assert!(config.claude_projects_path.ends_with(".claude/projects"));
        assert_eq!(config.watched_extensions, vec!["jsonl"]);
        assert!(config.process_existing);
    }

    #[test]
    fn test_is_session_file() {
        let config = WatcherConfig::default();
        let (watcher, _) = ClaudeWatcher::new(config).unwrap();

        assert!(watcher.is_session_file(Path::new("session.jsonl")));
        assert!(!watcher.is_session_file(Path::new("session.txt")));
        assert!(!watcher.is_session_file(Path::new("session")));
    }

    #[test]
    fn test_utils_discover_projects() {
        let temp_dir = TempDir::new().unwrap();
        let projects_path = temp_dir.path();

        // Create some project directories
        fs::create_dir(projects_path.join("project1")).unwrap();
        fs::create_dir(projects_path.join("project2")).unwrap();
        fs::File::create(projects_path.join("not_a_project.txt")).unwrap();

        let projects = utils::discover_projects(projects_path).unwrap();
        assert_eq!(projects.len(), 2);

        let project_names: Vec<_> = projects
            .iter()
            .filter_map(|p| p.file_name()?.to_str())
            .collect();
        assert!(project_names.contains(&"project1"));
        assert!(project_names.contains(&"project2"));
    }

    #[test]
    fn test_utils_discover_sessions() {
        let temp_dir = TempDir::new().unwrap();
        let project_path = temp_dir.path();

        // Create some session files
        fs::File::create(project_path.join("session1.jsonl")).unwrap();
        fs::File::create(project_path.join("session2.jsonl")).unwrap();
        fs::File::create(project_path.join("not_a_session.txt")).unwrap();

        let sessions = utils::discover_sessions(project_path).unwrap();
        assert_eq!(sessions.len(), 2);

        let session_names: Vec<_> = sessions
            .iter()
            .filter_map(|p| p.file_name()?.to_str())
            .collect();
        assert!(session_names.contains(&"session1.jsonl"));
        assert!(session_names.contains(&"session2.jsonl"));
    }

    #[test]
    fn test_utils_extract_project_name() {
        let name = utils::extract_project_name("/path/to/my-project");
        assert_eq!(name, "my-project");

        let name = utils::extract_project_name(Path::new("simple-name"));
        assert_eq!(name, "simple-name");
    }
}
