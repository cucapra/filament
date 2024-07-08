use crate::{GenConfig, Instance, Tool, ToolOutput};
use itertools::Itertools;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tempfile as tmp;

enum OutDir {
    /// A temporary directory
    Tmp(tmp::TempDir),
    /// A user-specified directory
    User(PathBuf),
}

impl OutDir {
    /// Creates a new, temporary directory to store generated files.
    fn tmp() -> Self {
        let tmp_dir = tmp::tempdir().unwrap();
        log::info!("Generator output directory: {}", tmp_dir.path().display());
        Self::Tmp(tmp_dir)
    }

    fn user(path: PathBuf) -> Self {
        // If the path doesn't exist, create it
        if !path.exists() {
            log::info!("Creating output directory: {}", path.display());
            fs::create_dir_all(&path).unwrap();
        } else {
            log::info!("Generator output directory: {}", path.display());
        }
        Self::User(path)
    }

    fn opt(path: Option<PathBuf>) -> Self {
        match path {
            Some(p) => Self::user(p),
            None => Self::tmp(),
        }
    }

    fn path(&self) -> &Path {
        match self {
            Self::Tmp(tmp_dir) => tmp_dir.path(),
            Self::User(path) => path,
        }
    }
}

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
    output_dir: OutDir,

    /// Config file
    config: GenConfig,

    /// Dry-run instead of executing commands
    dry_run: bool,
}

impl GenExec {
    pub fn new(
        dry_run: bool,
        out_dir: Option<PathBuf>,
        config: GenConfig,
    ) -> Self {
        GenExec {
            tools: HashMap::default(),
            generated: HashMap::default(),
            output_dir: OutDir::opt(out_dir),
            dry_run,
            config,
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
        log::info!("Registering tool from file: `{}`", path.display());

        let desc = fs::read_to_string(path.clone()).unwrap();
        let mut tool: Tool = toml::from_str(&desc).unwrap();
        // Replace the globals with the ones from the config file if it exists
        if let Some(globals) = self.config.remove(&tool.name) {
            tool.globals = globals;
        }
        // Get the absolute path to the binary if it is relative
        let tool_path = PathBuf::from(&tool.path);
        if !tool_path.is_absolute() {
            tool.path = path
                .parent()
                .unwrap()
                .join(tool_path)
                .to_string_lossy()
                .to_string();
        }
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
        let path = self.output_dir.path().join(name);
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
        let out_file = self.gen_file(format!("{}.v", gen_name));
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
            name: gen_name,
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
