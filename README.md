# coded-error

`coded-error` gives Rust services stable, machine-readable error codes without
losing ordinary Rust error handling. It separates an error's **code** from its
**class**, supports typed context, and provides integrations for the common
wire and error-reporting formats.

Implement the `CodedError` trait (or derive `CodedError` for an enum):

```rust
#[derive(Debug, thiserror::Error, coded_error::CodedError)]
#[coded(namespace = "billing.invoice")]
enum InvoiceError {
    #[error("missing")]
    #[coded(code = "missing", class = Logical)]
    Missing,
}

```

The derive supports enums only, not structs. With
`default-features = false` the core depends only on `coded-error-derive` and
`sha2`; the default `anyhow` feature adds `anyhow`.

## Features

| Feature | Integration |
| --- | --- |
| `anyhow` (default) | anyhow carriage and context |
| `serde` | serde serialization |
| `grpc` | tonic/gRPC status |
| `http` | HTTP responses |
| `mcp` | Model Context Protocol |
| `acp` | Agent Client Protocol (plain `AcpError` data, no ACP crate) |
| `cli` | command-line output |
| `proto` | prost/protobuf |
| `sqlx` | SQLx database errors (0.7 only) |

Only sqlx 0.7 is supported. The `concord.` code prefix is reserved for
`coded-error` and `concord-telemetry`.

## Code grammar

Codes have three or more dot-separated segments, each matching
`[a-z][a-z0-9_]*`. The bare `unknown` is the only exception (see
`coded-error/src/code.rs` `is_valid_code`) and represents an absent or
unclassified code. Code
namespaces must be unique within a crate and across an aggregate checked with
`check_codes`.

When converting an anyhow error with `into_anyhow` or `attach`, plain
`err.downcast_ref::<T>()` no longer sees through `Coded`; use
`coded_error::downcast_ref` instead. Anyhow-form mappings that carry a
message (gRPC, MCP, ACP) send the whole context chain (`{:#}`). Wire mappings
use `Display`.

ACP errors are JSON-RPC: Logical maps to -32602 and every other class to
-32603 (ACP reserves -32002 for resource-not-found, so Transient is told apart
by `data.class`, not by number). `#[coded(acp = -32001)]` overrides the
number; it must be a standard JSON-RPC code or in -32099..=-32000, checked at
compile time. `data` is the `ErrorInfo` object, as in MCP, and a caller may
insert keys of its own beside `code`, `class` and `fingerprint`.

At a boundary facing an untrusted caller, use
`grpc::to_status_anyhow_redacted(&err, source, "request failed")`,
`mcp::to_mcp_error_anyhow_redacted`, `acp::to_acp_error_anyhow_redacted`, or
`http::to_http_anyhow_redacted`
(which returns the message beside `ErrorInfo` for the caller to place in its
body). A logical typed error sends only its own `Display`; every other class,
an `attach`ed chain and a relayed remote message send the fixed text. Codes,
statuses and metadata are unchanged. A logical variant whose `#[error]`
interpolates caller input sends that input back, so keep such Display text
to what the caller already knows.

## Carrying a remote code (client side)

A client that receives a coded `tonic::Status` converts it with
`coded_error::grpc::from_status(&status)` (features `grpc` + `anyhow`). The
result is an anyhow error whose `Coded` carries the server's code and class,
so `from_anyhow`, `cli::exit_code_anyhow` and `mcp::to_mcp_error_anyhow` see
them exactly as for a local typed error. A status with no coded metadata is a
plain uncoded error. `coded_error::from_remote(code, class, message)` is the
transport-neutral form.

`Facts.code` is a `&'static str`, so a remote code is interned: leaked once per
distinct valid code. The set is first-come and bounded: at most 4096 distinct
codes of at most 128 bytes. Past that, or for a malformed code, a new code
reads as `unknown` with its class kept and the set never grows. A hostile peer
could fill the set, so treat remote codes as advisory.

Consume the crate by pinning a git revision in `Cargo.toml`:

```toml
coded-error = { git = "https://github.com/qtpi-bonding/coded-error.git", rev = "<revision>" }
```

The check script routes cargo through `cargo-slot`; set `CARGO_SLOT` to
override its cargo-slot path.
