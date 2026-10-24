use std::fmt;
use std::str::FromStr;

/// How an error should be treated. Separate from the code: class Unknown on
/// a real code is honest, only the code `unknown` marks an adoption gap.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "lowercase"))]
pub enum ErrorClass {
    /// The request is wrong or conflicts with state. Retrying won't help.
    Logical,
    /// Our side is broken: database, disk, config, a bug.
    Infrastructure,
    /// Retrying may succeed: timeout, unavailable, contention.
    Transient,
    /// Not classified.
    Unknown,
}

impl ErrorClass {
    /// Lowercase name, used by the fingerprint, metadata and serde.
    pub const fn as_str(self) -> &'static str {
        match self {
            ErrorClass::Logical => "logical",
            ErrorClass::Infrastructure => "infrastructure",
            ErrorClass::Transient => "transient",
            ErrorClass::Unknown => "unknown",
        }
    }
}

impl fmt::Display for ErrorClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseClassError(pub String);

impl fmt::Display for ParseClassError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown error class {:?}", self.0)
    }
}

impl std::error::Error for ParseClassError {}

impl FromStr for ErrorClass {
    type Err = ParseClassError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "logical" => Ok(ErrorClass::Logical),
            "infrastructure" => Ok(ErrorClass::Infrastructure),
            "transient" => Ok(ErrorClass::Transient),
            "unknown" => Ok(ErrorClass::Unknown),
            other => Err(ParseClassError(other.to_owned())),
        }
    }
}
