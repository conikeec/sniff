use sniff::{BullshitAnalyzer, Result};
use std::path::Path;

fn main() -> Result<()> {
    println\!("Testing direct analyzer on clean_test.rs");
    
    let mut analyzer = BullshitAnalyzer::new_with_learned_patterns(".")?;
    
    let test_file = Path::new("clean_test.rs");
    let detections = analyzer.analyze_file(test_file)?;
    
    println\!("Found {} patterns:", detections.len());
    for detection in &detections {
        println\!("  - {}: {}", detection.rule_name, detection.code_snippet);
    }
    
    Ok(())
}
EOF < /dev/null