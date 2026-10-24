use crate::{CodedError, ErrorClass, ErrorInfo, Facts, McpCode};
use rmcp::model::{ErrorCode, ErrorData};

fn from_facts(facts: Facts, message: String, source: &str) -> ErrorData {
    let code = facts.overrides.mcp.unwrap_or(match facts.class {
        ErrorClass::Logical => McpCode::InvalidParams,
        _ => McpCode::InternalError,
    });
    let data = serde_json::to_value(ErrorInfo::from_facts(&facts, source)).ok();
    ErrorData::new(ErrorCode(code as i32), message, data)
}

/// Convert a coded error into an MCP error with structured error information.
pub fn to_mcp_error<E: CodedError + ?Sized>(e: &E, source: &str) -> ErrorData {
    from_facts(Facts::of(e), e.to_string(), source)
}

#[cfg(feature = "anyhow")]
pub fn to_mcp_error_anyhow(err: &anyhow::Error, source: &str) -> ErrorData {
    // `{:#}`: the whole context chain, same choice as the gRPC form.
    from_facts(crate::facts_anyhow(err), format!("{err:#}"), source)
}
