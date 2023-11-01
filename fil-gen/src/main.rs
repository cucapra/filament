use fil_gen::GenExec;
use std::fs;

fn main() {
    let opts: fil_gen::Opts = argh::from_env();

    env_logger::Builder::from_default_env()
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        .filter_level(opts.log_level)
        .target(env_logger::Target::Stderr)
        .init();
    let mut gen = GenExec::new(opts.dry_run);

    // Deserialize the tool description
    let desc = fs::read_to_string(opts.tool).unwrap();
    let tool: fil_gen::Tool = toml::from_str(&desc).unwrap();
    let name = tool.name.clone();
    gen.register_tool(tool);
    // Deserialize the manifest file
    let manifest_str = fs::read_to_string(opts.manifest).unwrap();
    let manifest: fil_gen::Manifest = toml::from_str(&manifest_str).unwrap();

    for instance in &manifest.modules {
        gen.gen_instance(&name, instance);
    }
}
