use super::{Command, Fsm, Id, Invoke, Signature};
use crate::errors::{Error, FilamentResult};
use std::{fmt::Display};


/// A component in Filament
pub struct Component {
    // Signature of this component
    pub sig: Signature,
    /// Model for this component
    pub body: Vec<Command>,
    /// FSMs associated with this component
    pub fsms: Vec<Fsm>,
}

impl Component {
    pub fn validate(body: &[Command]) -> FilamentResult<()> {
        let mut is_low: Option<bool> = None;

        for con in body {
            match con {
                Command::Invoke(Invoke { ports, .. }) => match is_low {
                    Some(true) => {
                        if ports.is_some() {
                            return Err(Error::malformed(
                                "Malformed low-level component: Low-level component cannot use invoke with ports",
                            ));
                        }
                    }
                    Some(false) => {
                        if ports.is_none() {
                            return Err(Error::malformed(
                                "Malformed High-level component: Invokes must specify all ports",
                            ));
                        }
                    }
                    None => is_low = Some(ports.is_none()),
                },
                Command::Instance(_)
                | Command::Connect(_)
                | Command::Fact(_)
                | Command::Bundle(_)
                | Command::If(_)
                | Command::ForLoop(_) => (),
            }
        }

        Ok(())
    }

    pub fn new(sig: Signature, body: Vec<Command>) -> Self {
        Self {
            sig,
            body,
            fsms: Vec::new(),
        }
    }
}
impl Display for Component {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} {{", self.sig)?;
        for fsm in &self.fsms {
            writeln!(f, "  {}", fsm)?;
        }
        for com in &self.body {
            writeln!(f, "  {};", com)?;
        }
        writeln!(f, "}}")
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
    pub toplevel: String
}

impl Namespace {
    /// External signatures associated with the namespace
    pub fn externals(&self) -> impl Iterator<Item = (Id, &Signature)> {
        self.externs
            .iter()
            .flat_map(|(_, comps)| comps.iter().map(|s| (*s.name.inner(), s)))
    }

    pub fn signatures(&self) -> impl Iterator<Item = (Id, &Signature)> {
        self.externals().chain(
            self.components
                .iter()
                .map(|c| (*c.sig.name.inner(), &c.sig)),
        )
    }

    /// Get the index to the top-level component.
    /// Currently, this is the distinguished "main" component
    pub fn main_idx(&self) -> Option<usize> {
        self.components
            .iter()
            .position(|c| c.sig.name.inner() == &Id::from(self.toplevel.clone()))
    }
}

impl Display for Namespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for imp in &self.imports {
            writeln!(f, "import \"{}\";", imp)?;
        }
        for (path, sigs) in &self.externs {
            writeln!(f, "extern \"{}\" {{", path)?;
            for sig in sigs {
                writeln!(f, "  {};", sig)?;
            }
            writeln!(f, "}}")?;
        }
        for comp in &self.components {
            writeln!(f, "{}", comp)?;
        }
        Ok(())
    }
}
