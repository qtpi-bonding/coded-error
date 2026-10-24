use coded_error::CodedError;
#[derive(Debug, thiserror::Error, CodedError)]
enum Error { #[error("bad")] #[coded(code = "x", class = Logical)] Bad }
fn main() {}
