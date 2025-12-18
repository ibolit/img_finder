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

pub fn read_from_yaml<T>(path: &str) -> Result<T, YamlReadError>
where
    T: DeserializeOwned,
{
    let contents = fs::read_to_string(path)?;
    let ht: T = serde_yaml::from_str(&contents)?;
    Ok(ht)
}

pub fn write_to_yaml(what: &impl serde::Serialize, to: &str) {
    let yaml = serde_yaml::to_string(what).expect("Failed to serialize the data");
    let mut file = File::create(to).expect("Failed to open the file for writing the data");
    file.write_all(yaml.as_bytes())
        .expect("Failed to write data");
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_read_and_write() {
        let mut my_table: HashMap<String, Vec<i32>> = HashMap::new();
        my_table.insert("hello".to_string(), vec![3, 2, 1]);

        let thing = tempfile::Builder::new()
            .prefix("example")
            .tempdir()
            .unwrap();

        let file_path = &vec![thing.path().to_str().unwrap(), "/thing.yaml"].join("");
        write_to_yaml(&my_table, file_path);
        let from_file: HashMap<String, Vec<i32>> = read_from_yaml(file_path).unwrap();

        assert!(
            my_table == from_file,
            "What we read from file is not the same as what we created initially"
        );
    }
}
