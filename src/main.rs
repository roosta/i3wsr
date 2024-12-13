use clap::{Parser, ValueEnum};
use dirs::config_dir;
use i3ipc::{event::Event, MessageError, I3Connection, I3EventListener, Subscription};
use i3wsr::config::{Config, ConfigError};
use std::error::Error;
use std::io;
use std::path::Path;


/// Window property types for display
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Properties {
    Class,
    Instance,
    Name,
}

impl Properties {
    fn as_str(&self) -> &'static str {
        match self {
            Properties::Class => "class",
            Properties::Instance => "instance",
            Properties::Name => "name",
        }
    }
}

/// Command line arguments
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to toml config file
    #[arg(short, long)]
    config: Option<String>,

    /// Display only icon (if available) otherwise display name
    #[arg(short = 'm', long)]
    no_icon_names: bool,

    /// Do not display names
    #[arg(short, long)]
    no_names: bool,

    /// Remove duplicate entries in workspace
    #[arg(short, long)]
    remove_duplicates: bool,

    /// Which window property to use when no alias is found
    #[arg(short = 'p', long)]
    display_property: Option<Properties>,

    /// What character used to split the workspace title string
    #[arg(short = 'a', long)]
    split_at: Option<String>,
}

/// Loads configuration from file or creates default
fn load_config(config_path: Option<&str>) -> Result<Config, ConfigError> {
    let xdg_config = config_dir()
        .ok_or_else(|| ConfigError::IoError(io::Error::new(
            io::ErrorKind::NotFound,
            "Could not determine config directory"
        )))?
        .join("i3wsr/config.toml");

    match config_path {
        Some(path) => {
            println!("Loading config from: {path}");
            Config::new(Path::new(path))
        }
        None => {
            if xdg_config.exists() {
                Config::new(&xdg_config)
            } else {
                Ok(Config {
                    ..Default::default()
                })
            }
        }
    }
}

/// Applies command line arguments to configuration
fn apply_args_to_config(config: &mut Config, args: &Args) {
    // Apply boolean options
    let options = [
        ("no_icon_names", args.no_icon_names),
        ("no_names", args.no_names),
        ("remove_duplicates", args.remove_duplicates),
    ];

    for (key, value) in options {
        if value {
            config.options.insert(key.to_string(), value);
        }
    }

    // Apply general settings
    if let Some(split_char) = &args.split_at {
        config.general.insert("split_at".to_string(), split_char.clone());
    }

    let display_property = args
        .display_property
        .as_ref()
        .map_or("class", |p| p.as_str());
    config.general.insert("display_property".to_string(), display_property.to_string());
}

/// Setup program by handling args and populating config
fn setup() -> Result<Config, Box<dyn Error>> {
    let args = Args::parse();

    let mut config = load_config(args.config.as_deref())?;
    apply_args_to_config(&mut config, &args);

    Ok(config)
}

/// Handles i3 events and updates workspace names
fn handle_event(
    event: Result<Event, MessageError>,
    i3_conn: &mut I3Connection,
    config: &Config,
    res: &i3wsr::regex::Compiled,
) {
    match event {
        Ok(Event::WindowEvent(e)) => {
            if let Err(error) = i3wsr::handle_window_event(&e, i3_conn, config, res) {
                eprintln!("Window event error: {}", error);
            }
        }
        Ok(Event::WorkspaceEvent(e)) => {
            if let Err(error) = i3wsr::handle_ws_event(&e, i3_conn, config, res) {
                eprintln!("Workspace event error: {}", error);
            }
        }
        Ok(_) => {}
        Err(e) => eprintln!("Event error: {}", e),
    }
}

/// Entry main loop: continuously listen to i3 window events and workspace events
fn main() -> Result<(), Box<dyn Error>> {
    let config = setup()?;
    let res = i3wsr::regex::parse_config(&config)?;

    let mut listener = I3EventListener::connect()?;
    listener.subscribe(&[Subscription::Window, Subscription::Workspace])?;

    let mut i3_conn = I3Connection::connect()?;
    i3wsr::update_tree(&mut i3_conn, &config, &res)?;

    for event in listener.listen() {
        handle_event(event, &mut i3_conn, &config, &res);
    }

    Ok(())
}
