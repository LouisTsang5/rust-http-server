use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

pub struct RequestMap(HashMap<String, PathBuf>);
impl RequestMap {
    pub fn new(map: HashMap<String, PathBuf>) -> Self {
        Self(map)
    }

    pub fn get(&self, k: &str) -> Option<&Path> {
        self.0.get(k).map(|p| p.as_path())
    }
}
