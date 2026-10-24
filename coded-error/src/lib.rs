//! Typed, coded errors. See the repository README for the model.

mod class;
mod code;
mod fingerprint;
mod info;
mod overrides;

#[cfg(feature = "grpc")]
pub mod grpc;

#[cfg(feature = "http")]
pub mod http;

#[cfg(feature = "cli")]
pub mod cli;

#[cfg(feature = "mcp")]
pub mod mcp;

#[cfg(feature = "proto")]
pub mod proto;

#[cfg(feature = "sqlx")]
pub mod sqlx;

#[cfg(feature = "anyhow")]
mod carry;

pub use class::{ErrorClass, ParseClassError};
pub use code::{check_codes, is_valid_code, UNKNOWN_CODE};
pub use coded_error_derive::CodedError;
pub use fingerprint::fingerprint;
pub use info::ErrorInfo;
pub use overrides::{GrpcCode, McpCode, Overrides};

#[cfg(feature = "anyhow")]
pub use carry::{
    attach, downcast_ref, facts_anyhow, from_anyhow, into_anyhow, Coded, ResultExt, Uncoded,
};

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
    /// The stable, namespaced error code.
    pub code: &'static str,
    /// The error's broad classification.
    pub class: ErrorClass,
    /// Boundary-specific mapping overrides.
    pub overrides: Overrides,
}

impl Facts {
    /// Extract the boundary-mapping facts from a coded error.
    pub fn of<E: CodedError + ?Sized>(e: &E) -> Facts {
        Facts {
            code: e.code(),
            class: e.class(),
            overrides: e.overrides(),
        }
    }
}
