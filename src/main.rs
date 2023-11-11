use clap::{Parser, ValueEnum};
use dirs::config_dir;
use i3ipc::{event::Event, I3Connection, I3EventListener, Subscription};
use i3wsr::config::Config;
use std::error::Error;
use std::path::Path;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Icons {
    Awesome,
}

/// i3wsr config
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to toml config file
    #[arg(short, long)]
    config: Option<String>,

    /// Sets icons to be used
    #[arg(short, long)]
    icons: Option<Icons>,

    /// Display only icon (if available) otherwise display name
    #[arg(short = 'm', long)]
    no_icon_names: bool,

    /// Do not display names
    #[arg(short, long)]
    no_names: bool,

    /// Remove duplicate entries in workspace
    #[arg(short, long)]
    remove_duplicates: bool,

    /// What character used to split the workspace title string
    #[arg(short = 'a', long)]
    split_at: Option<String>,
}

/// Setup program by handling args and populating config
/// Returns result containing config
fn setup() -> Result<Config, Box<dyn Error>> {
    let args = Args::parse();

    // icons
    // Not really that useful this opt but keeping for posterity
    let icons = match args.icons {
        Some(icons) => match icons {
            Icons::Awesome => "awesome",
        },
        None => "",
    };

    // handle config
    let xdg_config = config_dir().unwrap().join("i3wsr/config.toml");
    let config_result = match args.config.as_deref() {
        Some(filename) => {
            println!("{filename}");
            Config::new(Path::new(filename), icons)
        }
        None => {
            if (xdg_config).exists() {
                Config::new(&xdg_config, icons)
            } else {
                Ok(Config {
                    icons: i3wsr::icons::get_icons(icons),
                    ..Default::default()
                })
            }
        }
    };

    let mut config = config_result?;

    // Flags
    if args.no_icon_names {
        config
            .options
            .insert("no_icon_names".to_string(), args.no_icon_names);
    }

    if args.no_names {
        config.options.insert("no_names".to_string(), args.no_names);
    }

    if args.remove_duplicates {
        config
            .options
            .insert("remove_duplicates".to_string(), args.remove_duplicates);
    }

    if let Some(split_char) = args.split_at {
        config.general.insert("split_at".to_string(), split_char);
    }

    Ok(config)
}

/// Entry main loop: continusly listen to i3 window events and workspace events, or exit on
/// abnormal error.
fn main() -> Result<(), Box<dyn Error>> {
    let config = setup()?;
    let res = i3wsr::regex::parse_config(&config)?;
    let mut listener = I3EventListener::connect()?;
    let subs = [Subscription::Window, Subscription::Workspace];

    listener.subscribe(&subs)?;

    let mut i3_conn = I3Connection::connect()?;
    i3wsr::update_tree(&mut i3_conn, &config, &res)?;

    for event in listener.listen() {
        match event? {
            Event::WindowEvent(e) => {
                if let Err(error) = i3wsr::handle_window_event(&e, &mut i3_conn, &config, &res) {
                    eprintln!("handle_window_event error: {}", error);
                }
            }
            Event::WorkspaceEvent(e) => {
                if let Err(error) = i3wsr::handle_ws_event(&e, &mut i3_conn, &config, &res) {
                    eprintln!("handle_ws_event error: {}", error);
                }
            }
            _ => {}
        }
    }
    Ok(())
}
