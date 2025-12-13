use std::fs::{self, File};
use std::io::{self, Write};

use serde::de::DeserializeOwned;

#[derive(Debug)]
pub enum YamlReadError {
    E,
}
impl From<io::Error> for YamlReadError {
    fn from(_value: io::Error) -> Self {
        Self::E
    }
}

impl From<serde_yaml::Error> for YamlReadError {
    fn from(_value: serde_yaml::Error) -> Self {
        Self::E
    }
}

pub fn read_yaml_file<T>(path: &str) -> Result<T, YamlReadError>
where
    T: DeserializeOwned,
{
    let contents = fs::read_to_string(path)?;
    let ht: T = serde_yaml::from_str(&contents)?;
    Ok(ht)
}

pub fn write_to_yaml(new_ht: &impl serde::Serialize, to: &str) {
    let yaml = serde_yaml::to_string(new_ht).expect("Da fuck");
    let mut file = File::create(to).expect("Failed to open a file for writing image info");
    file.write_all(yaml.as_bytes())
        .expect("Failed to write image info");
}
