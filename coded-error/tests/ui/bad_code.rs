use coded_error::CodedError;
#[derive(Debug, thiserror::Error, CodedError)]
#[coded(namespace = "test.Bad")]
enum Error { #[error("bad")] #[coded(code = "x", class = Logical)] Bad }
fn main() {}
