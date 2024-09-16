use crate::{cmdline, ir_visitor::Construct};
use fil_ast as ast;
use fil_utils::{Error, FilamentResult};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

/// Completely parse all dependecies of a Filament program
pub struct Resolver {
    // Location of the library
    lib: Vec<PathBuf>,
    // Location of the base file
    input: PathBuf,
    // Files that have already been imported
    already_imported: HashSet<PathBuf>,
}

impl From<&cmdline::Opts> for Resolver {
    fn from(opts: &cmdline::Opts) -> Self {
        let mut lib = opts.library.clone();
        // Do this to resolve the issue mentioned in `cmdline.rs`
        // where default values don't work with argh
        lib.push(From::from("."));
        Self {
            lib,
            input: opts.input.clone(),
            already_imported: HashSet::new(),
        }
    }
}

impl Resolver {
    fn new(lib: Vec<PathBuf>, input: PathBuf) -> Self {
        let mut lib = lib.clone();
        lib.push(From::from("."));
        Self {
            lib,
            input,
            already_imported: HashSet::default(),
        }
    }

    /// Resolve import either using opts.library or relative the parent directory of the input file.
    fn resolve_import(
        &self,
        imp: &String,
        dir: &Path,
    ) -> FilamentResult<PathBuf> {
        // Resolve against the library path
        let mut lib_base = self.lib.clone();

        // Append current directory to the library path
        lib_base.push(dir.to_path_buf());

        let (lib_resolved, lib_not_resolved): (Vec<_>, Vec<_>) = lib_base
            .into_iter()
            .map(|p| p.join(imp))
            .partition(|p| p.exists());

        if lib_resolved.len() > 1 {
            // Canonicalize the paths to make sure the paths are the same
            let lib_canon = lib_resolved
                .iter()
                .map(|p| fs::canonicalize(p).unwrap())
                .collect::<HashSet<_>>();

            if lib_canon.len() > 1 {
                // There are multiple canonical paths
                Err(Error::misc(format!(
                    "Refusing to resolve ambiguous import: {}. Conflicting candidates found:\n{}",
                    imp,
                    lib_canon.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join("\n"),
                )))
            } else {
                // There is only one canonical path
                Ok(lib_resolved[0].clone())
            }
        } else if !lib_resolved.is_empty() {
            Ok(lib_resolved[0].clone())
        } else {
            Err(Error::misc(format!(
                "Could not resolve import path: {}. Searched in the following locations:\n{}",
                imp,
                lib_not_resolved
                    .iter()
                    .map(|p| p.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
            )))
        }
    }

    // Get absolute path for a relative path `ext_path` wrt to the parent of the file `file_path`.
    fn absolute(ext_path: String, base: &Path) -> String {
        let ext: PathBuf = ext_path.into();
        if ext.is_relative() {
            let mut base = base.to_path_buf();
            base.push(ext);
            base
        } else {
            ext
        }
        .to_string_lossy()
        .to_string()
    }

    /// Get the parent of a given file
    fn parent(p: &Path) -> PathBuf {
        let mut p = p.to_path_buf();
        p.pop();
        p
    }

    /// Returns the path if it has not already been parsed before
    fn add_import(&mut self, p: PathBuf) -> Option<PathBuf> {
        let c = fs::canonicalize(&p).unwrap();
        if self.already_imported.contains(&c) {
            None
        } else {
            self.already_imported.insert(c);
            Some(p)
        }
    }

    pub fn parse_namespace(&mut self) -> FilamentResult<ast::Namespace> {
        // Parse the top-level file
        let mut ns = ast::FilamentParser::parse_file(&self.input)?;

        // Extern are resolved to thier absolute path relative to the input file.
        let base = Self::parent(&self.input);
        let mut imports: Vec<PathBuf> = ns
            .imports
            .drain(..)
            .filter_map(|s| match self.resolve_import(&s, &base) {
                Ok(file) => self.add_import(file).map(Ok),
                f @ Err(_) => Some(f),
            })
            .collect::<FilamentResult<_>>()?;

        ns.externs = ns
            .externs
            .drain(..)
            .map(|ext| ext.map_path(|p| Self::absolute(p, &base)))
            .collect();

        while let Some(path) = imports.pop() {
            let mut imp = ast::FilamentParser::parse_file(&path)?;
            let base = Self::parent(&path);
            imp.components.append(&mut ns.components);
            ns.components = imp.components;
            ns.externs.extend(
                imp.externs
                    .into_iter()
                    .map(|ext| ext.map_path(|p| Self::absolute(p, &base))),
            );
            let base = Self::parent(&path);
            imports.extend(
                imp.imports
                    .into_iter()
                    .filter_map(|s| match self.resolve_import(&s, &base) {
                        Ok(file) => self.add_import(file).map(Ok),
                        f @ Err(_) => Some(f),
                    })
                    .collect::<FilamentResult<Vec<_>>>()?,
            );
        }

        log::trace!("Imported: {:#?}", self.already_imported);
        log::trace!(
            "Components: {:#?}",
            ns.components
                .iter()
                .map(|c| c.sig.name.as_ref())
                .collect::<Vec<_>>()
        );
        log::trace!(
            "Externs: {:#?}",
            ns.externs
                .iter()
                .flat_map(|ast::Extern { comps, .. }| comps)
                .map(|c| c.name.as_ref())
                .collect::<Vec<_>>()
        );
        Ok(ns)
    }
}
