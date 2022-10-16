use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    cmdline,
    errors::{self, FilamentResult},
    frontend,
};

/// Completely parse all dependecies of a Filament program
pub struct Resolver {
    // Location of the library
    lib: PathBuf,
    // Location of the base file
    input: PathBuf,
    // Files that have already been imported
    already_imported: HashSet<PathBuf>,
}

impl From<&cmdline::Opts> for Resolver {
    fn from(opts: &cmdline::Opts) -> Self {
        Self {
            lib: opts.library.clone(),
            input: opts.input.clone(),
            already_imported: HashSet::new(),
        }
    }
}

impl Resolver {
    /// Resolve import either using opts.library or relative the parent directory of the input file.
    fn resolve_import(
        &self,
        imp: &String,
        dir: &Path,
    ) -> FilamentResult<PathBuf> {
        // Resolve against the library path
        let mut lib_base = self.lib.clone();
        lib_base.push(imp);

        // Resove against base
        let mut cur_base = dir.to_path_buf();
        cur_base.push(imp);
        if cur_base.exists() && lib_base.exists() {
            Err(errors::Error::misc(format!(
                "Refusing to resolve ambiguous import: {}. Both {} and {} exist.",
                imp,
                fs::canonicalize(lib_base).unwrap().display(),
                fs::canonicalize(cur_base).unwrap().display(),
            )))
        } else if cur_base.exists() {
            Ok(cur_base)
        } else if lib_base.exists() {
            Ok(lib_base)
        } else {
            Err(errors::Error::misc(format!(
                "Could not resolve import path: {}. Neither {} nor {} exist.",
                imp,
                lib_base.display(),
                cur_base.display()
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

    pub fn parse_namespace(
        &mut self,
    ) -> FilamentResult<frontend::ast::Namespace> {
        // Parse the top-level file
        let mut ns = frontend::FilamentParser::parse_file(&self.input)?;

        // Extern are resolved to thier absolute path relative to the input file.
        let base = Self::parent(&self.input);
        let mut imports: Vec<PathBuf> = ns
            .imports
            .drain(..)
            .filter_map(|s| match self.resolve_import(&s, &base) {
                Ok(file) => self.add_import(file).map(|f| Ok(f)),
                f @ Err(_) => Some(f),
            })
            .collect::<FilamentResult<_>>()?;

        ns.externs = ns
            .externs
            .drain(..)
            .map(|(p, imps)| (Self::absolute(p, &base), imps))
            .collect();

        while let Some(path) = imports.pop() {
            let mut imp = frontend::FilamentParser::parse_file(&path)?;
            let base = Self::parent(&path);
            log::info!(
                "Adding components from {}: {:?}",
                path.display(),
                imp.components
                    .iter()
                    .map(|c| c.sig.name.id().clone())
                    .collect::<Vec<_>>()
            );
            ns.components.append(&mut imp.components);
            ns.externs.extend(
                imp.externs
                    .into_iter()
                    .map(|(p, imps)| (Self::absolute(p, &base), imps)),
            );
            let base = Self::parent(&path);
            imports.extend(
                imp.imports
                    .into_iter()
                    .filter_map(|s| match self.resolve_import(&s, &base) {
                        Ok(file) => self.add_import(file).map(|f| Ok(f)),
                        f @ Err(_) => Some(f),
                    })
                    .collect::<FilamentResult<Vec<_>>>()?,
            );
        }
        log::info!("Imported: {:#?}", self.already_imported);
        log::info!(
            "Components: {:#?}",
            ns.components
                .iter()
                .map(|c| c.sig.name.id().clone())
                .collect::<Vec<_>>()
        );
        log::info!(
            "Externs: {:#?}",
            ns.externs
                .iter()
                .flat_map(|(_, comps)| comps)
                .map(|c| c.name.id().clone())
                .collect::<Vec<_>>()
        );
        Ok(ns)
    }
}
