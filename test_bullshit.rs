// Test file with various bullshit patterns for the sniff analyzer

fn complete_authentication() -> Result<User, Error> {
    // TODO: implement OAuth flow
    unimplemented!()
}

fn validate_user(user: &User) -> bool {
    // FIXME: add proper validation
    true
}

fn process_payment(amount: f64) -> PaymentResult {
    panic!("TODO: integrate with payment provider")
}

fn get_user_data(id: u64) -> User {
    let result = fetch_user(id);
    result.unwrap() // This should be handled better
}

fn another_incomplete_function() {
    // XXX: This is a hack
    unimplemented!()
}

struct User {
    id: u64,
    name: String,
}

struct PaymentResult {
    success: bool,
}

impl User {
    fn new(id: u64, name: String) -> Self {
        // TODO: add validation
        Self { id, name }
    }
}

// More bullshit patterns
fn lazy_implementation() {
    unimplemented!() // Another unimplemented
}

fn fetch_user(id: u64) -> Result<User, Error> {
    // HACK: hardcoded for now
    Ok(User { id, name: "test".to_string() })
}