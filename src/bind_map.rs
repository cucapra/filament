//! Implements a binding map that errors provides errors for missing and conflicting bindings.
use crate::{
    core::Id,
    errors::{Error, FilamentResult},
};
use std::collections::HashMap;

pub struct BindMap<K> {
    /// Map from abstract variables to concrete variables
    map: HashMap<Id, K>,
}

impl<K> BindMap<K> {
    // Construct a new binding map
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Get the value of a binding or panic
    pub fn get(&self, id: &Id) -> &K {
        &self.map[id]
    }

    /// Get the value of a binding or produce an error
    pub fn find(&self, id: &Id) -> FilamentResult<&K> {
        if let Some(v) = self.map.get(id) {
            Ok(v)
        } else {
            Err(Error::malformed(format!("Missing binding for {id}")))
        }
    }

    // Add a binding to the map, producing an error if the binding already exists
    pub fn add(&mut self, id: Id, val: K) -> FilamentResult<()> {
        if self.map.contains_key(&id) {
            Err(Error::malformed(format!("Conflicting binding for {id}")))
        } else {
            self.map.insert(id, val);
            Ok(())
        }
    }
}
