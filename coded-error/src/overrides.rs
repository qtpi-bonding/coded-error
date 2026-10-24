/// gRPC status codes, owned here so the core does not depend on tonic.
/// Discriminants are the wire values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GrpcCode {
    Ok = 0,
    Cancelled = 1,
    Unknown = 2,
    InvalidArgument = 3,
    DeadlineExceeded = 4,
    NotFound = 5,
    AlreadyExists = 6,
    PermissionDenied = 7,
    ResourceExhausted = 8,
    FailedPrecondition = 9,
    Aborted = 10,
    OutOfRange = 11,
    Unimplemented = 12,
    Internal = 13,
    Unavailable = 14,
    DataLoss = 15,
    Unauthenticated = 16,
}

/// JSON-RPC / MCP error codes a variant may override to. Non-exhaustive so
/// more (method not found, custom server codes) can be added later.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum McpCode {
    InvalidParams = -32602,
    InvalidRequest = -32600,
    ResourceNotFound = -32002,
    InternalError = -32603,
}

/// Per-variant boundary overrides. `None` means "use the class default".
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Overrides {
    pub grpc: Option<GrpcCode>,
    pub http: Option<u16>,
    pub mcp: Option<McpCode>,
}

impl Overrides {
    pub const NONE: Overrides = Overrides {
        grpc: None,
        http: None,
        mcp: None,
    };

    /// `self` wins where set; `inner` fills the rest. Used for transparent
    /// variants, whose own overrides beat the inner error's.
    pub const fn or(self, inner: Overrides) -> Overrides {
        Overrides {
            grpc: match self.grpc {
                Some(c) => Some(c),
                None => inner.grpc,
            },
            http: match self.http {
                Some(c) => Some(c),
                None => inner.http,
            },
            mcp: match self.mcp {
                Some(c) => Some(c),
                None => inner.mcp,
            },
        }
    }
}
