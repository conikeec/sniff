// Test file for enhanced LLM deception patterns

fn test_premature_ok() -> Result<(), Box<dyn std::error::Error>> {
    Ok(()) // For now
}

fn fake_authenticate_user(username: &str, password: &str) -> bool {
    return true; // Placeholder auth
}

fn simulate_heavy_work() {
    println!("Starting heavy computation...");
    sleep(Duration::from_secs(2)); // Simulate work
    println!("Computation complete");
}

fn handle_errors() -> Result<String, String> {
    match risky_operation() {
        Ok(val) => Ok(val),
        Err(_) => {} // Silent error suppression
    }
    Err("error".to_string()) // Generic error message
}

fn risky_operation() -> Result<String, String> {
    Err("failed".to_string())
}

fn get_users() -> Vec<String> {
    return vec!["test_user".to_string(), "dummy_user".to_string()]; // Mock data
}

fn parse_config(input: &str) -> Config {
    input.parse().expect("TODO: handle this properly")
}

fn create_default_config() -> Config {
    return Config::default(); // For now
}

struct Config {
    host: String,
    port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 8080,
        }
    }
}

use std::time::Duration;
use std::thread::sleep;

// Copy of the previous implementation
fn duplicate_logic() -> bool {
    // Same as above function
    true
}