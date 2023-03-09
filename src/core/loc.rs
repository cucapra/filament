use crate::utils::GPosIdx;
use std::fmt::Display;
use std::hash::Hash;

#[derive(Clone)]
/// A type that contains several position objects and contains and inner value.
pub struct Loc<T> {
    inner: T,
    pos: GPosIdx,
}

impl<T> Loc<T> {
    /// Construct a new `Loc` with the given inner value and no positions.
    pub fn new(inner: T, pos: GPosIdx) -> Self {
        Self { inner, pos }
    }

    /// A value with no position information.
    pub fn unknown(inner: T) -> Self {
        Self {
            inner,
            pos: GPosIdx::UNKNOWN,
        }
    }

    /// Get the position associted with this `Loc`.
    pub fn pos(&self) -> GPosIdx {
        self.pos
    }

    /// Set the position at the index `i` to the given position.
    /// Should be wrapped by the type to represent the semantic meaning of the position.
    pub fn set_pos(&mut self, pos: GPosIdx) {
        self.pos = pos;
    }

    /// Get a reference to  inner value.
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Get a mutable reference to the inner value.
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Get the inner value and the position
    pub fn split(self) -> (T, GPosIdx) {
        (self.inner, self.pos)
    }

    /// Get the inner value and drop the position
    pub fn take(self) -> T {
        self.inner
    }

    /// Map over the inner value.
    pub fn map<U, F>(self, mut f: F) -> Loc<U>
    where
        F: FnMut(T) -> U,
    {
        Loc {
            inner: f(self.inner),
            pos: self.pos,
        }
    }
}

impl<T> std::ops::Deref for Loc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> std::ops::DerefMut for Loc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T: Display> Display for Loc<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl<T: PartialEq> PartialEq for Loc<T> {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}
impl<T: Eq> Eq for Loc<T> {}

impl<T: PartialOrd> PartialOrd for Loc<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl<T: Hash> Hash for Loc<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state)
    }
}
