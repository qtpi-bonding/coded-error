use crate::{CodedError, ErrorClass, Facts, Overrides, UNKNOWN_CODE};
use std::error::Error;
use std::fmt;

/// What `Coded` wraps: a typed error (`into_anyhow`) or an existing anyhow
/// chain (`attach`). Keeping the anyhow error whole lets anyhow's own
/// downcast see through its context layers.
enum Inner {
    Typed(Box<dyn Error + Send + Sync + 'static>),
    Chain(anyhow::Error),
}

impl Inner {
    fn as_dyn(&self) -> &(dyn Error + 'static) {
        match self {
            Inner::Typed(b) => &**b,
            Inner::Chain(a) => &**a,
        }
    }
}

/// A coded error carried through anyhow. Display and `source()` are the
/// inner error's, so messages and `{:#}` chains are unchanged.
pub struct Coded {
    facts: Facts,
    inner: Inner,
}

impl Coded {
    pub fn facts(&self) -> Facts {
        self.facts
    }

    /// Downcast the wrapped error to a caller's own type.
    pub fn downcast_ref<T: Error + Send + Sync + 'static>(&self) -> Option<&T> {
        match &self.inner {
            Inner::Typed(b) => b.downcast_ref::<T>(),
            Inner::Chain(a) => a.downcast_ref::<T>(),
        }
    }
}

impl fmt::Display for Coded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.inner.as_dyn(), f)
    }
}

impl fmt::Debug for Coded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Coded")
            .field("facts", &self.facts)
            .field("inner", &self.inner.as_dyn())
            .finish()
    }
}

impl Error for Coded {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.inner.as_dyn().source()
    }
}

impl CodedError for Coded {
    fn code(&self) -> &'static str {
        self.facts.code
    }

    fn class(&self) -> ErrorClass {
        self.facts.class
    }

    fn overrides(&self) -> Overrides {
        self.facts.overrides
    }
}

/// Raise a typed error into anyhow with its facts attached.
pub fn into_anyhow<E: CodedError + Send + Sync + 'static>(e: E) -> anyhow::Error {
    let facts = Facts::of(&e);
    anyhow::Error::new(Coded {
        facts,
        inner: Inner::Typed(Box::new(e)),
    })
}

/// A failure reported by a remote peer (a gRPC status, say). Display is the
/// remote message.
#[derive(Debug)]
struct Remote(String);

impl fmt::Display for Remote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for Remote {}

/// Most distinct remote codes kept for the process lifetime. Real code
/// vocabularies are small; the cap bounds what a hostile peer can make us hold.
const MAX_REMOTE_CODES: usize = 4096;

/// `Facts.code` is `&'static str`, so a code that arrives at runtime is
/// interned: leaked once per distinct value and reused after. Invalid codes
/// (grammar, see `is_valid_code`) and codes past the cap read as `unknown`.
fn intern_code(code: &str) -> &'static str {
    use std::collections::HashSet;
    use std::sync::{Mutex, OnceLock};
    static SEEN: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
    if code == UNKNOWN_CODE || !crate::is_valid_code(code) {
        return UNKNOWN_CODE;
    }
    let mut seen = SEEN
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    if let Some(s) = seen.get(code) {
        return s;
    }
    if seen.len() >= MAX_REMOTE_CODES {
        return UNKNOWN_CODE;
    }
    let leaked: &'static str = Box::leak(code.to_owned().into_boxed_str());
    seen.insert(leaked);
    leaked
}

/// Raise a failure that happened on the other side of a boundary into anyhow
/// as a `Coded`, so `from_anyhow`, `facts_anyhow` and every boundary mapping
/// (cli exit codes, mcp, grpc re-encoding) see the remote `code` and `class`.
///
/// Design: the code is interned into the existing `Facts` (see `intern_code`)
/// rather than adding an owned-code variant, so `Facts` stays `Copy` and no
/// reader changes. A missing or malformed `code` becomes `unknown` and a
/// missing `class` becomes `Unknown`. With neither, the result is a plain
/// (uncoded) error. Display is `message`.
pub fn from_remote(
    code: Option<&str>,
    class: Option<ErrorClass>,
    message: String,
) -> anyhow::Error {
    if code.is_none() && class.is_none() {
        return anyhow::Error::new(Remote(message));
    }
    let facts = Facts {
        code: code.map_or(UNKNOWN_CODE, intern_code),
        class: class.unwrap_or(ErrorClass::Unknown),
        overrides: Overrides::NONE,
    };
    anyhow::Error::new(Coded {
        facts,
        inner: Inner::Typed(Box::new(Remote(message))),
    })
}

/// Attach a typed error's facts to an existing anyhow chain.
pub fn attach<E: CodedError + ?Sized>(err: anyhow::Error, typed: &E) -> anyhow::Error {
    let facts = Facts::of(typed);
    anyhow::Error::new(Coded {
        facts,
        inner: Inner::Chain(err),
    })
}

/// The nearest `Coded` in the chain.
pub fn from_anyhow(err: &anyhow::Error) -> Option<&Coded> {
    err.chain().find_map(|e| e.downcast_ref::<Coded>())
}

/// Downcast to a caller's own error type anywhere in the chain, looking
/// inside every `Coded`. Plain `err.downcast_ref` cannot see through `Coded`.
pub fn downcast_ref<T: Error + Send + Sync + 'static>(err: &anyhow::Error) -> Option<&T> {
    err.chain().find_map(|e| {
        e.downcast_ref::<T>()
            .or_else(|| e.downcast_ref::<Coded>()?.downcast_ref::<T>())
    })
}

/// Facts for any anyhow error: the nearest `Coded`, else `Uncoded::FACTS`.
pub fn facts_anyhow(err: &anyhow::Error) -> Facts {
    from_anyhow(err).map(Coded::facts).unwrap_or(Uncoded::FACTS)
}

/// An untyped error. Counting its `unknown` code measures the adoption gap.
#[derive(Debug)]
pub struct Uncoded(pub anyhow::Error);

impl Uncoded {
    /// The facts of any error without a `Coded`.
    pub const FACTS: Facts = Facts {
        code: UNKNOWN_CODE,
        class: ErrorClass::Unknown,
        overrides: Overrides::NONE,
    };
}

impl fmt::Display for Uncoded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl Error for Uncoded {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.0.source()
    }
}

impl CodedError for Uncoded {
    fn code(&self) -> &'static str {
        UNKNOWN_CODE
    }

    fn class(&self) -> ErrorClass {
        ErrorClass::Unknown
    }
}

/// `.coded()?` instead of `?` where a typed error crosses into anyhow.
pub trait ResultExt<T> {
    fn coded(self) -> anyhow::Result<T>;
}

impl<T, E: CodedError + Send + Sync + 'static> ResultExt<T> for Result<T, E> {
    fn coded(self) -> anyhow::Result<T> {
        self.map_err(into_anyhow)
    }
}
