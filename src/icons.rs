use std::collections::HashMap as Map;

// Source: https://github.com/greshake/i3status-rust/blob/master/src/icons.rs
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

// Source: https://github.com/gluons/Font-Awesome-Icon-Chars
lazy_static! {
    pub static ref AWESOME: Map<String, String> = map_to_owned! {
        "Firefox" => "\u{f269} ",
        "TelegramDesktop" => "\u{f2c6} ",
        "Alacritty" => "\u{f120} ",
        "Thunderbird" => "\u{f0e0} "
    };
}


pub fn get_icons(name: &str) -> Option<Map<String, String>> {
    match name {
        "awesome" => Some(AWESOME.clone()),
        _ => None,
    }
}
