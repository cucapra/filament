use std::sync;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GSym(u32);

type Pool = boxcar::Vec<String>;

fn singleton() -> &'static Pool {
    static SINGLETON: sync::LazyLock<Pool> =
        sync::LazyLock::new(boxcar::Vec::new);

    &SINGLETON
}

fn get_or_intern(pool: &Pool, s: &str) -> u32 {
    let idx = pool.iter().position(|(_, x)| x == s);
    match idx {
        Some(idx) => idx as u32,
        None => {
            let idx = pool.count();
            pool.push(s.to_owned());
            idx as u32
        }
    }
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
        GSym(get_or_intern(singleton(), s))
    }
}

impl From<String> for GSym {
    fn from(s: String) -> Self {
        GSym(get_or_intern(singleton(), &s))
    }
}

impl From<&String> for GSym {
    fn from(s: &String) -> Self {
        GSym(get_or_intern(singleton(), s))
    }
}

impl From<GSym> for &'static str {
    fn from(sym: GSym) -> Self {
        singleton()[sym.0 as usize].as_str()
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
