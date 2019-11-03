use std::collections::HashMap as Map;
use std::char;

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
    pub static ref AWESOME: Map<String, char> = map_to_owned! {
        "Firefox" => '',
        "TelegramDesktop" => '',
        "Alacritty" => '',
        "Thunderbird" => '',
        "KeeWeb" => '',
        "Org.gnome.Nautilus" => '',
        "Evince" => ''
    };

    pub static ref NONE: Map<String, char> = Map::new();
}

pub fn get_icons(name: &str) -> Map<String, char> {
    match name {
        "awesome" => AWESOME.clone(),
        _ => NONE.clone(),
    }
}
