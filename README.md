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
`coded_error::downcast_ref` instead. Anyhow-form mappings send the whole
context chain (`{:#}`) as the message. Wire mappings use `Display`.

## Carrying a remote code (client side)

A client that receives a coded `tonic::Status` converts it with
`coded_error::grpc::from_status(&status)` (features `grpc` + `anyhow`). The
result is an anyhow error whose `Coded` carries the server's code and class,
so `from_anyhow`, `cli::exit_code_anyhow` and `mcp::to_mcp_error_anyhow` see
them exactly as for a local typed error. The code is interned into the
`&'static str` in `Facts` (leaked once per distinct valid code, capped at
4096); a malformed code reads as `unknown`. A status with no coded metadata
is a plain uncoded error. `coded_error::from_remote(code, class, message)` is
the transport-neutral form.

Consume the crate by pinning a git revision in `Cargo.toml`:

```toml
coded-error = { git = "https://github.com/qtpi-bonding/coded-error.git", rev = "<revision>" }
```

The check script routes cargo through `cargo-slot`; set `CARGO_SLOT` to
override its cargo-slot path.
