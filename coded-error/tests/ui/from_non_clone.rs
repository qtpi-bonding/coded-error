use coded_error::CodedError;
struct Kind;
#[derive(Debug, thiserror::Error, CodedError)]
#[coded(namespace = "test")]
enum Error { #[error("bad")] #[coded(code = "x", class = from(kind))] Bad { kind: Kind } }
fn main() {}
