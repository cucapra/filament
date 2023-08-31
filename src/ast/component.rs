use super::{Command, Id, Signature};

#[derive(Default)]
/// A component in Filament
pub struct Component {
    // Signature of this component
    pub sig: Signature,
    /// Model for this component
    pub body: Vec<Command>,
}

impl Component {
    pub fn new(sig: Signature, body: Vec<Command>) -> Self {
        Self { sig, body }
    }
}

#[derive(Default)]
pub struct Namespace {
    /// Imported files
    pub imports: Vec<String>,
    /// Define externals and their files
    pub externs: Vec<(String, Vec<Signature>)>,
    /// Components defined in this file
    pub components: Vec<Component>,
    /// Top-level component id
    pub toplevel: String,
}

impl Namespace {
    /// External signatures associated with the namespace
    pub fn externals(&self) -> impl Iterator<Item = (Id, &Signature)> {
        self.externs
            .iter()
            .flat_map(|(_, comps)| comps.iter().map(|s| (*s.name.inner(), s)))
    }

    /// Get the index to the top-level component.
    /// Currently, this is the distinguished "main" component
    pub fn main_idx(&self) -> Option<usize> {
        self.components.iter().position(|c| {
            c.sig.name.inner() == &Id::from(self.toplevel.clone())
        })
    }
}
