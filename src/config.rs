use failure::Error;
use serde::Deserialize;
use std::collections::HashMap as Map;
use std::fs::File;
use std::io::Read;
use std::path::Path;

lazy_static! {
    pub static ref EMPTY_MAP: Map<String, String> = Map::new();
    pub static ref EMPTY_OPT_MAP: Map<String, bool> = Map::new();
}

#[serde(default)]
#[derive(Deserialize)]
pub struct Config {
    pub icons: Map<String, char>,
    pub aliases: Map<String, String>,
    pub general: Map<String, String>,
    pub options: Map<String, bool>,
}

impl Config {
    pub fn new(filename: &Path, icons_override: &str) -> Result<Self, Error> {
        let file_config = read_toml_config(filename)?;
        Ok(Config {
            icons: file_config
                .icons
                .into_iter()
                .chain(crate::icons::get_icons(icons_override))
                .collect(),
            ..file_config
        })
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            icons: super::icons::NONE.clone(),
            aliases: EMPTY_MAP.clone(),
            general: EMPTY_MAP.clone(),
            options: EMPTY_OPT_MAP.clone(),
        }
    }
}

fn read_toml_config(filename: &Path) -> Result<Config, Error> {
    let mut file = File::open(filename)?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;
    let config: Config = toml::from_str(&buffer)?;
    Ok(config)
}
