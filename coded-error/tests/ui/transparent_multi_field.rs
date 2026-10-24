use coded_error::CodedError;
#[derive(Debug, thiserror::Error, CodedError)]
#[coded(namespace = "test")]
enum Error {
 #[error("bad")]
 #[coded(transparent)]
 Bad(String, String),
}
fn main() {}
