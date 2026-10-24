use crate::ErrorClass;
use sqlx_core::error::{Error, ErrorKind};

/// A code suffix and class for a sqlx error. Repos append the suffix to
/// their own namespace (`grorg.storage.` + suffix).
pub fn classify_sqlx(e: &Error) -> (&'static str, ErrorClass) {
    match e {
        Error::RowNotFound => ("row_not_found", ErrorClass::Logical),
        Error::PoolTimedOut => ("pool_timeout", ErrorClass::Transient),
        Error::Database(db) => match db.kind() {
            ErrorKind::UniqueViolation => ("unique_violation", ErrorClass::Logical),
            ErrorKind::ForeignKeyViolation => ("foreign_key_violation", ErrorClass::Logical),
            ErrorKind::NotNullViolation => ("not_null_violation", ErrorClass::Logical),
            ErrorKind::CheckViolation => ("check_violation", ErrorClass::Logical),
            _ => ("database", ErrorClass::Infrastructure),
        },
        _ => ("sqlx", ErrorClass::Infrastructure),
    }
}
