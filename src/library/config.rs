use std::fs;

use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub image_formats: Vec<String>,
    pub known_formats: Vec<String>,
    pub skip_dirs: Vec<String>,
}

impl Config {
    pub fn new(path: &str) -> Self {
        let file = fs::read_to_string(path).unwrap();
        serde_yaml::from_str(&file).unwrap()
    }
}
