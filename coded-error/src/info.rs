use crate::{fingerprint, CodedError, ErrorClass, Facts};

/// The portable error record. Its JSON is identical to the proto3-JSON of
/// `coded_error.v1.ErrorInfo` because `class` is a string there too.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ErrorInfo {
    pub code: String,
    pub class: ErrorClass,
    pub fingerprint: String,
}

impl ErrorInfo {
    pub fn from_facts(facts: &Facts, source: &str) -> ErrorInfo {
        ErrorInfo {
            code: facts.code.to_owned(),
            class: facts.class,
            fingerprint: fingerprint(facts.code, facts.class, source),
        }
    }

    pub fn of<E: CodedError + ?Sized>(e: &E, source: &str) -> ErrorInfo {
        ErrorInfo::from_facts(&Facts::of(e), source)
    }
}
