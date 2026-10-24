use coded_error::CodedError;
#[derive(Debug, thiserror::Error, CodedError)]
#[coded(namespace = "test.ns")]
enum Error { #[error("bad")] #[coded(code = "x", class = Logical, http = 99)] Bad }
fn main() {}
