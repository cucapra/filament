use crate::errors::{self, WithPos};
use derivative::Derivative;

#[derive(Derivative, Clone, PartialOrd, Ord)]
#[derivative(Hash, Eq, Debug)]
pub struct Id {
    id: String,
    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    pos: Option<errors::Span>,
}

impl Id {
    pub fn id(&self) -> &str {
        self.id.as_ref()
    }
}
impl WithPos for Id {
    fn set_span(mut self, sp: Option<errors::Span>) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> Option<errors::Span> {
        self.pos.clone()
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
        &self.id
    }
}

impl From<&str> for Id {
    fn from(s: &str) -> Self {
        Id {
            id: s.to_string(),
            pos: None,
        }
    }
}

impl From<String> for Id {
    fn from(s: String) -> Self {
        Id { id: s, pos: None }
    }
}

impl PartialEq<str> for Id {
    fn eq(&self, other: &str) -> bool {
        self.id == other
    }
}

impl<S: AsRef<str>> PartialEq<S> for Id {
    fn eq(&self, other: &S) -> bool {
        self.id == other.as_ref()
    }
}
