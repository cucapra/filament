use std::path::PathBuf;

use super::{Command, Id, Signature};
use fil_gen as gen;
use gen::GenConfig;

#[derive(Default)]
/// A external or generate definition in Filament
pub struct Extern {
    pub path: String,
    pub comps: Vec<Signature>,
    /// name of the tool that generates this module
    pub gen: Option<String>,
}
impl Extern {
    pub fn new(
        path: String,
        comps: Vec<Signature>,
        gen: Option<String>,
    ) -> Self {
        Self { path, comps, gen }
    }

    pub fn map_path<F>(mut self, func: F) -> Self
    where
        F: Fn(String) -> String,
    {
        self.path = func(self.path);
        self
    }
}

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

pub struct Namespace {
    /// Imported files
    pub imports: Vec<String>,
    /// Define externals and their files
    pub externs: Vec<Extern>,
    /// Components defined in this file
    pub components: Vec<Component>,
    /// Top-level component id
    pub toplevel: String,
    /// Top level bindings
    pub bindings: Vec<u64>,
}

impl Namespace {
    pub fn new(toplevel: String) -> Self {
        Self {
            imports: Vec::default(),
            externs: Vec::default(),
            components: Vec::default(),
            bindings: Vec::default(),
            toplevel,
        }
    }

    /// Returns true if the namespace declares at least one generative module
    pub fn requires_gen(&self) -> bool {
        self.externs.iter().any(|Extern { gen, .. }| gen.is_some())
    }

    /// Initialize the generator executor using the given generate definitions.
    /// REQUIRES: The tools definitions must be in files with absolute paths.
    /// The folder containing the generated files is deleted when the destructor
    /// for GenExec runs.
    pub fn init_gen(
        &self,
        out_dir: Option<PathBuf>,
        config: GenConfig,
    ) -> gen::GenExec {
        let mut gen_exec = gen::GenExec::new(false, out_dir, config);
        for Extern { path, gen, .. } in &self.externs {
            let Some(tool_name) = gen else {
                continue;
            };
            let tool = gen_exec.register_tool_from_file(path.into());
            assert!(&tool.name == tool_name, "Generate definition for tool `{}` does not match the tool name `{}`", tool_name, tool.name);
        }
        gen_exec
    }

    /// External signatures associated with the namespace
    pub fn externals(&self) -> impl Iterator<Item = (Id, &Signature)> {
        self.externs.iter().flat_map(|Extern { comps, .. }| {
            comps.iter().map(|s| (*s.name.inner(), s))
        })
    }

    /// Get the index to the top-level component.
    /// Currently, this is the distinguished "main" component
    pub fn main_idx(&self) -> Option<usize> {
        self.components.iter().position(|c| {
            c.sig.name.inner() == &Id::from(self.toplevel.clone())
        })
    }
}
