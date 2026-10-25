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

    /// The wrapped typed error's own Display. `None` for an attached anyhow
    /// chain or a relayed remote message, whose text this process did not
    /// write and may carry anything.
    #[cfg(any(feature = "grpc", feature = "http", feature = "mcp", feature = "acp"))]
    fn own_message(&self) -> Option<String> {
        match &self.inner {
            Inner::Typed(b) if !b.is::<Remote>() => Some(b.to_string()),
            _ => None,
        }
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

/// Most distinct remote codes kept for the process lifetime, first come first
/// kept. Past it, a new code reads as `unknown` (class kept) and the set never
/// grows; codes already interned keep working.
const MAX_REMOTE_CODES: usize = 4096;

/// Longest remote code interned, in bytes. Longer codes read as `unknown`.
/// Together with `MAX_REMOTE_CODES` this bounds the leaked memory.
const MAX_REMOTE_CODE_LEN: usize = 128;

type Interned = std::sync::Mutex<std::collections::HashSet<&'static str>>;

fn intern_into(set: &Interned, code: &str, cap: usize) -> &'static str {
    if code == UNKNOWN_CODE || code.len() > MAX_REMOTE_CODE_LEN || !crate::is_valid_code(code) {
        return UNKNOWN_CODE;
    }
    let mut seen = set.lock().unwrap_or_else(|p| p.into_inner());
    if let Some(s) = seen.get(code) {
        return s;
    }
    if seen.len() >= cap {
        return UNKNOWN_CODE;
    }
    let leaked: &'static str = Box::leak(code.to_owned().into_boxed_str());
    seen.insert(leaked);
    leaked
}

/// `Facts.code` is `&'static str`, so a code that arrives at runtime is
/// interned: leaked once per distinct value and reused after. Invalid codes
/// (grammar, see `is_valid_code`), codes over 128 bytes and codes past the
/// 4096-code cap read as `unknown`.
fn intern_code(code: &str) -> &'static str {
    static SEEN: std::sync::OnceLock<Interned> = std::sync::OnceLock::new();
    intern_into(SEEN.get_or_init(Default::default), code, MAX_REMOTE_CODES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_code_same_pointer() {
        let set = Interned::default();
        let a = intern_into(&set, "t.intern.same", 4);
        let b = intern_into(&set, &String::from("t.intern.same"), 4);
        assert_eq!(a, "t.intern.same");
        assert!(std::ptr::eq(a, b));
    }

    #[test]
    fn cap_is_first_come_and_set_never_grows() {
        let set = Interned::default();
        assert_eq!(intern_into(&set, "t.cap.one", 2), "t.cap.one");
        assert_eq!(intern_into(&set, "t.cap.two", 2), "t.cap.two");
        assert_eq!(intern_into(&set, "t.cap.three", 2), UNKNOWN_CODE);
        assert_eq!(set.lock().unwrap().len(), 2);
        // already-interned codes still resolve past the cap
        assert_eq!(intern_into(&set, "t.cap.one", 2), "t.cap.one");
    }

    #[test]
    fn over_long_code_reads_unknown() {
        let set = Interned::default();
        let ok = format!("t.len.{}", "a".repeat(MAX_REMOTE_CODE_LEN - 6));
        assert_eq!(ok.len(), MAX_REMOTE_CODE_LEN);
        assert_eq!(intern_into(&set, &ok, 4), ok);
        let long = format!("{ok}a");
        assert_eq!(intern_into(&set, &long, 4), UNKNOWN_CODE);
        assert_eq!(set.lock().unwrap().len(), 1);
    }
}

/// Raise a failure that happened on the other side of a boundary into anyhow
/// as a `Coded`, so `from_anyhow`, `facts_anyhow` and every boundary mapping
/// (cli exit codes, mcp, grpc re-encoding) see the remote `code` and `class`.
///
/// Design: the code is interned into the existing `Facts` (see `intern_code`)
/// rather than adding an owned-code variant, so `Facts` stays `Copy` and no
/// reader changes. A missing or malformed `code` becomes `unknown` and a
/// missing `class` becomes `Unknown`. Interning is first-come: the set holds at
/// most 4096 distinct codes of at most 128 bytes; past that a new code reads as
/// `unknown` with its class kept (a peer can fill the set, so treat the code as
/// advisory for untrusted peers). With neither, the result is a plain
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

/// The message a boundary facing an untrusted caller sends: a logical typed
/// error's own Display, without the context added around it; `fixed` for
/// everything else (other classes, uncoded errors, chains coded with
/// `attach`, messages relayed from a remote).
#[cfg(any(feature = "grpc", feature = "http", feature = "mcp", feature = "acp"))]
pub(crate) fn redacted_message(err: &anyhow::Error, fixed: &str) -> String {
    from_anyhow(err)
        .filter(|coded| coded.facts.class == ErrorClass::Logical)
        .and_then(Coded::own_message)
        .unwrap_or_else(|| fixed.to_owned())
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
