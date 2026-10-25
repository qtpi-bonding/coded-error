#![cfg(feature = "acp")]
use coded_error::acp::to_acp_error;
use coded_error::{CodedError, ErrorInfo};

#[derive(Debug, thiserror::Error, CodedError)]
#[coded(namespace = "test.acp")]
enum E {
    #[error("bad")]
    #[coded(code = "bad", class = Logical)]
    Bad,
    #[error("already seen")]
    #[coded(code = "already_seen", class = Logical, acp = -32001)]
    AlreadySeen,
    #[error("down")]
    #[coded(code = "down", class = Transient)]
    Down,
    #[error("bug")]
    #[coded(code = "bug", class = Infrastructure)]
    Bug,
    #[error(transparent)]
    #[coded(transparent, acp = -32050)]
    Wrapped(Inner),
    #[error(transparent)]
    #[coded(transparent)]
    Passed(Inner),
}

#[derive(Debug, thiserror::Error, CodedError)]
#[coded(namespace = "test.acp_inner")]
enum Inner {
    #[error("inner")]
    #[coded(code = "inner", class = Logical)]
    One,
    #[error("inner seen")]
    #[coded(code = "seen", class = Logical, acp = -32001)]
    Seen,
}

#[test]
fn class_defaults_and_override() {
    assert_eq!(to_acp_error(&E::Bad, "s").code, -32602);
    assert_eq!(to_acp_error(&E::AlreadySeen, "s").code, -32001);
    assert_eq!(to_acp_error(&E::Down, "s").code, -32603);
    assert_eq!(to_acp_error(&E::Bug, "s").code, -32603);
    assert_eq!(to_acp_error(&E::Wrapped(Inner::One), "s").code, -32050);
    assert_eq!(to_acp_error(&E::Wrapped(Inner::Seen), "s").code, -32050);
    assert_eq!(to_acp_error(&E::Passed(Inner::Seen), "s").code, -32001);
    assert_eq!(to_acp_error(&E::Passed(Inner::One), "s").code, -32602);
}

#[test]
fn data_is_the_error_info_object_and_callers_can_add_keys() {
    let mut e = to_acp_error(&E::AlreadySeen, "s");
    assert_eq!(e.message, "already seen");
    assert_eq!(
        e.data,
        serde_json::to_value(ErrorInfo::of(&E::AlreadySeen, "s")).unwrap()
    );
    assert_eq!(e.data["code"], "test.acp.already_seen");
    assert_eq!(e.data["class"], "logical");
    e.data
        .as_object_mut()
        .unwrap()
        .insert("reason".into(), "already_seen_by_agent".into());
    assert_eq!(e.data["reason"], "already_seen_by_agent");
}

#[test]
fn valid_override_codes() {
    use coded_error::is_valid_acp_code;
    for ok in [-32700, -32603, -32600, -32099, -32001, -32000] {
        assert!(is_valid_acp_code(ok), "{ok}");
    }
    for bad in [-32701, -32604, -32599, -32100, -31999, 0, 1] {
        assert!(!is_valid_acp_code(bad), "{bad}");
    }
}

#[cfg(feature = "anyhow")]
mod anyhow_forms {
    use super::E;
    use coded_error::acp::{to_acp_error_anyhow, to_acp_error_anyhow_redacted};
    use coded_error::into_anyhow;

    const FIXED: &str = "request failed";

    #[test]
    fn unredacted_message_is_the_whole_chain() {
        let err = into_anyhow(E::Bad).context("loading");
        let e = to_acp_error_anyhow(&err, "s");
        assert_eq!(e.message, "loading: bad");
        assert_eq!(e.code, -32602);
        let u = to_acp_error_anyhow(&anyhow::anyhow!("boom"), "s");
        assert_eq!((u.code, &u.data["code"]), (-32603, &"unknown".into()));
    }

    #[test]
    fn redacted_keeps_only_a_logical_typed_message() {
        let e =
            to_acp_error_anyhow_redacted(&into_anyhow(E::AlreadySeen).context("k1"), "s", FIXED);
        assert_eq!((e.code, e.message.as_str()), (-32001, "already seen"));

        let attached = coded_error::attach(anyhow::anyhow!("10.0.0.5 refused"), &E::Bad);
        let remote = coded_error::from_remote(
            Some("peer.x"),
            Some(coded_error::ErrorClass::Logical),
            "10.0.0.5".into(),
        );
        for err in [
            into_anyhow(E::Down).context("db at 10.0.0.5"),
            anyhow::anyhow!("10.0.0.5 refused"),
            attached,
            remote,
        ] {
            let redacted = to_acp_error_anyhow_redacted(&err, "s", FIXED);
            let plain = to_acp_error_anyhow(&err, "s");
            assert_eq!(redacted.message, FIXED);
            assert_eq!((redacted.code, redacted.data), (plain.code, plain.data));
        }
    }
}
