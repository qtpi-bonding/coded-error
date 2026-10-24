use coded_error::CodedError;
#[derive(Debug, CodedError)]
#[coded(namespace = "test")]
struct Error;
fn main() {}
