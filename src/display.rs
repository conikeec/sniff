// Copyright (c) 2025 Chetan Conikee <conikee@gmail.com>
// Licensed under the MIT License

//! Enhanced display formatting for bullshit detection results.

#![allow(clippy::format_push_string)]
#![allow(clippy::format_in_format_args)]
#![allow(clippy::unused_self)]
#![allow(clippy::trivially_copy_pass_by_ref)]
#![allow(clippy::match_same_arms)]

use crate::analysis::{BullshitDetection, ContextLines};
use crate::playbook::Severity;
use colored::{Color, Colorize};
use comfy_table::presets::UTF8_FULL;
use comfy_table::{Attribute, Cell, CellAlignment, Color as TableColor, ContentArrangement, Table};
use console::Term;
use std::path::Path;

/// Enhanced formatter for bullshit detection results.
pub struct BullshitDisplayFormatter {
    /// Whether to use colors in output.
    use_colors: bool,
    /// Whether to show context lines.
    show_context: bool,
    /// Terminal instance for width detection.
    term: Term,
}

impl Default for BullshitDisplayFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl BullshitDisplayFormatter {
    /// Creates a new display formatter.
    #[must_use]
    pub fn new() -> Self {
        let term = Term::stdout();
        Self {
            use_colors: term.features().colors_supported(),
            show_context: true,
            term,
        }
    }

    /// Creates a formatter with custom settings.
    #[must_use]
    pub fn with_settings(use_colors: bool, show_context: bool) -> Self {
        Self {
            use_colors,
            show_context,
            term: Term::stdout(),
        }
    }

    /// Formats a single bullshit detection with enhanced display.
    #[must_use]
    pub fn format_detection(&self, detection: &BullshitDetection) -> String {
        let term_width = self.get_current_terminal_width();

        // For narrow terminals, use minimal format
        if term_width < 100 {
            return self.format_detection_minimal(detection);
        }

        let mut output = String::new();

        // Header with severity and rule info
        output.push_str(&self.format_header(detection));
        output.push('\n');

        // File location
        output.push_str(&self.format_location(detection));
        output.push('\n');

        // Code context if available
        if self.show_context {
            if let Some(context) = &detection.context_lines {
                output.push_str(&self.format_code_context(context, detection));
            } else {
                output.push_str(&self.format_simple_snippet(detection));
            }
            output.push('\n');
        }

        // Description and recommendations
        output.push_str(&self.format_description(detection));

        output
    }

    /// Formats a single detection for very narrow terminals.
    fn format_detection_minimal(&self, detection: &BullshitDetection) -> String {
        let severity_icon = self.get_severity_icon(&detection.severity);
        let line_info = format!("L{}", detection.line_number);

        if self.use_colors {
            format!(
                "{} {} {}\n{}\n",
                severity_icon,
                detection.rule_name.bold(),
                line_info.yellow(),
                detection.code_snippet.trim().dimmed()
            )
        } else {
            format!(
                "{} {} {}\n{}\n",
                severity_icon,
                detection.rule_name,
                line_info,
                detection.code_snippet.trim()
            )
        }
    }

    /// Formats the header with severity and rule information.
    fn format_header(&self, detection: &BullshitDetection) -> String {
        let severity_icon = self.get_severity_icon(&detection.severity);
        let severity_color = self.get_severity_color(&detection.severity);

        if self.use_colors {
            format!(
                "{}  {}",
                severity_icon,
                detection.rule_name.color(severity_color).bold()
            )
        } else {
            format!("{}  {}", severity_icon, detection.rule_name)
        }
    }

    /// Formats the file location information.
    fn format_location(&self, detection: &BullshitDetection) -> String {
        let file_name = Path::new(&detection.file_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&detection.file_path);

        if self.use_colors {
            format!(
                "   📁 {} {}:{}:{}",
                "at".dimmed(),
                file_name.cyan(),
                detection.line_number.to_string().yellow(),
                detection.column_number.to_string().yellow()
            )
        } else {
            format!(
                "   📁 at {}:{}:{}",
                file_name, detection.line_number, detection.column_number
            )
        }
    }

    /// Formats code context with line numbers and highlighting.
    fn format_code_context(
        &self,
        context: &ContextLines,
        _detection: &BullshitDetection,
    ) -> String {
        let mut output = String::new();
        let term_width = self.get_current_terminal_width();
        let max_line_num = context.start_line + context.before.len() + 1 + context.after.len();
        let line_num_width = max_line_num.to_string().len();

        // For narrow terminals, use minimal context display
        if term_width < 100 {
            return self.format_minimal_context(context, line_num_width);
        }

        // Create a separator line that fits current terminal width
        let separator_length = term_width.saturating_sub(6).min(80); // Cap at 80 chars
        let separator = if self.use_colors {
            "─".repeat(separator_length).dimmed().to_string()
        } else {
            "─".repeat(separator_length)
        };

        output.push_str(&format!("   ┌{separator}\n"));

        // Before lines
        for (i, line) in context.before.iter().enumerate() {
            let line_num = context.start_line + i;
            output.push_str(&self.format_context_line(line_num, line, false, line_num_width));
        }

        // Target line (highlighted)
        let target_line_num = context.start_line + context.before.len();
        output.push_str(&self.format_context_line(
            target_line_num,
            &context.target,
            true,
            line_num_width,
        ));

        // After lines
        for (i, line) in context.after.iter().enumerate() {
            let line_num = target_line_num + 1 + i;
            output.push_str(&self.format_context_line(line_num, line, false, line_num_width));
        }

        output.push_str(&format!("   └{separator}"));
        output
    }

    /// Formats a single line in the code context.
    fn format_context_line(
        &self,
        line_num: usize,
        line: &str,
        is_target: bool,
        line_num_width: usize,
    ) -> String {
        let trimmed_line = line.trim_end();
        let line_num_str = format!("{line_num:line_num_width$}");

        if self.use_colors {
            if is_target {
                format!(
                    "   │ {} │ {}\n",
                    line_num_str.red().bold(),
                    trimmed_line.on_red().white().bold()
                )
            } else {
                format!(
                    "   │ {} │ {}\n",
                    line_num_str.dimmed(),
                    trimmed_line.dimmed()
                )
            }
        } else {
            format!("   │ {line_num_str} │ {trimmed_line}\n")
        }
    }

    /// Formats a simple code snippet without context.
    fn format_simple_snippet(&self, detection: &BullshitDetection) -> String {
        let trimmed = detection.code_snippet.trim();
        if self.use_colors {
            format!("   💻 {}", trimmed.yellow())
        } else {
            format!("   💻 {trimmed}")
        }
    }

    /// Formats the description and any recommendations.
    fn format_description(&self, detection: &BullshitDetection) -> String {
        let mut output = String::new();

        if self.use_colors {
            output.push_str(&format!("   📝 {}\n", detection.description.white()));
        } else {
            output.push_str(&format!("   📝 {}\n", detection.description));
        }

        // Add performance impact if available
        if let Some(impact) = &detection.performance_impact {
            output.push_str(&format!("   ⚡ Impact: {}\n", impact.description));
            for recommendation in &impact.recommendations {
                if self.use_colors {
                    output.push_str(&format!("      💡 {}\n", recommendation.green()));
                } else {
                    output.push_str(&format!("      💡 {recommendation}\n"));
                }
            }
        }

        output
    }

    /// Gets the appropriate icon for a severity level.
    fn get_severity_icon(&self, severity: &Severity) -> &'static str {
        match severity {
            Severity::Critical => "🚨",
            Severity::High => "🔴",
            Severity::Medium => "🟡",
            Severity::Low => "🔵",
        }
    }

    /// Gets the appropriate color for a severity level.
    fn get_severity_color(&self, severity: &Severity) -> Color {
        match severity {
            Severity::Critical => Color::Red,
            Severity::High => Color::Red,
            Severity::Medium => Color::Yellow,
            Severity::Low => Color::Blue,
        }
    }

    /// Formats a summary header for multiple detections.
    #[must_use]
    pub fn format_file_header(&self, file_path: &str, detection_count: usize) -> String {
        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(file_path);

        if detection_count == 0 {
            if self.use_colors {
                format!("✅ {} - No issues found", file_name.green().bold())
            } else {
                format!("✅ {file_name} - No issues found")
            }
        } else if self.use_colors {
            format!(
                "💩 {} - {} issue{} found",
                file_name.red().bold(),
                detection_count.to_string().red().bold(),
                if detection_count == 1 { "" } else { "s" }
            )
        } else {
            format!(
                "💩 {} - {} issue{} found",
                file_name,
                detection_count,
                if detection_count == 1 { "" } else { "s" }
            )
        }
    }

    /// Formats a section separator.
    #[must_use]
    pub fn format_separator(&self) -> String {
        let term_width = self.get_current_terminal_width();
        let separator_length = term_width.saturating_sub(2).min(80); // Cap at reasonable width

        if self.use_colors {
            "═".repeat(separator_length).dimmed().to_string()
        } else {
            "═".repeat(separator_length)
        }
    }

    /// Gets the current terminal width, handling dynamic resize.
    fn get_current_terminal_width(&self) -> usize {
        self.term.size().1 as usize
    }

    /// Formats minimal context for narrow terminals.
    fn format_minimal_context(&self, context: &ContextLines, line_num_width: usize) -> String {
        let mut output = String::new();

        // Just show the target line with minimal formatting
        let target_line_num = context.start_line + context.before.len();

        if self.use_colors {
            output.push_str(&format!(
                "   {} │ {}\n",
                format!("{target_line_num:line_num_width$}").red().bold(),
                context.target.trim().yellow()
            ));
        } else {
            output.push_str(&format!(
                "   {} │ {}\n",
                format!("{:width$}", target_line_num, width = line_num_width),
                context.target.trim()
            ));
        }

        output
    }

    /// Formats multiple detections adaptively based on terminal width.
    #[must_use]
    pub fn format_detections_adaptive(&self, detections: &[BullshitDetection]) -> String {
        let term_width = self.get_current_terminal_width();

        // Determine layout based on terminal width
        if term_width < 80 {
            self.format_detections_minimal(detections)
        } else if term_width < 120 {
            self.format_detections_compact(detections)
        } else {
            self.format_detections_full(detections)
        }
    }

    /// Minimal format for very narrow terminals (< 60 chars).
    fn format_detections_minimal(&self, detections: &[BullshitDetection]) -> String {
        let mut output = String::new();

        for detection in detections {
            let severity_icon = self.get_severity_icon(&detection.severity);
            let line_info = format!("L{}", detection.line_number);

            if self.use_colors {
                output.push_str(&format!(
                    "{} {} {}\n{}\n\n",
                    severity_icon,
                    detection.rule_name.bold(),
                    line_info.yellow(),
                    detection.code_snippet.trim().dimmed()
                ));
            } else {
                output.push_str(&format!(
                    "{} {} {}\n{}\n\n",
                    severity_icon,
                    detection.rule_name,
                    line_info,
                    detection.code_snippet.trim()
                ));
            }
        }

        output
    }

    /// Compact format for medium terminals (60-100 chars).
    fn format_detections_compact(&self, detections: &[BullshitDetection]) -> String {
        let mut table = Table::new();
        table.set_content_arrangement(ContentArrangement::Dynamic);

        if self.use_colors {
            table.load_preset(UTF8_FULL);
        }

        // Add header
        table.set_header(vec![
            Cell::new("Severity").add_attribute(Attribute::Bold),
            Cell::new("Rule").add_attribute(Attribute::Bold),
            Cell::new("Line").add_attribute(Attribute::Bold),
            Cell::new("Code").add_attribute(Attribute::Bold),
        ]);

        for detection in detections {
            let severity_cell = if self.use_colors {
                Cell::new(format!(
                    "{} {}",
                    self.get_severity_icon(&detection.severity),
                    detection.severity.name()
                ))
                .fg(self.get_table_color(&detection.severity))
            } else {
                Cell::new(format!(
                    "{} {}",
                    self.get_severity_icon(&detection.severity),
                    detection.severity.name()
                ))
            };

            table.add_row(vec![
                severity_cell,
                Cell::new(&detection.rule_name),
                Cell::new(detection.line_number.to_string()).set_alignment(CellAlignment::Right),
                Cell::new(detection.code_snippet.trim()),
            ]);
        }

        table.to_string()
    }

    /// Full format for wide terminals (>= 100 chars).
    fn format_detections_full(&self, detections: &[BullshitDetection]) -> String {
        let mut output = String::new();

        for detection in detections {
            output.push_str(&self.format_detection(detection));
            output.push('\n');
        }

        output
    }

    /// Formats a file summary with adaptive layout.
    #[must_use]
    pub fn format_file_summary_adaptive(
        &self,
        file_path: &str,
        detections: &[BullshitDetection],
    ) -> String {
        let term_width = self.get_current_terminal_width();

        let header = self.format_file_header(file_path, detections.len());

        if detections.is_empty() {
            return header;
        }

        let mut output = String::new();
        output.push_str(&header);
        output.push('\n');

        if term_width < 80 {
            // Minimal: Just show count and most severe
            let most_severe = detections.iter().max_by_key(|d| match d.severity {
                Severity::Critical => 4,
                Severity::High => 3,
                Severity::Medium => 2,
                Severity::Low => 1,
            });

            if let Some(severe) = most_severe {
                output.push_str(&format!(
                    "   {} Most severe: {} (L{})\n",
                    self.get_severity_icon(&severe.severity),
                    severe.rule_name,
                    severe.line_number
                ));
            }
        } else {
            // Show adaptive table
            output.push_str(&self.format_detections_adaptive(detections));
        }

        output
    }

    /// Gets the appropriate table color for a severity level.
    fn get_table_color(&self, severity: &Severity) -> TableColor {
        match severity {
            Severity::Critical => TableColor::Red,
            Severity::High => TableColor::Red,
            Severity::Medium => TableColor::Yellow,
            Severity::Low => TableColor::Blue,
        }
    }

    /// Creates a summary tree view for very narrow terminals.
    #[must_use]
    pub fn format_summary_tree(
        &self,
        file_summaries: &[(String, Vec<BullshitDetection>)],
    ) -> String {
        let mut output = String::new();

        for (i, (file_path, detections)) in file_summaries.iter().enumerate() {
            let is_last = i == file_summaries.len() - 1;
            let tree_char = if is_last { "└── " } else { "├── " };

            let file_name = Path::new(file_path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(file_path);

            if detections.is_empty() {
                if self.use_colors {
                    output.push_str(&format!("{}{} ✅\n", tree_char, file_name.green()));
                } else {
                    output.push_str(&format!("{tree_char}{file_name} ✅\n"));
                }
            } else {
                let critical_count = detections
                    .iter()
                    .filter(|d| matches!(d.severity, Severity::Critical))
                    .count();
                let high_count = detections
                    .iter()
                    .filter(|d| matches!(d.severity, Severity::High))
                    .count();

                if self.use_colors {
                    output.push_str(&format!(
                        "{}{} {} 🚨{} 🔴{}\n",
                        tree_char,
                        file_name.red(),
                        detections.len(),
                        critical_count,
                        high_count
                    ));
                } else {
                    output.push_str(&format!(
                        "{}{} {} 🚨{} 🔴{}\n",
                        tree_char,
                        file_name,
                        detections.len(),
                        critical_count,
                        high_count
                    ));
                }
            }
        }

        output
    }
}
