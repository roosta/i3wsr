use std::collections::HashMap;
use std::char;

pub fn get_icons(name: &str) -> HashMap<String, char> {
    match name {
        "awesome" => {
           return HashMap::from([
                ("Firefox".to_string(),            ''),
                ("TelegramDesktop".to_string(),    ''),
                ("Alacritty".to_string(),          ''),
                ("Thunderbird".to_string(),        ''),
                ("KeeWeb".to_string(),             ''),
                ("Org.gnome.Nautilus".to_string(), ''),
                ("Evince".to_string(),             '')
            ]);
        },
        _ => HashMap::new(),
    }
}
