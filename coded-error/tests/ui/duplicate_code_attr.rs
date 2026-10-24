use coded_error::CodedError;
#[derive(Debug, thiserror::Error, CodedError)]
#[coded(namespace = "test")]
enum Error { #[error("bad")] #[coded(code = "a", code = "b", class = Logical)] Bad }
fn main() {}
