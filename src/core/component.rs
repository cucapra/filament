use super::{Command, Id, Invoke, PortParam, Signature, Time, TimeRep};
use crate::errors::{Error, FilamentResult, WithPos};
use std::{collections::HashMap, fmt::Display};

/// A component in Filament
pub struct Component<T>
where
    T: Clone + TimeRep,
{
    // Signature of this component
    pub sig: Signature<T, u64>,

    /// Model for this component
    pub body: Vec<Command<T>>,
}

impl<T> Component<T>
where
    T: Clone + TimeRep,
{
    pub fn validate(body: &[Command<T>]) -> FilamentResult<()> {
        let mut is_low: Option<bool> = None;

        for con in body {
            match con {
                Command::Invoke(inv @ Invoke { ports, .. }) => match is_low {
                    Some(true) => {
                        if ports.is_some() {
                            return Err(Error::malformed(
                                "Malformed low-level component",
                            )
                            .add_note(
                                "Low-level component cannot use invoke with ports",
                                inv.copy_span(),
                            ));
                        }
                    }
                    Some(false) => {
                        if ports.is_none() {
                            return Err(Error::malformed(
                                "Malformed High-level component",
                            )
                            .add_note("High-level component must use invokes that specify all ports", inv.copy_span()));
                        }
                    }
                    None => is_low = Some(ports.is_none()),
                },
                Command::Fsm(fsm) => {
                    if let Some(false) = is_low {
                        return Err(Error::malformed("Malforemd High-level component")
                        .add_note("High-level components cannot use `fsm` to schedule execution", fsm.copy_span()));
                    } else if is_low.is_none() {
                        is_low = Some(true);
                    }
                }
                Command::Instance(_) | Command::Connect(_) => (),
            }
        }

        Ok(())
    }

    pub fn new(sig: Signature<T, u64>, body: Vec<Command<T>>) -> Self {
        Self { sig, body }
    }
}
impl Display for Component<Time<u64>> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} {{", self.sig)?;
        for com in &self.body {
            writeln!(f, "  {};", com)?;
        }
        writeln!(f, "}}")
    }
}

pub struct Namespace<T>
where
    T: Clone + TimeRep,
{
    /// Imported files
    pub imports: Vec<String>,

    /// Define externals and their files
    pub externs: Vec<(String, Vec<Signature<T, PortParam>>)>,

    /// Components defined in this file
    pub components: Vec<Component<T>>,
}

impl<T> Namespace<T>
where
    T: TimeRep + Clone,
{
    /// External signatures associated with the namespace
    pub fn signatures(&self) -> HashMap<Id, &Signature<T, PortParam>> {
        self.externs
            .iter()
            .flat_map(|(_, comps)| comps.iter().map(|s| (s.name.clone(), s)))
            .collect()
    }
}

impl Display for Namespace<Time<u64>> {
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
