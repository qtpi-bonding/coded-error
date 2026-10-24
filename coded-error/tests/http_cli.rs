#![cfg(any(feature = "http", feature = "cli"))]
use coded_error::{CodedError, ErrorClass};

#[derive(Debug, thiserror::Error, CodedError)]
#[coded(namespace = "test.http")]
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