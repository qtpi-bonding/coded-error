//! Agent Client Protocol (JSON-RPC) boundary. Plain data, so the core does
//! not depend on an ACP crate: callers build their own error type from it,
//! e.g. `agent_client_protocol::Error::new(e.code, e.message).data(e.data)`.
use crate::{CodedError, ErrorClass, ErrorInfo, Facts};

/// A JSON-RPC error for an ACP response. `data` is always a JSON object
/// holding the `ErrorInfo` fields (`code`, `class`, `fingerprint`), the same
/// shape MCP's `data` has; a caller may insert keys of its own beside them.
#[derive(Clone, Debug, PartialEq)]
pub struct AcpError {
    pub code: i32,
    pub message: String,
    pub data: serde_json::Value,
}

fn default_code(class: ErrorClass) -> i32 {
    match class {
        ErrorClass::Logical => -32602,
        // -32002 is ACP's resource-not-found, so Transient keeps the generic
        // internal error; clients tell it apart by `data.class`.
        ErrorClass::Infrastructure | ErrorClass::Transient | ErrorClass::Unknown => -32603,
    }
}

fn from_facts(facts: Facts, message: String, source: &str) -> AcpError {
    let mut data = serde_json::to_value(ErrorInfo::from_facts(&facts, source))
        .expect("ErrorInfo serializes to a JSON object");
    data["chain"] = serde_json::json!([facts.code]);
    AcpError {
        code: facts
            .overrides
            .acp
            .unwrap_or_else(|| default_code(facts.class)),
        message,
        data,
    }
}

#[cfg(feature = "anyhow")]
fn from_anyhow(err: &anyhow::Error, source: &str, message: String) -> AcpError {
    let mut out = from_facts(crate::facts_anyhow(err), message, source);
    out.data["chain"] = serde_json::Value::Array(
        crate::chain_anyhow(err)
            .into_iter()
            .map(|link| {
                serde_json::Value::String(link.code.unwrap_or(crate::UNKNOWN_CODE).to_owned())
            })
            .collect(),
    );
    out
}

/// Convert a coded error into an ACP error. The message is its `Display`.
pub fn to_acp_error<E: CodedError + ?Sized>(e: &E, source: &str) -> AcpError {
    from_facts(Facts::of(e), e.to_string(), source)
}

#[cfg(feature = "anyhow")]
pub fn to_acp_error_anyhow(err: &anyhow::Error, source: &str) -> AcpError {
    // `{:#}`: the whole context chain, same choice as the gRPC and MCP forms.
    from_anyhow(err, source, format!("{err:#}"))
}

/// As `to_acp_error_anyhow`, for an untrusted caller: the message is chosen
/// as `grpc::to_status_anyhow_redacted` chooses its message. The code and
/// data are the same as `to_acp_error_anyhow`'s.
#[cfg(feature = "anyhow")]
pub fn to_acp_error_anyhow_redacted(err: &anyhow::Error, source: &str, fixed: &str) -> AcpError {
    let message = crate::carry::redacted_message(err, fixed);
    from_anyhow(err, source, message)
}
