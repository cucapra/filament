use super::{Base, Underlying};
use fil_ir as ir;
use std::collections::HashMap;
use std::hash::Hash;

/// Generic mapping with debug info and better error messages
pub struct MonoMapping<K, V> {
    map: HashMap<K, V>,
    name: &'static str,
    debug_mode: bool,
}

impl<K, V> MonoMapping<K, V>
where
    K: Hash + Eq + Copy,
    V: Copy,
{
    pub fn new(name: &'static str) -> Self {
        Self {
            map: HashMap::new(),
            name,
            debug_mode: std::env::var("FIL_MONO_DEBUG").is_ok(),
        }
    }

    /// Insert a new mapping, warning if overwriting
    pub fn insert(&mut self, key: K, value: V) {
        if let Some(_old) = self.map.insert(key, value) {
            if self.debug_mode {
                eprintln!("{}: Overwrote existing mapping", self.name);
            }
        } else if self.debug_mode {
            eprintln!("{}: Added new mapping", self.name);
        }
    }

    /// Get a mapping, panicking with detailed context if not found
    pub fn get(&self, key: K) -> V {
        *self.map.get(&key).unwrap_or_else(|| {
            panic!(
                "{}: Missing key. Available keys: {}",
                self.name,
                self.map.len()
            )
        })
    }

    /// Try to get a mapping, returning None if not found
    pub fn find(&self, key: K) -> Option<V> {
        let result = self.map.get(&key).copied();
        if self.debug_mode && result.is_none() {
            eprintln!("{}: Key not found", self.name);
        }
        result
    }

    /// Check if a key exists
    pub fn contains_key(&self, key: K) -> bool {
        self.map.contains_key(&key)
    }

    /// Get the number of mappings
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Check if the mapping is empty
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Iterate over all mappings
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.map.iter()
    }

    /// Clear all mappings
    pub fn clear(&mut self) {
        if self.debug_mode && !self.map.is_empty() {
            eprintln!("{}: Clearing {} mappings", self.name, self.map.len());
        }
        self.map.clear();
    }
}

impl<K, V> Default for MonoMapping<K, V>
where
    K: Hash + Eq + Copy,
    V: Copy,
{
    fn default() -> Self {
        Self::new("default")
    }
}

/// Specialized port mapping that handles complex composite keys
pub struct PortMapping {
    map: HashMap<
        (Option<Base<ir::Invoke>>, Underlying<ir::Port>),
        Base<ir::Port>,
    >,
    debug_mode: bool,
}

impl PortMapping {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            debug_mode: std::env::var("FIL_MONO_DEBUG").is_ok(),
        }
    }

    /// Insert a port mapping
    pub fn insert(
        &mut self,
        inv: Option<Base<ir::Invoke>>,
        old_port: Underlying<ir::Port>,
        new_port: Base<ir::Port>,
    ) {
        let key = (inv, old_port);
        if let Some(_old) = self.map.insert(key, new_port) {
            if self.debug_mode {
                eprintln!("PortMapping: Overwrote existing port mapping");
            }
        } else if self.debug_mode {
            eprintln!("PortMapping: Added new port mapping");
        }
    }

    /// Get a port mapping, returning None if not found
    pub fn get(
        &self,
        inv: Option<Base<ir::Invoke>>,
        old_port: Underlying<ir::Port>,
    ) -> Option<Base<ir::Port>> {
        let key = (inv, old_port);
        let result = self.map.get(&key).copied();
        if self.debug_mode && result.is_none() {
            eprintln!("PortMapping: Port key not found");
        }
        result
    }

    /// Get a port mapping, panicking if not found
    pub fn expect(
        &self,
        inv: Option<Base<ir::Invoke>>,
        old_port: Underlying<ir::Port>,
    ) -> Base<ir::Port> {
        let key = (inv, old_port);
        *self.map.get(&key).unwrap_or_else(|| {
            panic!(
                "PortMapping: Missing port key. Available keys: {}",
                self.map.len()
            )
        })
    }

    /// Check if a port mapping exists
    pub fn contains_key(
        &self,
        inv: Option<Base<ir::Invoke>>,
        old_port: Underlying<ir::Port>,
    ) -> bool {
        self.map.contains_key(&(inv, old_port))
    }

    /// Get the number of mappings
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Check if the mapping is empty
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Clear all mappings
    pub fn clear(&mut self) {
        if self.debug_mode && !self.map.is_empty() {
            eprintln!("PortMapping: Clearing {} mappings", self.map.len());
        }
        self.map.clear();
    }
}

impl Default for PortMapping {
    fn default() -> Self {
        Self::new()
    }
}

/// Type aliases for commonly used mappings
pub type EventMapping = MonoMapping<Underlying<ir::Event>, Base<ir::Event>>;
pub type ParamMapping = MonoMapping<Underlying<ir::Param>, Base<ir::Param>>;
pub type InstanceMapping =
    MonoMapping<Underlying<ir::Instance>, Base<ir::Instance>>;
pub type InvokeMapping = MonoMapping<Underlying<ir::Invoke>, Base<ir::Invoke>>;
