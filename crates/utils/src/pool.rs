use std::{collections::HashMap, sync::RwLock};

#[derive(Default)]
pub struct Pool {
    values: boxcar::Vec<String>,
    map: RwLock<HashMap<String, u32>>,
}

impl Pool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_or_intern(&self, s: &str) -> u32 {
        if let Some(&idx) = self.map.read().unwrap().get(s) {
            idx
        } else {
            let idx = self.values.push(s.to_owned());
            self.map.write().unwrap().insert(s.to_owned(), idx as u32);
            idx as u32
        }
    }

    pub fn get(&self, idx: u32) -> &str {
        &self.values[idx as usize]
    }
}
