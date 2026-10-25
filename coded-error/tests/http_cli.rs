#![cfg(any(feature = "http", feature = "cli"))]
use coded_error::CodedError;
#[cfg(feature = "http")]
use coded_error::ErrorClass;

#[derive(Debug, thiserror::Error, CodedError)]
#[coded(namespace = "test.http")]
#[allow(dead_code)]
enum E {
    #[error("bad")]
    #[coded(code = "bad", class = Logical)]
    Bad,
    #[error("missing")]
    #[coded(code = "missing", class = Logical, http = 404)]
    Missing,
    #[error("down")]
    #[coded(code = "down", class = Transient)]
    Down,
    #[error("bug")]
    #[coded(code = "bug", class = Infrastructure)]
    Bug,
}

#[cfg(feature = "http")]
#[test]
fn http_defaults_and_override() {
    use coded_error::http::to_http;
    assert_eq!(to_http(&E::Bad, "s").0, 422);
    assert_eq!(to_http(&E::Missing, "s").0, 404);
    assert_eq!(to_http(&E::Down, "s").0, 503);
    let (code, body) = to_http(&E::Bug, "s");
    assert_eq!((code, body.class), (500, ErrorClass::Infrastructure));
    assert_eq!(
        body.fingerprint,
        coded_error::fingerprint("test.http.bug", ErrorClass::Infrastructure, "s")
    );
}

#[cfg(all(feature = "http", feature = "anyhow"))]
#[test]
fn http_anyhow_forms() {
    use coded_error::http::to_http_anyhow;
    use coded_error::into_anyhow;

    let missing = to_http_anyhow(&into_anyhow(E::Missing).context("c"), "s");
    assert_eq!(missing.0, 404);
    assert_eq!(missing.1.code, "test.http.missing");

    let unknown = to_http_anyhow(&anyhow::anyhow!("boom"), "s");
    assert_eq!(unknown.0, 500);
    assert_eq!(unknown.1.code, "unknown");
    assert_eq!(unknown.1.class, ErrorClass::Unknown);
    assert_eq!(
        unknown.1.fingerprint,
        coded_error::fingerprint("unknown", ErrorClass::Unknown, "s")
    );
}

#[cfg(all(feature = "http", feature = "anyhow"))]
#[test]
fn http_redacted_form_keeps_only_a_logical_typed_message() {
    use coded_error::http::{to_http_anyhow, to_http_anyhow_redacted};
    use coded_error::into_anyhow;
    const FIXED: &str = "request failed";

    let (status, info, message) =
        to_http_anyhow_redacted(&into_anyhow(E::Missing).context("tenant t9"), "s", FIXED);
    assert_eq!((status, info.code.as_str()), (404, "test.http.missing"));
    assert_eq!(message, "missing");

    let attached = coded_error::attach(anyhow::anyhow!("10.0.0.5 refused"), &E::Bad);
    for err in [
        into_anyhow(E::Bug).context("db at 10.0.0.5"),
        into_anyhow(E::Down),
        anyhow::anyhow!("10.0.0.5 refused"),
        attached,
        coded_error::from_remote(Some("peer.x"), Some(ErrorClass::Logical), "10.0.0.5".into()),
    ] {
        let (status, info, message) = to_http_anyhow_redacted(&err, "s", FIXED);
        assert_eq!((status, info), to_http_anyhow(&err, "s"));
        assert_eq!(message, FIXED);
    }
}

#[cfg(feature = "cli")]
#[test]
fn cli_sysexits_and_repo_table() {
    use coded_error::cli::{exit_code, exit_code_with, ExitTable};
    assert_eq!(
        [exit_code(&E::Bad), exit_code(&E::Bug), exit_code(&E::Down)],
        [65, 69, 75]
    );
    // archegraph: every error is 1.
    assert_eq!(exit_code_with(&E::Down, &ExitTable::all(1)), 1);
}

#[cfg(all(feature = "cli", feature = "anyhow"))]
#[test]
fn cli_anyhow_forms() {
    use coded_error::cli::{exit_code_anyhow, exit_code_anyhow_with, ExitTable};
    let err = coded_error::into_anyhow(E::Bad).context("ctx");
    assert_eq!(exit_code_anyhow(&err), 65);
    assert_eq!(exit_code_anyhow_with(&err, &ExitTable::all(1)), 1);
    assert_eq!(exit_code_anyhow(&anyhow::anyhow!("boom")), 70); // unknown -> SYSEXITS unknown
}
