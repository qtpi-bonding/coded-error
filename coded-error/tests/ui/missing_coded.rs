use coded_error::CodedError;

#[derive(Debug, thiserror::Error, CodedError)]
#[coded(namespace = "test")]
enum Error {
    #[error("bad")]
    Bad,
}

fn main() {}
