use super::{Command, Id, Invoke, Signature};
use crate::errors::{Error, FilamentResult};
use std::fmt::Display;

/// A component in Filament
pub struct Component {
    // Signature of this component
    pub sig: Signature,

    /// Model for this component
    pub body: Vec<Command>,
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
                Command::Fsm(_) => {
                    if let Some(false) = is_low {
                        return Err(Error::malformed("Malforemd High-level component: Cannot use the `fsm` construct"));
                    } else if is_low.is_none() {
                        is_low = Some(true);
                    }
                }
                Command::Instance(_)
                | Command::Connect(_)
                | Command::Bundle(_)
                | Command::If(_)
                | Command::ForLoop(_) => (),
            }
        }

        Ok(())
    }

    pub fn new(sig: Signature, body: Vec<Command>) -> Self {
        Self { sig, body }
    }
}
impl Display for Component {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} {{", self.sig)?;
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
}

impl Namespace {
    /// External signatures associated with the namespace
    pub fn signatures(&self) -> impl Iterator<Item = (Id, &Signature)> {
        self.externs
            .iter()
            .flat_map(|(_, comps)| comps.iter().map(|s| (*s.name.inner(), s)))
    }

    /// Get the index to the top-level component.
    /// Currently, this is the distinguished "main" component
    pub fn main_idx(&self) -> Option<usize> {
        self.components
            .iter()
            .position(|c| c.sig.name.inner() == "main")
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
