use crate::Pool;
use std::sync;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GSym(u32);

fn singleton() -> &'static Pool {
    static SINGLETON: sync::LazyLock<Pool> = sync::LazyLock::new(Pool::new);

    &SINGLETON
}

impl GSym {
    /// Intern a string into the global symbol table.
    pub fn new(s: impl AsRef<str>) -> Self {
        s.as_ref().into()
    }

    /// Convert this symbol into the string in the static, global symbol table.
    pub fn as_str(&self) -> &'static str {
        (*self).into()
    }
}

impl From<&str> for GSym {
    fn from(s: &str) -> Self {
        GSym(singleton().get_or_intern(s))
    }
}

impl From<String> for GSym {
    fn from(s: String) -> Self {
        GSym(singleton().get_or_intern(&s))
    }
}

impl From<&String> for GSym {
    fn from(s: &String) -> Self {
        GSym(singleton().get_or_intern(s))
    }
}

impl From<GSym> for &'static str {
    fn from(sym: GSym) -> Self {
        singleton().get(sym.0)
    }
}

impl std::fmt::Debug for GSym {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(self.as_str(), f)
    }
}

impl std::fmt::Display for GSym {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(self.as_str(), f)
    }
}
