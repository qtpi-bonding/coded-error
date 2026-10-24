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
