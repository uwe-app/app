use std::path::Path;
use std::convert::AsRef;

use serde_json::{json, Value, Map};

pub struct Manifest {
    map: Map<String, Value>
}

impl Manifest {
    pub fn new() -> Self {
        Manifest{map: Map::new()}
    }

    fn get_key<P: AsRef<Path>>(&self, file: P) -> String {
        file.as_ref().to_string_lossy().into_owned()
    }

    pub fn is_dirty<P: AsRef<Path>>(&self, file: P, dest: P) -> bool {
        let key = self.get_key(file);
        if !self.map.contains_key(&key) {
            return true 
        }
        true 
    }

    pub fn touch<P: AsRef<Path>>(&mut self, file: P, dest: P) {
        let key = self.get_key(file);
        let output = dest.as_ref().to_string_lossy().into_owned();
        let mut value = Map::new();
        value.insert("output".to_string(), json!(output));

        self.map.insert(key, json!(value));
    }
}
