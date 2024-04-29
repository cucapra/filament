// Defines the schema for a config configuration file
use std::collections::HashMap;

/// A tool that can generate external modules for Filament
pub type GenConfig = HashMap<String, HashMap<String, String>>;
