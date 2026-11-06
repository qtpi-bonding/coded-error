//! gRPC boundary. Message = Display text; code, class and fingerprint in metadata.
use crate::{fingerprint, CodedError, ErrorClass, Facts, GrpcCode};
use tonic::metadata::MetadataValue;
use tonic::{Code, Status};

/// Metadata key carrying the stable error code.
pub const CODE_KEY: &str = "x-error-code";
/// Metadata key carrying the error class.
pub const CLASS_KEY: &str = "x-error-class";
/// Metadata key carrying the error fingerprint.
pub const FINGERPRINT_KEY: &str = "x-error-fingerprint";
pub const CHAIN_KEY: &str = "x-error-chain";

fn default_code(class: ErrorClass) -> GrpcCode {
    match class {
        ErrorClass::Logical => GrpcCode::InvalidArgument,
        ErrorClass::Infrastructure => GrpcCode::Internal,
        ErrorClass::Transient => GrpcCode::Unavailable,
        ErrorClass::Unknown => GrpcCode::Unknown,
    }
}

fn status_from(facts: Facts, message: String, source: &str) -> Status {
    let grpc = facts
        .overrides
        .grpc
        .unwrap_or_else(|| default_code(facts.class));
    let mut status = Status::new(Code::from(grpc as i32), message);
    let md = status.metadata_mut();
    // Codes and classes are ASCII by construction; fingerprint is hex.
    for (k, v) in [
        (CODE_KEY, facts.code.to_owned()),
        (CLASS_KEY, facts.class.as_str().to_owned()),
        (
            FINGERPRINT_KEY,
            fingerprint(facts.code, facts.class, source),
        ),
    ] {
        if let Ok(val) = MetadataValue::try_from(v.as_str()) {
            md.insert(k, val);
        }
    }
    status
}

/// Convert a coded error into a gRPC status with coded metadata.
pub fn to_status<E: CodedError + ?Sized>(e: &E, source: &str) -> Status {
    let mut status = status_from(Facts::of(e), e.to_string(), source);
    let code = e.code();
    if let Ok(value) = MetadataValue::try_from(code) {
        status.metadata_mut().insert(CHAIN_KEY, value);
    }
    status
}

#[cfg(feature = "anyhow")]
fn add_chain_metadata(status: &mut Status, err: &anyhow::Error) {
    let codes = crate::chain_anyhow(err)
        .iter()
        .map(|l| l.code.unwrap_or(crate::UNKNOWN_CODE))
        .collect::<Vec<_>>()
        .join(",");
    if let Ok(v) = MetadataValue::try_from(codes) {
        status.metadata_mut().insert(CHAIN_KEY, v);
    }
}

#[cfg(feature = "anyhow")]
pub fn to_status_anyhow(err: &anyhow::Error, source: &str) -> Status {
    // `{:#}`: the whole context chain, as operators expect (see the design's Display-on-the-wire note).
    let mut status = status_from(crate::facts_anyhow(err), format!("{err:#}"), source);
    add_chain_metadata(&mut status, err);
    status
}

/// As `to_status_anyhow`, for a boundary facing an untrusted caller. A
/// logical typed error (`into_anyhow`) keeps its own Display, without the
/// context added around it. Everything else sends `fixed` instead: other
/// classes, uncoded errors, chains coded with `attach`, and messages relayed
/// from a remote status. The metadata is the same as `to_status_anyhow`'s.
#[cfg(feature = "anyhow")]
pub fn to_status_anyhow_redacted(err: &anyhow::Error, source: &str, fixed: &str) -> Status {
    let message = crate::carry::redacted_message(err, fixed);
    let mut status = status_from(crate::facts_anyhow(err), message, source);
    add_chain_metadata(&mut status, err);
    status
}

/// Client side: read the facts back so callers branch on `code`, not text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CodedStatus {
    pub code: Option<String>,
    pub class: Option<ErrorClass>,
    pub fingerprint: Option<String>,
    pub chain: Vec<String>,
}

impl From<&Status> for CodedStatus {
    fn from(s: &Status) -> Self {
        let get = |k: &str| {
            s.metadata()
                .get(k)
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned)
        };
        CodedStatus {
            code: get(CODE_KEY),
            class: get(CLASS_KEY).and_then(|c| c.parse().ok()),
            fingerprint: get(FINGERPRINT_KEY),
            chain: get(CHAIN_KEY)
                .map(|v| v.split(',').map(str::to_owned).collect())
                .unwrap_or_default(),
        }
    }
}

/// Client side: turn a received status into an anyhow error the readers
/// recognise (`from_anyhow`, `exit_code_anyhow`, `to_mcp_error_anyhow`,
/// `to_status_anyhow`), carrying the server's code and class. A status
/// without coded metadata is a plain uncoded error (`facts_anyhow` gives
/// `unknown`). Codes are interned first-come, at most 4096 distinct codes of at
/// most 128 bytes; beyond that a new code reads as `unknown` with its class kept. Display is the status message.
#[cfg(feature = "anyhow")]
pub fn from_status(status: &Status) -> anyhow::Error {
    from_coded_status(&CodedStatus::from(status), status.message())
}

/// As `from_status`, for callers that already extracted the `CodedStatus`.
#[cfg(feature = "anyhow")]
pub fn from_coded_status(coded: &CodedStatus, message: &str) -> anyhow::Error {
    let chain = coded.chain.iter().map(String::as_str).collect::<Vec<_>>();
    crate::from_remote_chain(
        coded.code.as_deref(),
        coded.class,
        message.to_owned(),
        &chain,
    )
}
