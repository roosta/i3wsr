use std::collections::HashMap as Map;
use std::fs::File;
use std::io::Read;
use serde::Deserialize;
use failure::Error;

lazy_static! {
    pub static ref EMPTY_CLASSES_MAP: Map<String, String> = Map::new();
}

#[derive(Deserialize)]
pub struct TomlConfig {
    pub icons: Map<String, char>,
    pub classes: Map<String, String>,
}

pub fn read_toml_config(filename: &str) -> Result<TomlConfig, Error> {
    let mut file = File::open(filename)?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;
    let config: TomlConfig = toml::from_str(&buffer)?;
    Ok(config)
}
