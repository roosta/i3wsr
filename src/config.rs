use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Deserialize)]
#[serde(default)]
pub struct Aliases {
    pub class: HashMap<String, String>,
    pub instance: HashMap<String, String>,
    pub name: HashMap<String, String>,
}

#[derive(Deserialize)]
#[serde(default)]
pub struct Config {
    pub icons: HashMap<String, char>,
    pub aliases: Aliases,
    pub general: HashMap<String, String>,
    pub options: HashMap<String, bool>,
}

impl Config {
    pub fn new(filename: &Path, icons_override: &str) -> Result<Self, Box<dyn Error>> {
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

impl Default for Aliases {
    fn default() -> Self {
        Aliases {
            class: HashMap::new(),
            instance: HashMap::new(),
            name: HashMap::new(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            icons: HashMap::new(),
            aliases: Aliases {
                class: HashMap::new(),
                instance: HashMap::new(),
                name: HashMap::new(),
            },
            general: HashMap::new(),
            options: HashMap::new(),
        }
    }
}

fn read_toml_config(filename: &Path) -> Result<Config, Box<dyn Error>> {
    let mut file = File::open(filename)?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;
    let config: Config = toml::from_str(&buffer)?;
    Ok(config)
}
