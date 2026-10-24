#![cfg(feature = "anyhow")]
use coded_error::{attach, downcast_ref, facts_anyhow, from_anyhow, into_anyhow, CodedError, ErrorClass, ResultExt, Uncoded};

#[derive(Debug, thiserror::Error, coded_error::CodedError)]
#[coded(namespace = "test.op")]
enum OpError {
    #[error("thing {0} not found")]
    #[coded(code = "not_found", class = Logical, grpc = NotFound)]
    NotFound(String),
}

fn typed() -> Result<(), OpError> { Err(OpError::NotFound("a1".into())) }

#[test]
fn into_anyhow_keeps_message_and_facts() {
    let err = into_anyhow(OpError::NotFound("a1".into()));
    assert_eq!(err.to_string(), "thing a1 not found");
    let c = from_anyhow(&err).unwrap();
    assert_eq!(c.code(), "test.op.not_found");
    assert_eq!(c.class(), ErrorClass::Logical);
}

#[test]
fn found_through_multiple_context_layers() {
    let err = into_anyhow(OpError::NotFound("a1".into()))
        .context("loading action")
        .context("handling GetAction");
    assert_eq!(from_anyhow(&err).unwrap().code(), "test.op.not_found");
    assert_eq!(format!("{err:#}"), "handling GetAction: loading action: thing a1 not found");
}

#[test]
fn coded_suffix_survives_question_mark() {
    fn op() -> anyhow::Result<()> {
        typed().coded()?;
        Ok(())
    }
    let err = op().unwrap_err();
    assert_eq!(facts_anyhow(&err).code, "test.op.not_found");
}

#[test]
fn bare_question_mark_lands_as_unknown() {
    fn op() -> anyhow::Result<()> {
        typed()?;
        Ok(())
    }
    let err = op().unwrap_err();
    assert!(from_anyhow(&err).is_none());
    let f = facts_anyhow(&err);
    assert_eq!((f.code, f.class), ("unknown", ErrorClass::Unknown));
}

#[test]
fn attach_to_existing_chain() {
    let base = anyhow::anyhow!("driver said no").context("query");
    let err = attach(base, &OpError::NotFound("x".into()));
    assert_eq!(err.to_string(), "query");
    assert_eq!(from_anyhow(&err).unwrap().code(), "test.op.not_found");
}

#[test]
fn nearest_coded_wins() {
    let inner = into_anyhow(OpError::NotFound("inner".into()));
    #[derive(Debug, thiserror::Error, coded_error::CodedError)]
    #[coded(namespace = "test.outer")]
    enum Outer {
        #[error("outer")]
        #[coded(code = "failed", class = Transient)]
        Failed,
    }
    let err = attach(inner.context("mid"), &Outer::Failed);
    assert_eq!(from_anyhow(&err).unwrap().code(), "test.outer.failed");
}

#[test]
fn uncoded_reports_unknown() {
    let u = Uncoded(anyhow::anyhow!("boom"));
    assert_eq!(u.code(), "unknown");
    assert_eq!(u.class(), ErrorClass::Unknown);
    assert_eq!(u.to_string(), "boom");
    assert_eq!(facts_anyhow(&anyhow::anyhow!("boom")), Uncoded::FACTS);
}

#[test]
fn downcast_sees_typed_error_through_coded_and_context() {
    // Plain anyhow downcast no longer sees through Coded; ours does.
    let err = into_anyhow(OpError::NotFound("a1".into())).context("loading");
    assert!(err.downcast_ref::<OpError>().is_none());
    assert!(matches!(downcast_ref::<OpError>(&err), Some(OpError::NotFound(id)) if id == "a1"));
    assert!(downcast_ref::<std::io::Error>(&err).is_none());
}

#[test]
fn downcast_sees_foreign_error_through_attach() {
    let bare = anyhow::Error::new(std::io::Error::other("disk"));
    let ctx = anyhow::Error::new(std::io::Error::other("disk")).context("query");
    for base in [bare, ctx] {
        let err = attach(base, &OpError::NotFound("x".into()));
        assert_eq!(from_anyhow(&err).unwrap().code(), "test.op.not_found");
        let io = downcast_ref::<std::io::Error>(&err).expect("io::Error survives attach");
        assert_eq!(io.to_string(), "disk");
    }
}
