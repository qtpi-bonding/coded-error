use crate::{CodedError, ErrorClass, ErrorInfo, Facts, McpCode};
use rmcp::model::{ErrorCode, ErrorData};

fn from_facts(facts: Facts, message: String, source: &str) -> ErrorData {
    let code = facts.overrides.mcp.unwrap_or(match facts.class {
        ErrorClass::Logical => McpCode::InvalidParams,
        _ => McpCode::InternalError,
    });
    let mut data = serde_json::to_value(ErrorInfo::from_facts(&facts, source)).ok();
    if let Some(object) = data.as_mut().and_then(|v| v.as_object_mut()) {
        object.insert("chain".to_owned(), serde_json::json!([facts.code]));
    }
    ErrorData::new(ErrorCode(code as i32), message, data)
}

#[cfg(feature = "anyhow")]
fn from_anyhow(err: &anyhow::Error, source: &str, message: String) -> ErrorData {
    let mut out = from_facts(crate::facts_anyhow(err), message, source);
    if let Some(object) = out.data.as_mut().and_then(|v| v.as_object_mut()) {
        object.insert(
            "chain".to_owned(),
            serde_json::Value::Array(
                crate::chain_anyhow(err)
                    .into_iter()
                    .map(|link| {
                        serde_json::Value::String(
                            link.code.unwrap_or(crate::UNKNOWN_CODE).to_owned(),
                        )
                    })
                    .collect(),
            ),
        );
    }
    out
}

/// Convert a coded error into an MCP error with structured error information.
pub fn to_mcp_error<E: CodedError + ?Sized>(e: &E, source: &str) -> ErrorData {
    from_facts(Facts::of(e), e.to_string(), source)
}

#[cfg(feature = "anyhow")]
pub fn to_mcp_error_anyhow(err: &anyhow::Error, source: &str) -> ErrorData {
    // `{:#}`: the whole context chain, same choice as the gRPC form.
    from_anyhow(err, source, format!("{err:#}"))
}

/// As `to_mcp_error_anyhow`, for an untrusted caller: the message is chosen
/// as `grpc::to_status_anyhow_redacted` chooses its message. The code and
/// data are the same as `to_mcp_error_anyhow`'s.
#[cfg(feature = "anyhow")]
pub fn to_mcp_error_anyhow_redacted(err: &anyhow::Error, source: &str, fixed: &str) -> ErrorData {
    let message = crate::carry::redacted_message(err, fixed);
    from_anyhow(err, source, message)
}
