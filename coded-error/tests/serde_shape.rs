use coded_error::{CodedError, ErrorClass};

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum FailureKind {
    Infrastructure,
    Transient,
}

impl From<FailureKind> for ErrorClass {
    fn from(k: FailureKind) -> Self {
        match k {
            FailureKind::Infrastructure => ErrorClass::Infrastructure,
            FailureKind::Transient => ErrorClass::Transient,
        }
    }
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error, serde::Serialize, CodedError)]
#[serde(tag = "kind", content = "data")]
#[coded(namespace = "compat.framework")]
pub enum CompatError {
    #[error("database: {message}")]
    #[coded(code = "database", class = from(kind))]
    Database { kind: FailureKind, message: String },
    #[error("not found")]
    #[coded(code = "not_found", class = Logical, http = 404)]
    NotFound,
}

#[test]
fn derive_coexists_with_thiserror2_and_serde() {
    let e = CompatError::Database {
        kind: FailureKind::Transient,
        message: "x".into(),
    };
    assert_eq!(e.code(), "compat.framework.database");
    assert_eq!(e.class(), ErrorClass::Transient);
    let v = serde_json::to_value(&e).unwrap();
    assert_eq!(v["kind"], "Database");
    assert_eq!(v["data"]["kind"], "Transient");
    assert_eq!(e.to_string(), "database: x");
}
