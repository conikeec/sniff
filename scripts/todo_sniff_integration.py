#!/usr/bin/env python3
"""
Integration script showing how to combine TODO workflow with sniff verification
"""

import subprocess
import json
import sys
from pathlib import Path
from typing import List, Dict, Any, Optional

class SniffTodoWorkflow:
    def __init__(self, project_root: str):
        self.project_root = Path(project_root)
        self.sniff_binary = self.project_root / "target" / "debug" / "sniff"
    
    def run_sniff_analysis(self, files: List[str]) -> Dict[str, Any]:
        """Run sniff analysis on specified files"""
        cmd = [
            str(self.sniff_binary),
            "analyze-files", 
            "--format", "json",
            "--detailed"
        ] + files
        
        try:
            result = subprocess.run(cmd, capture_output=True, text=True, check=True)
            return json.loads(result.stdout)
        except subprocess.CalledProcessError as e:
            print(f"❌ Sniff analysis failed: {e}")
            return {"error": str(e)}
    
    def verify_quality_gate(self, analysis_result: Dict[str, Any], todo: Dict[str, Any]) -> bool:
        """Check if the analysis meets quality requirements"""
        if "error" in analysis_result:
            return False
            
        # Extract quality metrics
        total_issues = analysis_result.get("total_detections", 0)
        critical_issues = analysis_result.get("critical_issues", 0)
        avg_quality = analysis_result.get("average_quality_score", 0)
        
        # Check against thresholds
        min_quality = todo.get("min_quality_score", 80)
        max_critical = todo.get("max_critical_issues", 0)
        
        if critical_issues > max_critical:
            print(f"🚨 {critical_issues} critical issues found (max allowed: {max_critical})")
            return False
            
        if avg_quality < min_quality:
            print(f"📉 Quality score {avg_quality:.1f}% below threshold {min_quality}%")
            return False
            
        print(f"✅ Quality gate passed: {avg_quality:.1f}% quality, {total_issues} total issues")
        return True
    
    def display_sniff_issues(self, analysis_result: Dict[str, Any]):
        """Display sniff issues in a readable format"""
        print("\n🔍 Sniff Analysis Results:")
        print("=" * 50)
        
        for file_result in analysis_result.get("file_results", []):
            file_path = file_result.get("file_path", "unknown")
            detections = file_result.get("detections", [])
            quality = file_result.get("quality_score", 0)
            
            if detections:
                print(f"\n📄 {file_path} (Quality: {quality:.1f}%)")
                for detection in detections[:5]:  # Show first 5 issues
                    severity = detection.get("severity", "unknown")
                    rule = detection.get("rule_name", "unknown") 
                    line = detection.get("line_number", "?")
                    snippet = detection.get("code_snippet", "").strip()
                    
                    emoji = {
                        "critical": "🚨", "high": "⚠️", "medium": "📝", 
                        "low": "💡", "info": "ℹ️"
                    }.get(severity.lower(), "❓")
                    
                    print(f"  {emoji} {rule} (line {line}): {snippet[:60]}...")
                
                if len(detections) > 5:
                    print(f"  ... and {len(detections) - 5} more issues")
    
    def complete_todo_with_verification(self, todo_id: str, todos: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
        """Complete a TODO with sniff verification"""
        
        # Find the TODO
        todo = None
        for t in todos:
            if t["id"] == todo_id:
                todo = t
                break
        
        if not todo:
            print(f"❌ TODO {todo_id} not found")
            return todos
            
        # Check if files need verification
        files = todo.get("files", [])
        if not files:
            # No files to verify, mark complete
            todo["status"] = "completed"
            print(f"✅ TODO {todo_id} completed (no files to verify)")
            return todos
        
        # Run sniff analysis
        print(f"🔍 Running sniff verification on {len(files)} files...")
        analysis = self.run_sniff_analysis(files)
        
        # Check quality gate
        if self.verify_quality_gate(analysis, todo):
            # Passed verification
            todo["status"] = "completed"
            todo["sniff_verified"] = True
            print(f"✅ TODO {todo_id} completed with sniff verification")
        else:
            # Failed verification  
            self.display_sniff_issues(analysis)
            todo["status"] = "needs-revision"
            print(f"🔄 TODO {todo_id} needs revision due to sniff issues")
        
        return todos

# Example usage
def example_enhanced_workflow():
    """Example of enhanced TODO workflow with sniff integration"""
    
    # Initial TODOs with file tracking
    todos = [
        {
            "id": "implement-auth",
            "content": "Implement user authentication system", 
            "status": "todo",
            "priority": "high",
            "files": ["src/auth.rs", "src/middleware/auth.rs"],
            "min_quality_score": 85,
            "max_critical_issues": 0
        },
        {
            "id": "add-validation", 
            "content": "Add input validation",
            "status": "todo",
            "priority": "medium", 
            "files": ["src/validation.rs"],
            "min_quality_score": 80,
            "max_critical_issues": 1
        }
    ]
    
    workflow = SniffTodoWorkflow("/Users/chetanconikee/tulving/sniff")
    
    # Process each TODO
    for todo in todos:
        if todo["status"] == "todo":
            print(f"\n🚀 Starting TODO: {todo['content']}")
            todo["status"] = "in-progress"
            
            # === IMPLEMENTATION WOULD HAPPEN HERE ===
            # (Developer implements the feature)
            
            # Verify with sniff
            todos = workflow.complete_todo_with_verification(todo["id"], todos)
    
    return todos

if __name__ == "__main__":
    # Run example
    result_todos = example_enhanced_workflow()
    
    # Display final status
    print("\n📋 Final TODO Status:")
    print("=" * 30)
    for todo in result_todos:
        status_emoji = {"completed": "✅", "needs-revision": "🔄", "in-progress": "⏳", "todo": "📝"}
        emoji = status_emoji.get(todo["status"], "❓")
        verified = "🔍" if todo.get("sniff_verified") else ""
        print(f"{emoji} {verified} {todo['content']} ({todo['status']})")
