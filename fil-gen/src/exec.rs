use itertools::Itertools;

use crate::{Instance, Tool, ToolOutput};
use std::collections::HashMap;

/// The main execution management engine for Filament's `gen` framework.
/// Manages registering new tools and executing the tools to generate particular instances.
pub struct GenExec {
    /// Tools registered with the execution engine
    tools: HashMap<String, Tool>,

    /// Instances that have already been generated
    generated: HashMap<
        /*tool=*/ String,
        HashMap</*instance=*/ Instance, ToolOutput>,
    >,
}

impl GenExec {
    /// Check if a tool is registered
    pub fn has_tool(&self, tool: &str) -> bool {
        self.tools.contains_key(tool)
    }

    /// Register a new tool
    pub fn register_tool(&mut self, tool: Tool) {
        assert!(
            !self.has_tool(&tool.name),
            "Tool already registered: `{}`",
            tool.name
        );
        log::info!("Registering tool: `{}`", tool.name);
        self.tools.insert(tool.name.clone(), tool);
    }

    /// Execute a particular manifest to generate instances
    pub fn gen_instance(
        &mut self,
        tool: &str,
        instance: Instance,
    ) -> ToolOutput {
        assert!(self.has_tool(tool), "Unknown tool: `{tool}");

        if let Some(output) = self.generated.get(tool) {
            if let Some(output) = output.get(&instance) {
                log::info!("Using cached output for `{}`", instance);
                return output.clone();
            }
        }

        let tool = self.tools.get(tool).unwrap();
        let Some(module) = tool.get_module(&instance.name) else {
            panic!(
                "Tool `{}' does not define module `{}`",
                tool.name, instance.name
            );
        };

        assert!(
            module.parameters.len() == instance.parameters.len(),
            "Module `{}' has {} parameters, but the manifest specifies {} parameters",
            instance.name,
            module.parameters.len(),
            instance.parameters.len()
        );

        let binding = module
            .parameters
            .iter()
            .cloned()
            .zip(instance.parameters.iter().cloned())
            .collect_vec();

        unimplemented!()
    }
}
