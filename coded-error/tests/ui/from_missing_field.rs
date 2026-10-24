use coded_error::CodedError;
#[derive(Debug, thiserror::Error, CodedError)]
#[coded(namespace = "test")]
enum Error { #[error("bad")] #[coded(code = "x", class = from(nope))] Bad { value: String } }
fn main() {}
