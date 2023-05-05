use super::SExp;
use crate::{core::Id, diagnostics::InfoIdx, errors::Error};
use itertools::Itertools;

/// An obligation generated by during type checking.
pub struct Obligation {
    /// Variables that need to be defined for this obligation
    pub defines: Vec<Id>,
    /// Path condition under which this obligation was generated
    path_cond: Vec<SExp>,
    /// The constraint that needs to be satisfied
    cons: SExp,
    /// Why this obligation was created
    reason: String,
    /// Any extra information that can be used to debug the obligation
    info: Vec<InfoIdx>,
}

impl Obligation {
    /// Construct a new obligation
    pub fn new<S: ToString>(cons: SExp, reason: S) -> Self {
        Self {
            cons,
            reason: reason.to_string(),
            info: vec![],
            path_cond: Vec::new(),
            defines: Vec::new(),
        }
    }

    /// Add defines to the obligation
    pub fn with_defines<I>(mut self, defines: I) -> Self
    where
        I: IntoIterator<Item = Id>,
    {
        self.defines.extend(defines.into_iter());
        self
    }

    /// Add assumptions to the obligation
    pub fn with_path_cond<I, S>(mut self, assumes: I) -> Self
    where
        S: Into<SExp>,
        I: IntoIterator<Item = S>,
    {
        self.path_cond.extend(assumes.into_iter().map(|x| x.into()));
        self
    }

    /// Adds extra information to the obligation
    pub fn add_note(mut self, info: InfoIdx) -> Self {
        self.info.push(info);
        self
    }

    /// The constraint associated with this obligation
    pub fn constraint(&self) -> SExp {
        if self.path_cond.is_empty() {
            self.cons.clone()
        } else {
            let assumes = format!(
                "(and {})",
                self.path_cond
                    .iter()
                    .map(|x| x.to_string())
                    .collect_vec()
                    .join(" ")
            );
            SExp(format!("(=> {} {})", assumes, &self.cons))
        }
    }

    /// Turn this Obligation into an Error
    pub fn error(self) -> Error {
        self.into()
    }
}

impl From<Obligation> for Error {
    fn from(v: Obligation) -> Self {
        let mut e = Error::misc(v.reason);
        for i in v.info {
            e = e.add_note(i);
        }
        e
    }
}
