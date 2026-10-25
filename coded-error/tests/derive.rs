use coded_error::{CodedError, ErrorClass, GrpcCode, McpCode, Overrides};

#[derive(Debug, Clone, Copy)]
pub enum FailureKind {
    Logical,
    Transient,
}
impl From<FailureKind> for ErrorClass {
    fn from(k: FailureKind) -> ErrorClass {
        match k {
            FailureKind::Logical => ErrorClass::Logical,
            FailureKind::Transient => ErrorClass::Transient,
        }
    }
}

#[derive(Debug, thiserror::Error, CodedError)]
#[coded(namespace = "test.storage")]
pub enum StorageError {
    #[error("io")]
    #[coded(code = "io", class = Infrastructure, grpc = Internal, http = 503)]
    Io,
}

fn classify(e: &ActionError) -> ErrorClass {
    match e {
        ActionError::Backend(n) if *n > 0 => ErrorClass::Transient,
        _ => ErrorClass::Unknown,
    }
}

#[derive(Debug, thiserror::Error, CodedError)]
#[coded(namespace = "test.action")]
pub enum ActionError {
    #[error("action not found")]
    #[coded(code = "not_found", class = Logical, grpc = NotFound, http = 404, mcp = ResourceNotFound)]
    NotFound { action_id: String },

    #[error(transparent)]
    #[coded(transparent, grpc = Unavailable)]
    Storage(#[from] StorageError),

    #[error("database: {message}")]
    #[coded(code = "database", class = from(kind))]
    Database { kind: FailureKind, message: String },

    #[error("backend {0}")]
    #[coded(code = "backend", class_with = classify)]
    Backend(u32),

    #[error("tuple from")]
    #[coded(code = "tuple_from", class = from(0))]
    TupleFrom(FailureKind),
}

#[test]
fn codes_and_classes() {
    let nf = ActionError::NotFound {
        action_id: "a".into(),
    };
    assert_eq!(nf.code(), "test.action.not_found");
    assert_eq!(nf.class(), ErrorClass::Logical);
    assert_eq!(
        nf.overrides(),
        Overrides {
            grpc: Some(GrpcCode::NotFound),
            http: Some(404),
            mcp: Some(McpCode::ResourceNotFound),
            acp: None,
        }
    );

    let db = ActionError::Database {
        kind: FailureKind::Transient,
        message: "x".into(),
    };
    assert_eq!(db.code(), "test.action.database");
    assert_eq!(db.class(), ErrorClass::Transient);

    assert_eq!(ActionError::Backend(1).class(), ErrorClass::Transient);
    assert_eq!(ActionError::Backend(0).class(), ErrorClass::Unknown);
    assert_eq!(
        ActionError::TupleFrom(FailureKind::Logical).class(),
        ErrorClass::Logical
    );
}

#[test]
fn transparent_takes_inner_code_and_class_and_outer_override_wins() {
    let e = ActionError::Storage(StorageError::Io);
    assert_eq!(e.code(), "test.storage.io");
    assert_eq!(e.class(), ErrorClass::Infrastructure);
    // The outer grpc override wins; http is not set outside, so the inner 503 falls through.
    assert_eq!(e.overrides().grpc, Some(GrpcCode::Unavailable));
    assert_eq!(e.overrides().http, Some(503));
    assert_eq!(e.overrides().mcp, None);
}

#[test]
fn all_codes_lists_own_codes_only() {
    assert_eq!(
        ActionError::ALL_CODES,
        &[
            "test.action.not_found",
            "test.action.database",
            "test.action.backend",
            "test.action.tuple_from"
        ]
    );
    coded_error::check_codes("test.action.", &[ActionError::ALL_CODES]).unwrap();
}
