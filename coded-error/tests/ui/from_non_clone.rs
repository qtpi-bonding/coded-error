use coded_error::CodedError;
#[derive(Debug)]
struct Kind;

impl From<Kind> for coded_error::ErrorClass {
    fn from(_: Kind) -> Self {
        Self::Logical
    }
}

#[derive(Debug, thiserror::Error, CodedError)]
#[coded(namespace = "test.ns")]
enum Error { #[error("bad")] #[coded(code = "x", class = from(kind))] Bad { kind: Kind } }
fn main() {}
