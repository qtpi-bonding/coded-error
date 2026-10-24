//! Typed, coded errors. See the repository README for the model.

mod class;
mod code;
mod overrides;

pub use class::{ErrorClass, ParseClassError};
pub use code::{check_codes, is_valid_code, UNKNOWN_CODE};
pub use coded_error_derive::CodedError;
pub use overrides::{GrpcCode, McpCode, Overrides};

/// An error with a stable, namespaced code and a class.
pub trait CodedError: std::error::Error {
    /// Namespaced, stable: `grorg.action.not_found`.
    fn code(&self) -> &'static str;
    /// Runtime value: a variant may compute it from its fields.
    fn class(&self) -> ErrorClass;
    /// Per-variant boundary overrides.
    fn overrides(&self) -> Overrides {
        Overrides::NONE
    }
}

/// The facts every boundary mapping works from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Facts {
    pub code: &'static str,
    pub class: ErrorClass,
    pub overrides: Overrides,
}

impl Facts {
    pub fn of<E: CodedError + ?Sized>(e: &E) -> Facts {
        Facts {
            code: e.code(),
            class: e.class(),
            overrides: e.overrides(),
        }
    }
}
