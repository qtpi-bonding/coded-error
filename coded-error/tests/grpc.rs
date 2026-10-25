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
    #[error("internal failure")]
    #[coded(code = "internal_failure", class = Infrastructure)]
    Internal,
    #[error("unknown failure")]
    #[coded(code = "unknown_failure", class = Unknown)]
    Unknown,
}

#[test]
fn class_defaults_and_override() {
    assert_eq!(to_status(&E::Missing, "m").code(), Code::InvalidArgument);
    assert_eq!(to_status(&E::NotFound, "m").code(), Code::NotFound);
    assert_eq!(to_status(&E::Down, "m").code(), Code::Unavailable);
    assert_eq!(to_status(&E::Internal, "m").code(), Code::Internal);
    assert_eq!(to_status(&E::Unknown, "m").code(), Code::Unknown);
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
        Some(
            fingerprint(
                "test.rpc.not_found",
                ErrorClass::Logical,
                "grorg_server::GetAction"
            )
            .as_str()
        )
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
    assert_eq!(CodedStatus::from(&u).class, Some(ErrorClass::Unknown));
    assert_eq!(
        CodedStatus::from(&u).fingerprint.as_deref(),
        Some(fingerprint("unknown", ErrorClass::Unknown, "m").as_str())
    );
}

#[cfg(feature = "anyhow")]
#[test]
fn anyhow_message_is_the_whole_chain() {
    use coded_error::grpc::to_status_anyhow;
    use coded_error::into_anyhow;
    let err = into_anyhow(E::NotFound)
        .context("loading action")
        .context("handling GetAction");
    assert_eq!(
        to_status_anyhow(&err, "m").message(),
        "handling GetAction: loading action: no such action"
    );
}

#[cfg(feature = "anyhow")]
mod remote {
    use super::*;
    use coded_error::grpc::from_status;
    use coded_error::{facts_anyhow, from_anyhow, CodedError};

    #[test]
    fn status_becomes_coded_anyhow() {
        let s = to_status(&E::NotFound, "server");
        let err = from_status(&s);
        assert_eq!(err.to_string(), "no such action");
        let c = from_anyhow(&err).expect("coded");
        assert_eq!(c.code(), "test.rpc.not_found");
        assert_eq!(c.class(), ErrorClass::Logical);
        // survives context layers and re-encoding on another hop
        let err = err.context("calling GetAction");
        assert_eq!(facts_anyhow(&err).code, "test.rpc.not_found");
        let again = coded_error::grpc::to_status_anyhow(&err, "client");
        assert_eq!(
            CodedStatus::from(&again).code.as_deref(),
            Some("test.rpc.not_found")
        );
    }

    #[test]
    fn uncoded_status_reads_unknown() {
        let err = from_status(&tonic::Status::internal("plain"));
        assert_eq!(err.to_string(), "plain");
        let f = facts_anyhow(&err);
        assert_eq!(f.code, "unknown");
        assert_eq!(f.class, ErrorClass::Unknown);
        assert!(from_anyhow(&err).is_none());
    }

    #[test]
    fn server_unknown_code_and_malformed_code() {
        let s = coded_error::grpc::to_status_anyhow(&anyhow::anyhow!("boom"), "m");
        assert_eq!(facts_anyhow(&from_status(&s)).code, "unknown");
        let mut bad = tonic::Status::internal("x");
        bad.metadata_mut()
            .insert("x-error-code", "Not A Code".parse().unwrap());
        bad.metadata_mut()
            .insert("x-error-class", "transient".parse().unwrap());
        let f = facts_anyhow(&from_status(&bad));
        assert_eq!(f.code, "unknown");
        assert_eq!(f.class, ErrorClass::Transient);
    }

    #[cfg(feature = "cli")]
    #[test]
    fn exit_code_matches_original() {
        use coded_error::cli::exit_code_anyhow;
        use coded_error::into_anyhow;
        fn make(i: usize) -> E {
            match i {
                0 => E::Missing,
                1 => E::Down,
                2 => E::Internal,
                _ => E::Unknown,
            }
        }
        for i in 0..4 {
            let want = exit_code_anyhow(&into_anyhow(make(i)));
            let got = exit_code_anyhow(&from_status(&to_status(&make(i), "s")));
            assert_eq!(got, want);
        }
        assert_eq!(
            exit_code_anyhow(&from_status(&to_status(&E::Down, "s"))),
            75
        );
    }

    #[cfg(feature = "mcp")]
    #[test]
    fn mcp_data_matches_original() {
        use coded_error::into_anyhow;
        use coded_error::mcp::to_mcp_error_anyhow;
        let orig = to_mcp_error_anyhow(&into_anyhow(E::Missing), "s");
        let remote = to_mcp_error_anyhow(&from_status(&to_status(&E::Missing, "srv")), "s");
        assert_eq!(remote.code, orig.code);
        assert_eq!(remote.message, orig.message);
        assert_eq!(remote.data, orig.data);
        assert_eq!(
            remote.data.as_ref().unwrap()["code"],
            "test.rpc.missing_name"
        );
    }
}

#[cfg(feature = "anyhow")]
mod redacted {
    use super::E;
    use coded_error::grpc::{to_status_anyhow, to_status_anyhow_redacted, CodedStatus};
    use coded_error::into_anyhow;
    use tonic::Code;

    const FIXED: &str = "request failed";

    #[test]
    fn a_logical_error_keeps_its_own_text_without_context() {
        let err = into_anyhow(E::NotFound).context("looking up action a1 in tenant t9");
        let status = to_status_anyhow_redacted(&err, "m", FIXED);
        assert_eq!(status.code(), Code::NotFound);
        assert_eq!(status.message(), "no such action");
    }

    #[test]
    fn every_other_class_sends_the_fixed_text() {
        for (err, code) in [
            (into_anyhow(E::Internal), Code::Internal),
            (into_anyhow(E::Down), Code::Unavailable),
            (into_anyhow(E::Unknown), Code::Unknown),
            (
                anyhow::anyhow!("connection to 10.0.0.5 refused"),
                Code::Unknown,
            ),
        ] {
            let status = to_status_anyhow_redacted(&err, "m", FIXED);
            assert_eq!(status.code(), code);
            assert_eq!(status.message(), FIXED);
        }
    }

    #[test]
    fn a_logical_attached_chain_or_relayed_message_sends_the_fixed_text() {
        let attached = coded_error::attach(
            anyhow::anyhow!("connection to 10.0.0.5 refused").context("tenant t9"),
            &E::Missing,
        );
        let relayed =
            coded_error::grpc::from_status(&coded_error::grpc::to_status(&E::Missing, "peer"));
        for err in [attached, relayed] {
            let status = to_status_anyhow_redacted(&err, "m", FIXED);
            assert_eq!(status.code(), Code::InvalidArgument);
            assert_eq!(status.message(), FIXED);
        }
    }

    #[test]
    fn the_metadata_matches_the_unredacted_status() {
        let err = into_anyhow(E::Down).context("db at 10.0.0.5");
        let plain = CodedStatus::from(&to_status_anyhow(&err, "m"));
        let redacted = CodedStatus::from(&to_status_anyhow_redacted(&err, "m", FIXED));
        assert_eq!(redacted, plain);
        assert_eq!(redacted.code.as_deref(), Some("test.rpc.db_down"));
    }
}
