// Copyright (c) 2025 Chetan Conikee <conikee@gmail.com>
// Licensed under the MIT License

//! Progress indicators and user feedback for CLI operations.
//!
//! This module provides entertaining progress indicators and clear completion
//! messages to keep users informed during long-running operations.

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// A progress indicator that shows spinning animation with fun messages.
pub struct ProgressIndicator {
    /// Whether the progress indicator is currently running.
    running: Arc<AtomicBool>,
    /// Handle to the background thread showing progress.
    handle: Option<thread::JoinHandle<()>>,
    /// Start time for duration calculation.
    start_time: Instant,
}

/// Fun messages to display during different operations.
pub struct ProgressMessages;

impl ProgressMessages {
    /// Messages for scanning operations.
    pub const SCANNING: &'static [&'static str] = &[
        "🔍 Analyzing your Claude conversations...",
        "📚 Reading through your session histories...",
        "🧠 Extracting thinking patterns...",
        "⚡ Building knowledge trees...",
        "🔧 Categorizing tool operations...",
        "📈 Computing session metrics...",
        "🎯 Indexing searchable content...",
        "🌟 Discovering conversation insights...",
        "⚙️  Processing message flows...",
        "🚀 Optimizing data structures...",
    ];

    /// Messages for database operations.
    pub const DATABASE: &'static [&'static str] = &[
        "💾 Updating database indexes...",
        "🔐 Securing data with Blake3 hashes...",
        "📊 Computing storage statistics...",
        "⚡ Optimizing query performance...",
        "🎪 Juggling Merkle tree nodes...",
    ];

    /// Messages for completion.
    pub const COMPLETION: &'static [&'static str] = &[
        "✨ Analysis complete!",
        "🎉 Mission accomplished!",
        "⭐ All done!",
        "🚀 Processing finished!",
        "🌟 Success!",
    ];
}

impl ProgressIndicator {
    /// Creates a new progress indicator with the given operation type.
    #[must_use]
    pub fn new(operation: &str) -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        let operation = operation.to_string();

        let handle = thread::spawn(move || {
            let messages = match operation.as_str() {
                "scan" => ProgressMessages::SCANNING,
                "database" => ProgressMessages::DATABASE,
                _ => ProgressMessages::SCANNING,
            };

            let spinners = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let mut spinner_idx = 0;
            let mut message_idx = 0;
            let mut last_message_change = Instant::now();

            while running_clone.load(Ordering::Relaxed) {
                // Change message every 3 seconds
                if last_message_change.elapsed() >= Duration::from_secs(3) {
                    message_idx = (message_idx + 1) % messages.len();
                    last_message_change = Instant::now();
                }

                print!("\r{} {}", spinners[spinner_idx], messages[message_idx]);
                io::stdout().flush().unwrap_or(());

                spinner_idx = (spinner_idx + 1) % spinners.len();
                thread::sleep(Duration::from_millis(100));
            }

            // Clear the line when done
            print!("\r{:50}\r", "");
            io::stdout().flush().unwrap_or(());
        });

        Self {
            running,
            handle: Some(handle),
            start_time: Instant::now(),
        }
    }

    /// Updates the progress with a specific message.
    pub fn update(&self, message: &str) {
        print!("\r⚡ {}{:30}\r", message, "");
        io::stdout().flush().unwrap_or(());
    }

    /// Stops the progress indicator and shows completion message.
    pub fn finish(mut self, success_message: Option<&str>) {
        self.running.store(false, Ordering::Relaxed);

        if let Some(handle) = self.handle.take() {
            handle.join().unwrap_or(());
        }

        let duration = self.start_time.elapsed();
        let completion_msg = success_message.unwrap_or(ProgressMessages::COMPLETION[0]);

        println!(
            "{} (completed in {:.2}s)",
            completion_msg,
            duration.as_secs_f64()
        );
    }

    /// Stops the progress indicator and shows an error message.
    pub fn finish_with_error(mut self, error_message: &str) {
        self.running.store(false, Ordering::Relaxed);

        if let Some(handle) = self.handle.take() {
            handle.join().unwrap_or(());
        }

        let duration = self.start_time.elapsed();
        println!(
            "❌ {} (failed after {:.2}s)",
            error_message,
            duration.as_secs_f64()
        );
    }
}

impl Drop for ProgressIndicator {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            handle.join().unwrap_or(());
        }
    }
}

/// A simple progress bar for operations with known total work.
pub struct ProgressBar {
    /// Total number of items to process.
    total: usize,
    /// Number of items completed.
    completed: usize,
    /// Start time for duration calculation.
    start_time: Instant,
}

impl ProgressBar {
    /// Creates a new progress bar.
    #[must_use]
    pub fn new(total: usize) -> Self {
        Self {
            total,
            completed: 0,
            start_time: Instant::now(),
        }
    }

    /// Updates the progress bar with current completion count.
    pub fn update(&mut self, completed: usize, current_item: &str) {
        self.completed = completed;
        let percentage = if self.total > 0 {
            (completed as f64 / self.total as f64 * 100.0) as u8
        } else {
            0
        };

        let bar_width = 30;
        let filled = (percentage as usize * bar_width) / 100;
        let bar = "█".repeat(filled) + &"░".repeat(bar_width - filled);

        print!(
            "\r[{}] {}% ({}/{}) - {}",
            bar, percentage, completed, self.total, current_item
        );
        io::stdout().flush().unwrap_or(());
    }

    /// Finishes the progress bar with a completion message.
    pub fn finish(&self, message: &str) {
        let duration = self.start_time.elapsed();
        println!(
            "\n✅ {} (processed {} items in {:.2}s)",
            message,
            self.completed,
            duration.as_secs_f64()
        );
    }
}

/// Shows a simple status message.
pub fn show_status(message: &str) {
    println!("📋 {message}");
}

/// Shows a success message.
pub fn show_success(message: &str) {
    println!("✅ {message}");
}

/// Shows an error message.
pub fn show_error(message: &str) {
    println!("❌ {message}");
}

/// Shows a warning message.
pub fn show_warning(message: &str) {
    println!("⚠️  {message}");
}

/// Shows an info message.
pub fn show_info(message: &str) {
    println!("ℹ️  {message}");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_progress_indicator_creation() {
        let progress = ProgressIndicator::new("scan");
        assert!(progress.running.load(Ordering::Relaxed));

        // Let it run briefly to test the spinner
        thread::sleep(Duration::from_millis(200));

        progress.finish(Some("Test completed"));
    }

    #[test]
    fn test_progress_bar() {
        let mut bar = ProgressBar::new(10);
        bar.update(5, "Processing item 5");
        assert_eq!(bar.completed, 5);
        bar.finish("Test completed");
    }

    #[test]
    fn test_message_functions() {
        // These functions should not panic
        show_status("Test status");
        show_success("Test success");
        show_error("Test error");
        show_warning("Test warning");
        show_info("Test info");
    }
}
