use std::collections::HashMap as Map;
use std::fs::File;
use std::io::Read;
use serde::Deserialize;
use failure::Error;

lazy_static! {
    pub static ref EMPTY_ALIASES_MAP: Map<String, String> = Map::new();
}

#[derive(Deserialize)]
struct TomlDeserializableConfig {
    icons: Option<Map<String, char>>,
    aliases: Option<Map<String, String>>,
}

pub struct Config {
    pub icons: Map<String, char>,
    pub aliases: Map<String, String>,
}

pub fn read_toml_config(filename: &str) -> Result<Config, Error> {
    let mut file = File::open(filename)?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;
    let config: TomlDeserializableConfig = toml::from_str(&buffer)?;
    Ok(Config {
        icons: config.icons.unwrap_or(super::icons::NONE.clone()),
        aliases: config.aliases.unwrap_or(EMPTY_ALIASES_MAP.clone()),
    })
}
