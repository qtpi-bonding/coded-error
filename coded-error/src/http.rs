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
    let status = facts.overrides.http.unwrap_or_else(|| default_status(facts.class));
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