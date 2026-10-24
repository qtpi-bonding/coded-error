#![cfg(feature = "grpc")]
use coded_error::grpc::{to_status, CodedStatus};
use coded_error::{fingerprint, CodedError, ErrorClass};
use tonic::Code;

#[derive(Debug, thiserror::Error, CodedError)]
#[coded(namespace = "test.rpc")]
enum E {
    #[error("missing name")]
    #[coded(code = "missing_name", class = Logical)]
    Missing,
    #[error("no such action")]
    #[coded(code = "not_found", class = Logical, grpc = NotFound)]
    NotFound,
    #[error("db down")]
    #[coded(code = "db_down", class = Transient)]
    Down,
}

#[test]
fn class_defaults_and_override() {
    assert_eq!(to_status(&E::Missing, "m").code(), Code::InvalidArgument);
    assert_eq!(to_status(&E::NotFound, "m").code(), Code::NotFound);
    assert_eq!(to_status(&E::Down, "m").code(), Code::Unavailable);
}

#[test]
fn metadata_round_trips() {
    let s = to_status(&E::NotFound, "grorg_server::GetAction");
    assert_eq!(s.message(), "no such action");
    let c = CodedStatus::from(&s);
    assert_eq!(c.code.as_deref(), Some("test.rpc.not_found"));
    assert_eq!(c.class, Some(ErrorClass::Logical));
    assert_eq!(
        c.fingerprint.as_deref(),
        Some(fingerprint("test.rpc.not_found", ErrorClass::Logical, "grorg_server::GetAction").as_str())
    );
}

#[cfg(feature = "anyhow")]
#[test]
fn anyhow_form_and_unknown_fallback() {
    use coded_error::grpc::to_status_anyhow;
    use coded_error::into_anyhow;
    let s = to_status_anyhow(&into_anyhow(E::NotFound), "m");
    assert_eq!(s.code(), Code::NotFound);
    let u = to_status_anyhow(&anyhow::anyhow!("boom"), "m");
    assert_eq!(u.code(), Code::Unknown);
    assert_eq!(CodedStatus::from(&u).code.as_deref(), Some("unknown"));
}

#[cfg(feature = "anyhow")]
#[test]
fn anyhow_message_is_the_whole_chain() {
    use coded_error::grpc::to_status_anyhow;
    use coded_error::into_anyhow;
    let err = into_anyhow(E::NotFound).context("loading action").context("handling GetAction");
    assert_eq!(to_status_anyhow(&err, "m").message(), "handling GetAction: loading action: no such action");
}