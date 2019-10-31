use std::collections::HashMap as Map;
use std::fs::File;
use std::io::Read;
use std::char;
use failure::Error;

use serde::Deserialize;

#[derive(Deserialize)]
struct TomlConfig {
    icons: Map<String, String>,
}

// taken from https://github.com/greshake/i3status-rust/blob/master/src/icons.rs
macro_rules! map_to_owned (
    { $($key:expr => $value:expr),+ } => {
        {
            let mut m = ::std::collections::HashMap::new();
            $(
                m.insert($key.to_owned(), $value.to_owned());
            )+
            m
        }
     };
);

lazy_static! {
    pub static ref AWESOME: Map<String, String> = map_to_owned! {
        "Firefox" => "\u{f269}",
        "TelegramDesktop" => "\u{f2c6}",
        "Alacritty" => "\u{f120}",
        "Thunderbird" => "\u{f0e0}",
        "KeeWeb" => "\u{f023}",
        "Org.gnome.Nautilus" => "\u{f07b}",
        "Evince" => "\u{f1c1}"
    };

    pub static ref NONE: Map<String, String> = Map::new();
}

pub fn get_icons(name: &str) -> Map<String, String> {
    if name.contains(".toml") {
        return match read_toml_icons(name) {
            Ok(icons) => icons,
            Err(e) => {
                println!("Could not read icons {}", e);
                NONE.clone()
            }
        }
    }
    match name {
        "awesome" => AWESOME.clone(),
        _ => NONE.clone(),
    }
}

pub fn read_toml_icons(filename: &str) -> Result<Map<String, String>, Error> {
    let mut file = File::open(filename)?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;
    let config: TomlConfig = toml::from_str(&buffer)?;
    let icons = {
        let mut m = Map::new();
        for (key, value) in &config.icons {
            let new_value = match char::from_u32(u32::from_str_radix(value, 16)?) {
                Some(value) => value.to_string(),
                None => {
                    println!("Could not parse icon {}", value);
                    "".to_string()
                },
            };
            m.insert(key.to_string(), new_value);
        }
        m
    };
    Ok(icons)
}
