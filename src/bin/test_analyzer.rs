// Test binary for the bullshit analyzer

use sniff::{BullshitAnalyzer, Result};
use std::path::Path;

fn main() -> Result<()> {
    println!("🕵️  Testing Sniff - AI Bullshit Detector");
    println!("========================================");

    // Create the analyzer
    let mut analyzer = BullshitAnalyzer::new()?;

    // The analyzer automatically loads default playbooks
    println!("📚 Default playbooks loaded automatically");

    // Test language detection first
    println!("\n🔍 Testing language detection:");
    let rust_file = Path::new("test_bullshit.rs");
    let python_file = Path::new("test_bullshit.py");

    if let Ok(Some(lang)) = analyzer.detect_language(rust_file) {
        println!("✅ Detected {} for {}", lang.name(), rust_file.display());
    } else {
        println!("❌ Failed to detect language for {}", rust_file.display());
    }

    if let Ok(Some(lang)) = analyzer.detect_language(python_file) {
        println!("✅ Detected {} for {}", lang.name(), python_file.display());
    } else {
        println!("❌ Failed to detect language for {}", python_file.display());
    }

    // Test analyzing the Rust file
    println!("\n🦀 Analyzing Rust test file:");
    match analyzer.analyze_file(rust_file) {
        Ok(detections) => {
            println!("Found {} bullshit patterns:", detections.len());
            for detection in &detections {
                println!(
                    "  🚨 {} ({}:{}): {}",
                    detection.severity.emoji(),
                    detection.line_number,
                    detection.column_number,
                    detection.rule_name
                );
                println!("     Code: '{}'", detection.code_snippet);
                println!("     Context: {}", detection.context);
                println!();
            }
        }
        Err(e) => {
            println!("❌ Error analyzing Rust file: {}", e);
        }
    }

    // Test analyzing the Python file
    println!("\n🐍 Analyzing Python test file:");
    match analyzer.analyze_file(python_file) {
        Ok(detections) => {
            println!("Found {} bullshit patterns:", detections.len());
            for detection in &detections {
                println!(
                    "  🚨 {} ({}:{}): {}",
                    detection.severity.emoji(),
                    detection.line_number,
                    detection.column_number,
                    detection.rule_name
                );
                println!("     Code: '{}'", detection.code_snippet);
                println!("     Context: {}", detection.context);
                println!();
            }
        }
        Err(e) => {
            println!("❌ Error analyzing Python file: {}", e);
        }
    }

    // Test analyzing the current source directory
    println!("\n📂 Analyzing src/ directory:");
    match analyzer.analyze_directory(Path::new("src")) {
        Ok(detections) => {
            println!("Found {} bullshit patterns in src/:", detections.len());

            // Group by file for better readability
            let mut by_file = std::collections::HashMap::new();
            for detection in detections {
                by_file
                    .entry(detection.file_path.clone())
                    .or_insert_with(Vec::new)
                    .push(detection);
            }

            for (file_path, file_detections) in by_file {
                if !file_detections.is_empty() {
                    println!("\n  📄 {}:", file_path);
                    for detection in file_detections {
                        println!(
                            "    🚨 {} Line {}: {}",
                            detection.severity.emoji(),
                            detection.line_number,
                            detection.rule_name
                        );
                        println!("       '{}'", detection.code_snippet);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Error analyzing src/ directory: {}", e);
        }
    }

    println!("\n✅ Analysis complete!");
    Ok(())
}
