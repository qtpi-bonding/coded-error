use coded_error::CodedError;
#[derive(Debug, thiserror::Error, CodedError)]
#[coded(namespace = "test.ns")]
enum Error { #[error("bad")] #[coded(code = "x", class = Logical, acp = -32100)] Bad }
fn main() {}
