use coded_error::CodedError;

#[derive(Debug, thiserror::Error, CodedError)]
#[coded(namespace = "test.ns")]
enum InnerError {
    #[error("inner")]
    #[coded(code = "inner", class = Logical)]
    Inner,
}

#[derive(Debug, thiserror::Error, CodedError)]
#[coded(namespace = "test.ns")]
enum Error {
    #[error(transparent)]
    #[coded(transparent, http = 99)]
    Bad(InnerError),
}

fn main() {}
