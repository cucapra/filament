/// The filament gen tool
#[derive(argh::FromArgs)]
pub struct Opts {
    /// location of the tool description
    #[argh(option, short = 't')]
    pub tool: String,
    /// location of the manifest file
    #[argh(option, short = 'm')]
    pub manifest: String,
}
