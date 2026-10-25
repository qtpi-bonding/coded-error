#![cfg(feature = "mcp")]
use coded_error::mcp::to_mcp_error;
use coded_error::{CodedError, ErrorInfo};
use rmcp::model::ErrorCode;

#[derive(Debug, thiserror::Error, CodedError)]
#[coded(namespace = "test.mcp")]
enum E {
    #[error("bad")]
    #[coded(code = "bad", class = Logical)]
    Bad,
    #[error("missing")]
    #[coded(code = "missing", class = Logical, mcp = ResourceNotFound)]
    Missing,
    #[error("bug")]
    #[coded(code = "bug", class = Infrastructure)]
    Bug,
}

#[test]
fn class_defaults_override_message_and_data() {
    assert_eq!(to_mcp_error(&E::Bad, "s").code, ErrorCode::INVALID_PARAMS);
    assert_eq!(
        to_mcp_error(&E::Missing, "s").code,
        ErrorCode::RESOURCE_NOT_FOUND
    );
    assert_eq!(to_mcp_error(&E::Bug, "s").code, ErrorCode::INTERNAL_ERROR);
    let e = to_mcp_error(&E::Missing, "s");
    assert_eq!(e.message, "missing");
    assert_eq!(
        e.data,
        Some(serde_json::to_value(ErrorInfo::of(&E::Missing, "s")).unwrap())
    );
}

#[cfg(feature = "anyhow")]
#[test]
fn anyhow_form_message_is_the_whole_chain() {
    use coded_error::mcp::to_mcp_error_anyhow;
    let err = coded_error::into_anyhow(E::Bad)
        .context("loading")
        .context("handling call");
    let e = to_mcp_error_anyhow(&err, "s");
    assert_eq!(e.message, "handling call: loading: bad");
    assert_eq!(e.code, ErrorCode::INVALID_PARAMS);
    let u = to_mcp_error_anyhow(&anyhow::anyhow!("boom"), "s");
    assert_eq!(u.code, ErrorCode::INTERNAL_ERROR);
    assert_eq!(u.data.unwrap()["code"], "unknown");
}

#[cfg(feature = "anyhow")]
#[test]
fn redacted_form_keeps_only_a_logical_typed_message() {
    use coded_error::mcp::{to_mcp_error_anyhow, to_mcp_error_anyhow_redacted};
    const FIXED: &str = "request failed";
    let logical = coded_error::into_anyhow(E::Missing).context("tenant t9");
    let e = to_mcp_error_anyhow_redacted(&logical, "s", FIXED);
    assert_eq!(e.message, "missing");
    assert_eq!(e.code, ErrorCode::RESOURCE_NOT_FOUND);

    let attached = coded_error::attach(anyhow::anyhow!("10.0.0.5 refused"), &E::Bad);
    for err in [
        coded_error::into_anyhow(E::Bug).context("db at 10.0.0.5"),
        anyhow::anyhow!("10.0.0.5 refused"),
        attached,
        coded_error::from_remote(
            Some("peer.x"),
            Some(coded_error::ErrorClass::Logical),
            "10.0.0.5".into(),
        ),
    ] {
        let redacted = to_mcp_error_anyhow_redacted(&err, "s", FIXED);
        let plain = to_mcp_error_anyhow(&err, "s");
        assert_eq!(redacted.message, FIXED);
        assert_eq!((redacted.code, redacted.data), (plain.code, plain.data));
    }
}
