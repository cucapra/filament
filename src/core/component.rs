use super::{Command, Id, Invoke, Signature, TimeRep};
use crate::errors::{Error, FilamentResult, WithPos};
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

/// A component in Filament
pub struct Component<T>
where
    T: Clone + TimeRep,
{
    // Signature of this component
    pub sig: Signature<T>,

    /// Model for this component
    pub body: Vec<Command<T>>,
}

impl<T> Component<T>
where
    T: Clone + TimeRep,
{
    pub fn validate(
        sig: &Signature<T>,
        body: &[Command<T>],
    ) -> FilamentResult<()> {
        let is_low = !sig.interface_signals.is_empty();

        if is_low {
            // All events should have corresponding interface signals
            let defined_interfaces = sig
                .interface_signals
                .iter()
                .map(|id| &id.event)
                .cloned()
                .collect();
            let all_events =
                sig.abstract_vars.iter().cloned().collect::<HashSet<_>>();

            if let Some(ev) = all_events.difference(&defined_interfaces).next()
            {
                return Err(Error::malformed(format!("Low-level component does not have define interface port for event `{ev}`")));
            }

            // There should be no high-level invokes
            for con in body {
                if let Command::Invoke(inv @ Invoke { ports, .. }) = &con {
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
            }
        } else {
            // There should be no FSM constructs or low-level invokes
            for con in body {
                match &con {
                    Command::Invoke(inv @ Invoke { ports, .. }) => {
                        if ports.is_none() {
                            return Err(Error::malformed(
                                "Malformed High-level component",
                            )
                            .add_note("High-level component must use invokes that specify all ports", inv.copy_span()))
                        }
                    }
                    Command::Fsm(fsm) => {
                        return Err(Error::malformed("Malforemd High-level component")
                        .add_note("High-level components cannot use `fsm` to schedule execution", fsm.copy_span()))
                    }
                    _ => (),
                }
            }
        }

        Ok(())
    }

    pub fn new(
        sig: Signature<T>,
        body: Vec<Command<T>>,
    ) -> FilamentResult<Self> {
        Self::validate(&sig, &body)?;
        Ok(Self { sig, body })
    }
}
impl<T> Display for Component<T>
where
    T: Clone + TimeRep + Display,
{
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
    pub externs: Vec<(String, Vec<Signature<T>>)>,

    /// Components defined in this file
    pub components: Vec<Component<T>>,
}

impl<T> Namespace<T>
where
    T: TimeRep + Clone,
{
    /// External signatures associated with the namespace
    pub fn signatures(&self) -> HashMap<Id, &Signature<T>> {
        self.externs
            .iter()
            .flat_map(|(_, comps)| comps.iter().map(|s| (s.name.clone(), s)))
            .collect()
    }
}

impl<T> Display for Namespace<T>
where
    T: Clone + TimeRep + Display,
{
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
