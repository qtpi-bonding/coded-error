use coded_error::CodedError;
#[derive(Debug, thiserror::Error, CodedError)]
#[coded(namespace = "test")]
enum Error { #[error(transparent)] #[coded(transparent)] Bad(String) }
fn main() {}
