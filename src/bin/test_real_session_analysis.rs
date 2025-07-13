// Test binary for REAL session analysis using only verified working components

#![allow(clippy::manual_flatten)]

use sniff::{Result, SimpleSessionAnalyzer};
use std::path::Path;

fn main() -> Result<()> {
    println!("🕵️  Testing REAL Session Analysis (No Bullshit)");
    println!("==============================================");

    // Create the simple, working session analyzer
    println!("🔧 Creating simple session analyzer...");
    let mut analyzer = SimpleSessionAnalyzer::new()?;
    println!("✅ Analyzer created with verified components:");
    println!("   • JSONL parser for session files");
    println!("   • Operation extractor for file operations");
    println!("   • Bullshit detector with working rules");

    // Test with our mock files first
    println!("\n🧪 Testing with mock files:");
    test_with_mock_files(&mut analyzer)?;

    // Try to find real Claude Code sessions
    println!("\n🔍 Looking for real Claude Code sessions...");
    test_with_real_sessions(&mut analyzer)?;

    println!("\n✅ Real session analysis test complete!");

    Ok(())
}

fn test_with_mock_files(analyzer: &mut SimpleSessionAnalyzer) -> Result<()> {
    // Create a simple mock session JSONL file
    let session_content = create_mock_session_content();
    let session_file = std::env::temp_dir().join("test_session.jsonl");

    std::fs::write(&session_file, session_content)
        .map_err(|e| sniff::SniffError::file_system(&session_file, e))?;

    // Also create the mock files referenced in the session
    create_mock_modified_files()?;

    println!("📄 Analyzing mock session...");
    match analyzer.analyze_session(&session_file) {
        Ok(analysis) => {
            println!("✅ Mock session analysis successful!");
            print_analysis_results(&analysis);
        }
        Err(e) => {
            println!("❌ Mock session analysis failed: {}", e);
        }
    }

    // Clean up
    let _ = std::fs::remove_file(&session_file);
    let _ = std::fs::remove_file("test_fibonacci.rs");
    let _ = std::fs::remove_file("test_helper.py");

    Ok(())
}

fn create_mock_session_content() -> String {
    // Real JSONL format that matches Claude Code sessions
    r#"{"uuid":"msg-001","timestamp":"2025-01-13T10:00:00Z","type":"user","content":"Create a fibonacci function","sessionId":"test-session-001","cwd":"/tmp","version":"1.0.0","isSidechain":false,"userType":"external","parentUuid":null}
{"uuid":"msg-002","timestamp":"2025-01-13T10:01:00Z","type":"assistant","content":[{"type":"tool_use","id":"tool-001","name":"Write","input":{"file_path":"test_fibonacci.rs","content":"fn fibonacci(n: u32) -> u64 {\n    // TODO: implement fibonacci calculation\n    unimplemented!()\n}\n\nfn main() {\n    let result = fibonacci(10).unwrap(); // This should be handled better\n    println!(\"Fibonacci(10) = {}\", result);\n}"}}],"sessionId":"test-session-001","cwd":"/tmp","version":"1.0.0","isSidechain":false,"userType":"external","parentUuid":"msg-001"}
{"uuid":"msg-003","timestamp":"2025-01-13T10:02:00Z","type":"user","content":[{"tool_use_id":"tool-001","type":"tool_result","content":"File written successfully"}],"sessionId":"test-session-001","cwd":"/tmp","version":"1.0.0","isSidechain":false,"userType":"external","parentUuid":"msg-002"}
{"uuid":"msg-004","timestamp":"2025-01-13T10:03:00Z","type":"assistant","content":[{"type":"tool_use","id":"tool-002","name":"Write","input":{"file_path":"test_helper.py","content":"def helper_function():\n    # FIXME: add actual implementation\n    pass\n\ndef process_data(data):\n    raise NotImplementedError(\"TODO: implement data processing\")\n\ndef main():\n    helper_function()\n    print(\"Done\")"}}],"sessionId":"test-session-001","cwd":"/tmp","version":"1.0.0","isSidechain":false,"userType":"external","parentUuid":"msg-003"}"#.to_string()
}

fn create_mock_modified_files() -> Result<()> {
    // Create the Rust file with bullshit patterns
    let rust_content = r#"fn fibonacci(n: u32) -> u64 {
    // TODO: implement fibonacci calculation
    unimplemented!()
}

fn main() {
    let result = fibonacci(10).unwrap(); // This should be handled better
    println!("Fibonacci(10) = {}", result);
}"#;

    std::fs::write("test_fibonacci.rs", rust_content)
        .map_err(|e| sniff::SniffError::file_system(Path::new("test_fibonacci.rs"), e))?;

    // Create the Python file with bullshit patterns
    let python_content = r#"def helper_function():
    # FIXME: add actual implementation
    pass

def process_data(data):
    raise NotImplementedError("TODO: implement data processing")

def main():
    helper_function()
    print("Done")"#;

    std::fs::write("test_helper.py", python_content)
        .map_err(|e| sniff::SniffError::file_system(Path::new("test_helper.py"), e))?;

    Ok(())
}

fn test_with_real_sessions(analyzer: &mut SimpleSessionAnalyzer) -> Result<()> {
    // Look for real Claude Code projects
    let home_dir = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let claude_projects_dir = home_dir.join(".claude").join("projects");

    if !claude_projects_dir.exists() {
        println!(
            "📁 No Claude Code projects directory found at {}",
            claude_projects_dir.display()
        );
        println!("   This is expected if Claude Code hasn't been used on this system");
        return Ok(());
    }

    println!(
        "📁 Found Claude Code projects directory: {}",
        claude_projects_dir.display()
    );

    // Look for project subdirectories
    if let Ok(entries) = std::fs::read_dir(&claude_projects_dir) {
        let mut project_count = 0;
        let mut session_count = 0;

        for entry in entries {
            if let Ok(entry) = entry {
                let project_path = entry.path();
                if project_path.is_dir() {
                    project_count += 1;
                    println!(
                        "🏗️  Analyzing project: {}",
                        project_path.file_name().unwrap().to_string_lossy()
                    );

                    match analyzer.analyze_project_directory(&project_path) {
                        Ok(analyses) => {
                            session_count += analyses.len();
                            for analysis in analyses {
                                println!(
                                    "   📄 Session {}: {} files, {} patterns, {:.1}% quality",
                                    analysis.session_id,
                                    analysis.metrics.files_modified,
                                    analysis.metrics.total_bullshit_patterns,
                                    analysis.metrics.quality_score
                                );
                            }
                        }
                        Err(e) => {
                            println!("   ❌ Failed to analyze project: {}", e);
                        }
                    }
                }
            }
        }

        if project_count == 0 {
            println!("📁 No project subdirectories found");
        } else {
            println!(
                "✅ Analyzed {} projects with {} sessions total",
                project_count, session_count
            );
        }
    }

    Ok(())
}

fn print_analysis_results(analysis: &sniff::SimpleSessionAnalysis) {
    println!("\n📊 Analysis Results for Session: {}", analysis.session_id);
    println!("   📁 Files modified: {}", analysis.metrics.files_modified);
    println!("   🔍 File operations: {}", analysis.file_operations.len());
    println!(
        "   💩 Bullshit patterns: {}",
        analysis.metrics.total_bullshit_patterns
    );
    println!(
        "   🚨 Critical patterns: {}",
        analysis.metrics.critical_patterns
    );
    println!(
        "   📈 Quality score: {:.1}%",
        analysis.metrics.quality_score
    );

    if !analysis.modified_files.is_empty() {
        println!("\n📄 Modified Files:");
        for file in &analysis.modified_files {
            println!("   • {}", file);
        }
    }

    if !analysis.bullshit_detections.is_empty() {
        println!("\n💩 Bullshit Patterns Detected:");
        for detection in &analysis.bullshit_detections {
            println!(
                "   {} {} ({}:{}): {}",
                detection.severity.emoji(),
                detection.rule_name,
                detection.file_path,
                detection.line_number,
                detection.code_snippet.trim()
            );
        }
    }

    if !analysis.recommendations.is_empty() {
        println!("\n🎯 Recommendations:");
        for rec in &analysis.recommendations {
            println!("   {}", rec);
        }
    }
}
