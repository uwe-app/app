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

    pub fn is_dirty<P: AsRef<Path>>(&self, file: P, _dest: P) -> bool {
        let key = self.get_key(file);

        println!("is_dirty {:?}", key);

        if !self.map.contains_key(&key) {
            println!("does not contain key");
            return true 
        }

        println!("AFTER DOES NOT CONTAIN KEY");

        if let Some(val) = self.map.get(&key) {
            println!("got existing value in manifest {:?} {:?}", key, val)
        }

        true 
    }

    pub fn touch<P: AsRef<Path>>(&mut self, file: P, dest: P) {
        let key = self.get_key(file);
        let output = dest.as_ref().to_string_lossy().into_owned();
        let mut value = Map::new();
        value.insert("output".to_string(), json!(output));

        let copy = key.clone();

        self.map.insert(key, json!(value));

        println!("manifest saving entry {:?} {:?}", &copy, self.map.contains_key(&copy));
    }
}
