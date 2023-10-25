use std::fs;

fn main() {
    // Get the command-line arguments
    let opts: fil_gen::Opts = argh::from_env();
    // Deserialize the tool description
    let desc = fs::read_to_string(opts.tool).unwrap();
    let tool: fil_gen::Tool = toml::from_str(&desc).unwrap();
    // Deserialize the manifest file
    let manifest_str = fs::read_to_string(opts.manifest).unwrap();
    let manifest: fil_gen::Manifest = toml::from_str(&manifest_str).unwrap();
}
