use std::{mem, sync};
use string_interner::{
    StringInterner, backend::BucketBackend, symbol::SymbolU32,
};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GSym(SymbolU32);

type Pool = StringInterner<BucketBackend>;

fn singleton() -> *mut Pool {
    static mut SINGLETON: mem::MaybeUninit<Pool> = mem::MaybeUninit::uninit();
    static ONCE: sync::Once = sync::Once::new();

    // SAFETY:
    // - writing to the singleton is OK because we only do it one time
    // - the ONCE guarantees that SINGLETON is init'ed before assume_init_ref
    #[allow(static_mut_refs)]
    unsafe {
        ONCE.call_once(|| {
            SINGLETON.write(Pool::new());
        });

        SINGLETON.as_mut_ptr() as *mut Pool
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
        GSym(unsafe { singleton().as_mut().unwrap().get_or_intern(s) })
    }
}

impl From<String> for GSym {
    fn from(s: String) -> Self {
        GSym(unsafe { singleton().as_mut().unwrap().get_or_intern(&s) })
    }
}

impl From<&String> for GSym {
    fn from(s: &String) -> Self {
        GSym(unsafe { singleton().as_mut().unwrap().get_or_intern(s) })
    }
}

impl From<GSym> for &'static str {
    fn from(sym: GSym) -> Self {
        unsafe { singleton().as_ref().unwrap().resolve(sym.0).unwrap() }
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
