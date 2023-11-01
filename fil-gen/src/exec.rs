use itertools::Itertools;

use crate::{Instance, Tool, ToolOutput};
use std::{collections::HashMap, fs, path::PathBuf, process::Command};

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

    /// Directory to store all the generated files
    output_dir: PathBuf,

    /// Dry-run instead of executing commands
    dry_run: bool,
}

impl GenExec {
    pub fn new(output_dir: PathBuf, dry_run: bool) -> Self {
        if !output_dir.exists() {
            std::fs::create_dir_all(output_dir.clone()).unwrap();
        }
        GenExec {
            tools: HashMap::default(),
            generated: HashMap::default(),
            output_dir,
            dry_run,
        }
    }

    /// Should we dry run instead of executing commands?
    pub fn dry_run(&mut self, dry_run: bool) {
        self.dry_run = dry_run;
    }

    /// Check if a tool is registered
    pub fn has_tool(&self, tool: &str) -> bool {
        self.tools.contains_key(tool)
    }

    /// Register a tool by reading its definition from a file
    pub fn register_tool_from_file(&mut self, path: PathBuf) -> &Tool {
        let desc = fs::read_to_string(path).unwrap();
        let tool: Tool = toml::from_str(&desc).unwrap();
        let name = tool.name.clone();
        self.register_tool(tool);
        self.tools.get(&name).unwrap()
    }

    /// Register a new tool
    pub fn register_tool(&mut self, tool: Tool) {
        assert!(
            !self.has_tool(&tool.name),
            "Tool already registered: `{}`",
            tool.name
        );
        tool.validate();
        log::info!("Registering tool: `{}`", tool.name);
        self.tools.insert(tool.name.clone(), tool);
    }

    /// Generate a new file with the given name in the output directory
    pub fn gen_file(&mut self, name: String) -> PathBuf {
        let path = self.output_dir.join(name);
        assert!(!path.exists(), "File already exists: `{}`", path.display());
        log::info!("Generating file: `{}`", path.display());
        path
    }

    /// Execute a particular manifest to generate instances
    pub fn gen_instance(
        &mut self,
        tool: &str,
        instance: &Instance,
    ) -> ToolOutput {
        assert!(self.has_tool(tool), "Unknown tool: `{tool}");

        if let Some(output) = self.generated.get(tool) {
            if let Some(out) = output.get(instance) {
                log::info!("Using cached output for `{}`", instance);
                return out.clone();
            }
        }

        let tool = self.tools.get(tool).unwrap().clone();
        let Some(module) = tool.get_module(&instance.name) else {
            unreachable!(
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

        let mut binding = module
            .parameters
            .iter()
            .cloned()
            .zip(instance.parameters.iter().cloned())
            .chain(tool.globals.clone())
            .collect_vec();

        let gen_name = module.name(&binding).unwrap();
        binding.push(("NAME_FORMAT".to_string(), gen_name.clone()));

        assert!(
            tool.requires_out_file.is_some() && tool.requires_out_file.unwrap(),
            "tool does not support $OUT_FILE"
        );
        let out_file = self.gen_file(format!("{}.v", gen_name.clone()));
        binding.push((
            "OUT_FILE".to_string(),
            out_file.to_string_lossy().to_string(),
        ));

        let args = module.cli(&binding).unwrap();
        log::info!("Executing: {} {}", tool.path, args);

        // Return early in dry-run mode
        if self.dry_run {
            return ToolOutput::default();
        }

        let output = Command::new(&tool.path)
            .args(args.split_whitespace())
            .output()
            .expect("Failed to execute tool");

        log::info!(
            "Command exited with status: {}.\nSTDOUT:\n{}\nSTDERR:{}\n",
            output.status,
            std::str::from_utf8(&output.stdout).unwrap(),
            std::str::from_utf8(&output.stderr).unwrap()
        );

        // Parse bindings for existential parameters from the output
        let mut key_map: HashMap<String, String> = HashMap::default();
        for line in std::str::from_utf8(&output.stdout).unwrap().lines() {
            let mut parts = line.split('=');
            if let (Some(name), Some(val)) = (parts.next(), parts.next()) {
                key_map.insert(name.trim().to_string(), val.trim().to_string());
            }
        }
        log::info!("Parsed key-values: {:?}", key_map);
        let exist_params = module
            .outputs
            .iter()
            .map(|(param, out_name)| {
                let val = key_map
                    .get(out_name)
                    .unwrap_or_else(|| unreachable!("tool did not produce binding for existential parameter: {out_name}"));
                (param.clone(), val.clone())
            })
            .collect();
        log::info!("Existential parameters: {:?}", exist_params);

        // Generate the output and cache the result
        let out = ToolOutput {
            file: out_file,
            exist_params,
        };
        self.generated
            .entry(tool.name.clone())
            .or_default()
            .insert(instance.clone(), out.clone());

        out
    }
}
