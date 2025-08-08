use super::Underlying;
use fil_ir::{self as ir};

/// Enhanced parameter resolver that provides better debugging and scope management
pub struct ParamResolver {
    scopes: Vec<ParamScope>,
    debug_mode: bool,
}

#[derive(Clone)]
struct ParamScope {
    name: String,
    bindings: Vec<(Underlying<ir::Param>, u64)>,
}

impl ParamResolver {
    pub fn new() -> Self {
        Self {
            scopes: vec![ParamScope {
                name: "base".to_string(),
                bindings: vec![],
            }],
            debug_mode: std::env::var("FIL_MONO_DEBUG").is_ok(),
        }
    }

    /// Create a new resolver with initial bindings (for component signature parameters)
    pub fn with_bindings(bindings: Vec<(Underlying<ir::Param>, u64)>) -> Self {
        let mut resolver = Self::new();
        resolver.scopes[0].bindings = bindings;
        resolver
    }

    /// Execute a function within a new parameter scope
    pub fn with_scope<F, R>(
        &mut self,
        name: impl Into<String>,
        bindings: Vec<(Underlying<ir::Param>, u64)>,
        f: F,
    ) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        self.scopes.push(ParamScope {
            name: name.into(),
            bindings,
        });

        if self.debug_mode {
            eprintln!(
                "Entering scope '{}' with {} bindings",
                self.scopes.last().unwrap().name,
                self.scopes.last().unwrap().bindings.len()
            );
        }

        let result = f(self);

        if self.debug_mode {
            let scope = self.scopes.last().unwrap();
            eprintln!("Exiting scope '{}'", scope.name);
        }

        self.scopes.pop();
        result
    }

    /// Add a binding to the current scope
    pub fn push(&mut self, param: Underlying<ir::Param>, value: u64) {
        if self.debug_mode {
            eprintln!(
                "Binding param = {} in scope '{}'",
                value,
                self.scopes.last().unwrap().name
            );
        }
        self.scopes
            .last_mut()
            .unwrap()
            .bindings
            .push((param, value));
    }

    /// Extend the current scope with multiple bindings
    pub fn extend(
        &mut self,
        bindings: impl IntoIterator<Item = (Underlying<ir::Param>, u64)>,
    ) {
        for (param, value) in bindings {
            self.push(param, value);
        }
    }

    /// Remove the last n bindings from the current scope
    pub fn pop_n(&mut self, n: usize) {
        let current_scope = self.scopes.last_mut().unwrap();
        let new_len = current_scope.bindings.len().saturating_sub(n);
        current_scope.bindings.truncate(new_len);

        if self.debug_mode {
            eprintln!(
                "Popped {} bindings from scope '{}', {} remain",
                n,
                current_scope.name,
                current_scope.bindings.len()
            );
        }
    }

    /// Get the current number of bindings across all scopes
    pub fn len(&self) -> usize {
        self.scopes.iter().map(|s| s.bindings.len()).sum()
    }

    /// Try to resolve a parameter, returning None if not found
    pub fn get(&self, param: &Underlying<ir::Param>) -> Option<u64> {
        // Search from innermost to outermost scope
        for scope in self.scopes.iter().rev() {
            // Search from newest to oldest binding in each scope
            for (p, v) in scope.bindings.iter().rev() {
                if p == param {
                    if self.debug_mode {
                        eprintln!(
                            "Resolved param = {} from scope '{}'",
                            v, scope.name
                        );
                    }
                    return Some(*v);
                }
            }
        }

        if self.debug_mode {
            eprintln!("Failed to resolve param");
            self.debug_print_state();
        }
        None
    }

    /// Resolve a parameter or panic with detailed context
    pub fn get_or_panic(
        &self,
        param: &Underlying<ir::Param>,
        context: &str,
    ) -> u64 {
        self.get(param).unwrap_or_else(|| {
            let mut msg =
                format!("Parameter not found in context: {}", context);
            msg.push_str(&format!("\nScope stack: {:?}", self.scope_names()));

            if self.debug_mode {
                msg.push_str("\nFull resolver state:");
                for (i, scope) in self.scopes.iter().enumerate() {
                    msg.push_str(&format!(
                        "\n  Scope {}: '{}' with {} bindings",
                        i,
                        scope.name,
                        scope.bindings.len()
                    ));
                }
            }

            panic!("{}", msg);
        })
    }

    /// Get the names of all current scopes for debugging
    fn scope_names(&self) -> Vec<&str> {
        self.scopes.iter().map(|s| s.name.as_str()).collect()
    }

    /// Print debug information about the current resolver state
    fn debug_print_state(&self) {
        eprintln!("ParamResolver state:");
        for (i, scope) in self.scopes.iter().enumerate() {
            eprintln!(
                "  Scope {}: '{}' with {} bindings",
                i,
                scope.name,
                scope.bindings.len()
            );
            eprintln!("    {} bindings", scope.bindings.len());
        }
    }

    /// Get an iterator over all current bindings (for compatibility)
    pub fn iter(&self) -> impl Iterator<Item = (&Underlying<ir::Param>, &u64)> {
        self.scopes
            .iter()
            .flat_map(|scope| scope.bindings.iter())
            .map(|(p, v)| (p, v))
    }

    /// Get the inner bindings as a Vec (for compatibility with existing code)
    pub fn inner(&self) -> Vec<(Underlying<ir::Param>, u64)> {
        self.scopes
            .iter()
            .flat_map(|scope| scope.bindings.iter())
            .cloned()
            .collect()
    }
}
