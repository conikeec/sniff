// Test file for new LLM deception patterns

fn test_premature_ok() -> Result<(), Box<dyn std::error::Error>> {
    // Some setup code
    Ok(()) // For now
}

fn fake_auth() -> bool {
    // Should check credentials properly
    true // Placeholder auth
}

fn simulate_work() {
    println!("Starting work");
    std::thread::sleep(std::time::Duration::from_secs(1)); // Simulate work
    println!("Work done");
}

fn error_handler() -> Result<String, String> {
    match some_operation() {
        Ok(val) => Ok(val),
        Err(_) => {} // Silent suppression
    }
    Ok("default".to_string())
}

fn some_operation() -> Result<String, String> {
    Err("error".to_string()) // Generic error
}

fn get_test_data() -> Vec<i32> {
    return vec![1, 2, 3]; // Mock data
}

fn process_data(data: &str) -> Result<String, String> {
    data.parse().expect("TODO: handle this error properly")
}

fn get_config() -> Config {
    Config::default() // For now
}

struct Config {
    port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self { port: 8080 }
    }
}

// Same as above function
fn another_test_function() {
    // Duplicate logic - should refactor
    println!("test");
}