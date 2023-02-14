use crate::{
    errors::WithPos,
    utils::{GPosIdx, GSym},
};
use derivative::Derivative;

#[derive(Derivative, Clone)]
#[derivative(Hash, Eq, Debug, PartialOrd, Ord)]
pub struct Id {
    id: GSym,
    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    #[derivative(PartialOrd = "ignore")]
    #[derivative(Ord = "ignore")]
    pos: GPosIdx,
}

impl Id {
    pub fn new<S: ToString>(id: S) -> Self {
        Id {
            id: id.to_string().into(),
            pos: GPosIdx::UNKNOWN,
        }
    }
}

impl WithPos for Id {
    fn set_span(mut self, sp: GPosIdx) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> GPosIdx {
        self.pos
    }
}

/* =================== Impls for Id to make them easier to use ============== */

impl Default for Id {
    fn default() -> Self {
        Id::new("")
    }
}

impl std::fmt::Display for Id {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.id)
    }
}

impl AsRef<str> for Id {
    fn as_ref(&self) -> &str {
        self.id.as_str()
    }
}

impl From<&str> for Id {
    fn from(s: &str) -> Self {
        Id::new(s)
    }
}

impl From<String> for Id {
    fn from(s: String) -> Self {
        Id::new(s)
    }
}
impl PartialEq<Id> for Id {
    fn eq(&self, other: &Id) -> bool {
        self.id == other.id
    }
}
impl PartialEq<GSym> for Id {
    fn eq(&self, other: &GSym) -> bool {
        self.id == *other
    }
}
impl PartialEq<str> for Id {
    fn eq(&self, other: &str) -> bool {
        self.id == GSym::from(other)
    }
}
impl PartialEq<&str> for Id {
    fn eq(&self, other: &&str) -> bool {
        self.id == GSym::from(*other)
    }
}
impl PartialEq<&Id> for Id {
    fn eq(&self, other: &&Id) -> bool {
        self.id == other.id
    }
}
impl PartialEq<String> for Id {
    fn eq(&self, other: &String) -> bool {
        self.id == GSym::from(other)
    }
}

impl From<Id> for GSym {
    fn from(id: Id) -> Self {
        id.id
    }
}

impl From<&Id> for GSym {
    fn from(id: &Id) -> Self {
        id.id
    }
}
