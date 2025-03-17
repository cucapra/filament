use super::{Command, Id, Signature};
use fgen::GenConfig;
use fil_gen as fgen;
use fil_utils::{self as utils, AttrCtx};
use std::path::PathBuf;

#[derive(Default)]
/// A external or generate definition in Filament
pub struct Extern {
    pub path: String,
    pub comps: Vec<Signature>,
    /// name of the tool that generates this module
    pub gen_tool: Option<String>,
}
impl Extern {
    pub fn new(
        path: String,
        comps: Vec<Signature>,
        gen_tool: Option<String>,
    ) -> Self {
        Self {
            path,
            comps,
            gen_tool,
        }
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

#[derive(Default)]
pub struct Namespace {
    /// Imported files
    pub imports: Vec<String>,
    /// Define externals and their files
    pub externs: Vec<Extern>,
    /// Components defined in this file
    pub components: Vec<Component>,
    /// Top level bindings
    pub bindings: Vec<u64>,
}

impl Namespace {
    /// Returns true if the namespace declares at least one generative module
    pub fn requires_gen(&self) -> bool {
        self.externs
            .iter()
            .any(|Extern { gen_tool, .. }| gen_tool.is_some())
    }

    /// Initialize the generator executor using the given generate definitions.
    /// REQUIRES: The tools definitions must be in files with absolute paths.
    /// The folder containing the generated files is deleted when the destructor
    /// for GenExec runs.
    pub fn init_gen(
        &self,
        out_dir: Option<PathBuf>,
        config: GenConfig,
    ) -> fgen::GenExec {
        let mut gen_exec = fgen::GenExec::new(false, out_dir, config);
        for Extern { path, gen_tool, .. } in &self.externs {
            let Some(tool_name) = gen_tool else {
                continue;
            };
            let tool = gen_exec.register_tool_from_file(path.into());
            assert!(
                &tool.name == tool_name,
                "Generate definition for tool `{}` does not match the tool name `{}`",
                tool_name,
                tool.name
            );
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
        self.components
            .iter()
            .position(|c| c.sig.attributes.has(utils::CompBool::TopLevel))
    }

    /// Get the toplevel component name
    pub fn toplevel(&self) -> Option<&str> {
        self.main_idx()
            .map(|idx| self.components[idx].sig.name.inner().as_ref())
    }
}
