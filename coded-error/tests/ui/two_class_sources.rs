use coded_error::CodedError;
fn f(_: &Error) -> coded_error::ErrorClass { coded_error::ErrorClass::Unknown }
#[derive(Debug, thiserror::Error, CodedError)]
#[coded(namespace = "test")]
enum Error {
    #[error("bad")]
    #[coded(code = "x", class = Logical, class_with = f)]
    Bad,
}
fn main() {}
