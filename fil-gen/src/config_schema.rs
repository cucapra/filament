//! Defines the schema for a config configuration file
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, Debug, Deserialize)]
/// A tool that can generate external modules for Filament
pub struct Config {
    /// Globals map
    pub globals: HashMap<String, HashMap<String, String>>,
}
