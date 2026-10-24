use coded_error::CodedError;
#[derive(Debug, thiserror::Error, CodedError)]
#[coded(namespace = "test")]
enum Error {
 #[error("a")] #[coded(code = "x", class = Logical)] A,
 #[error("b")] #[coded(code = "x", class = Logical)] B,
}
fn main() {}
