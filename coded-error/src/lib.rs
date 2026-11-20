//! Typed, coded errors. See the repository README for the model.

mod chain;
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

#[cfg(feature = "acp")]
pub mod acp;

#[cfg(feature = "proto")]
pub mod proto;

#[cfg(feature = "sqlx")]
pub mod sqlx;

#[cfg(feature = "anyhow")]
mod carry;

#[cfg(feature = "anyhow")]
pub use chain::chain_anyhow;
pub use chain::{chain, ChainLink, Origin};
pub use class::{ErrorClass, ParseClassError};
pub use code::{check_codes, is_valid_code, UNKNOWN_CODE};
pub use coded_error_derive::CodedError;
pub use fingerprint::fingerprint;
pub use info::ErrorInfo;
pub use overrides::{is_valid_acp_code, GrpcCode, McpCode, Overrides};

#[cfg(feature = "anyhow")]
pub use carry::{
    attach, downcast_ref, facts_anyhow, from_anyhow, from_remote, from_remote_chain, into_anyhow,
    Coded, ResultExt, Uncoded,
};

/// An error with a stable, namespaced code and a class.
pub trait CodedError: std::error::Error {
    /// Namespaced, stable: `grorg.action.not_found`.
    fn code(&self) -> &'static str;
    /// Runtime value: a variant may compute it from its fields.
    fn class(&self) -> ErrorClass;
    fn type_name(&self) -> &'static str {
        "unknown"
    }
    /// Per-variant boundary overrides.
    fn overrides(&self) -> Overrides {
        Overrides::NONE
    }
}

/// One code an error type can return, with its class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CodeEntry {
    /// The stable, namespaced error code.
    pub code: &'static str,
    /// The variant's class, or `None` when it is decided at runtime.
    pub class: Option<ErrorClass>,
}

/// Every code an error type can return, reachable through a generic bound.
/// Derived by `#[derive(CodedError)]`.
pub trait CodeCatalog {
    /// One entry per variant in declaration order; `#[coded(transparent, catalog)]`
    /// variants splice in the inner type's entries. De-duplicated by code.
    fn codes() -> Vec<CodeEntry>;
}

#[doc(hidden)]
pub fn dedup_entries(entries: Vec<CodeEntry>) -> Vec<CodeEntry> {
    let mut out: Vec<CodeEntry> = Vec::with_capacity(entries.len());
    for e in entries {
        if !out.iter().any(|o| o.code == e.code) {
            out.push(e);
        }
    }
    out
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
