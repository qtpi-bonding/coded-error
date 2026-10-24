# coded-error

`coded-error` gives Rust services stable, machine-readable error codes without
losing ordinary Rust error handling. It separates an error's **code** from its
**class**, supports typed context, and provides integrations for the common
wire and error-reporting formats.

Implement the `CodedError` trait (or derive `ErrorClass` for an enum):

```rust
use coded_error::ErrorClass;

#[derive(Debug, ErrorClass)]
#[error_class(code = "billing.invoice")]
enum InvoiceError {
    #[error_class(code = "missing")]
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

Codes are dot-separated, lowercase segments: `domain.subject.reason` (one or
more non-empty segments, using ASCII letters, digits, and `-`). `unknown` is
outside this grammar and represents an absent or unclassified code. Code
namespaces must be unique within a crate and across an aggregate checked with
`check_codes`.

When converting an anyhow error with `into_anyhow` or `attach`, plain
`err.downcast_ref::<T>()` no longer sees through `Coded`; use
`coded_error::downcast_ref` instead. Anyhow-form mappings send the whole
context chain (`{:#}`) as the message. Wire mappings use `Display`.

Consume the crate by pinning a git revision in `Cargo.toml`:

```toml
coded-error = { git = "https://github.com/concord-codes/coded-error", rev = "<revision>" }
```

The check script routes cargo through `cargo-slot`; set `CARGO_SLOT` to
override its cargo-slot path.
