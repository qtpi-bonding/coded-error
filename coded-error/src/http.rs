use crate::{CodedError, ErrorClass, ErrorInfo, Facts};

fn default_status(class: ErrorClass) -> u16 {
    match class {
        ErrorClass::Logical => 422,
        ErrorClass::Infrastructure => 500,
        ErrorClass::Transient => 503,
        ErrorClass::Unknown => 500,
    }
}

fn from_facts(facts: Facts, source: &str) -> (u16, ErrorInfo) {
    let status = facts
        .overrides
        .http
        .unwrap_or_else(|| default_status(facts.class));
    (status, ErrorInfo::from_facts(&facts, source))
}

/// Status code and JSON body (`ErrorInfo`). Framework-neutral: axum callers
/// wrap it in `(StatusCode::from_u16(s), Json(body))`.
pub fn to_http<E: CodedError + ?Sized>(e: &E, source: &str) -> (u16, ErrorInfo) {
    from_facts(Facts::of(e), source)
}

#[cfg(feature = "anyhow")]
pub fn to_http_anyhow(err: &anyhow::Error, source: &str) -> (u16, ErrorInfo) {
    from_facts(crate::facts_anyhow(err), source)
}

/// As `to_http_anyhow`, plus a message safe to show an untrusted caller,
/// chosen as `grpc::to_status_anyhow_redacted` chooses its message.
/// `ErrorInfo` stays the proto record; the caller decides where the message
/// goes in its body.
#[cfg(feature = "anyhow")]
pub fn to_http_anyhow_redacted(
    err: &anyhow::Error,
    source: &str,
    fixed: &str,
) -> (u16, ErrorInfo, String) {
    let (status, info) = to_http_anyhow(err, source);
    (status, info, crate::carry::redacted_message(err, fixed))
}
