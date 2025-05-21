//! # i3wsr - i3/Sway Workspace Renamer
//!
//!
//! A dynamic workspace renamer for i3 and Sway that updates names to reflect their
//! active applications.
//!
//! ## Usage
//!
//! 1. Install using cargo:
//!    ```bash
//!    cargo install i3wsr
//!    ```
//!
//! 2. Add to your i3/Sway config:
//!    ```
//!    exec_always --no-startup-id i3wsr
//!    ```
//!
//! 3. Ensure numbered workspaces in i3/Sway config:
//!    ```
//!    bindsym $mod+1 workspace number 1
//!    assign [class="(?i)firefox"] number 1
//!    ```
//!
//! ## Configuration
//!
//! Configuration can be done via:
//! - Command line arguments
//! - TOML configuration file (default: `$XDG_CONFIG_HOME/i3wsr/config.toml`)
//!
//! ### Config File Sections:
//!
//! ```toml
//! [icons]
//! # Map window classes to icons
//! Firefox = "🌍"
//! default_icon = "💻"
//!
//! [aliases.app_id]
//! "^firefox$" = "Firefox"
//!
//! [aliases.class]
//! # Map window classes to friendly names
//! "Google-chrome" = "Chrome"
//!
//! [aliases.instance]
//! # Map window instances to friendly names
//! "web.whatsapp.com" = "WhatsApp"
//!
//! [aliases.name]
//! # Map window names using regex
//! ".*mutt$" = "Mail"
//!
//! [general]
//! separator = " | "          # Separator between window names
//! split_at = ":"             # Character to split workspace number
//! empty_label = "🌕"         # Label for empty workspaces
//! display_property = "class" # Default property to display (class/app_id/instance/name)
//!
//! [options]
//! remove_duplicates = false # Remove duplicate window names
//! no_names = false          # Show only icons
//! no_icon_names = false     # Show names only if no icon available
//! focus_fix = false         # Enable experimental focus fix, see #34 for more. Ignore if you don't know you need this.
//! ```
//!
//! ### Command Line Options:
//!
//! - `--verbose`: Enable detailed logging
//! - `--config <FILE>`: Use alternative config file
//! - `--no-icon-names`: Show only icons when available
//! - `--no-names`: Never show window names
//! - `--remove-duplicates`: Remove duplicate entries
//! - `--display-property <PROPERTY>`: Window property to use (class/app_id/instance/name)
//! - `--split-at <CHAR>`: Character to split workspace names
//!
//! ### Window Properties:
//!
//! Three window properties can be used for naming:
//! - `class`: Default, most stable (WM_CLASS)
//! - `app_id`: In place of class only for sway/wayland
//! - `instance`: More specific than class (WM_INSTANCE)
//! - `name`: Most detailed but volatile (WM_NAME)
//!
//! Properties are checked in order: name -> instance -> class/app_id
//!
//! ### Special Features:
//!
//! - Regex support in aliases
//! - Custom icons per window
//! - Default icons
//! - Empty workspace labels
//! - Duplicate removal
//! - Custom separators
//!
//! For more details, see the [README](https://github.com/roosta/i3wsr)

use clap::{Parser, ValueEnum};
use dirs::config_dir;
use i3wsr_core::config::{Config, ConfigError};
use std::io;
use std::path::Path;
use swayipc::{Connection, Event, EventType, Fallible, WorkspaceChange};
use std::env;

use i3wsr_core::AppError;

/// Window property types that can be used for workspace naming.
///
/// These properties determine which window attribute is used when displaying
/// window names in workspaces:
/// - `Class`: Uses WM_CLASS (default, most stable)
/// - `Instance`: Uses WM_INSTANCE (more specific than class)
/// - `Name`: Uses WM_NAME (most detailed but volatile)
/// - `AppId`: In place of class only for sway/wayland
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum Properties {
    Class,
    Instance,
    Name,
    AppId
}

impl Properties {
    fn as_str(&self) -> &'static str {
        match self {
            Properties::Class => "class",
            Properties::Instance => "instance",
            Properties::Name => "name",
            Properties::AppId => "app_id",
        }
    }
}

/// Command line arguments for i3wsr
///
/// Configuration can be provided either through command line arguments
/// or through a TOML configuration file. Command line arguments take
/// precedence over configuration file settings.
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Dynamic workspace renamer for i3 and Sway window managers"
)]
#[command(
    long_about = "Automatically renames workspaces based on their window contents. \
    Supports custom icons, aliases, and various display options. \
    Can be configured via command line flags or a TOML configuration file."
)]
struct Args {
    /// Enable verbose logging of events and operations
    #[arg(
        short,
        long,
        help = "Print detailed information about events and operations"
    )]
    verbose: bool,

    #[arg(
        long,
        help = "Enable experimental focus fix, see #34 for more. Ignore if you don't know you need this."
    )]
    focus_fix: bool,

    /// Deprecated: Icon set option (maintained for backwards compatibility)
    #[arg(
        long,
        value_name = "SET",
        help = "[DEPRECATED] Icon set selection - will be removed in future versions"
    )]
    icons: Option<String>,
    /// Path to TOML configuration file
    #[arg(
        short,
        long,
        help = "Path to TOML config file (default: $XDG_CONFIG_HOME/i3wsr/config.toml)",
        value_name = "FILE"
    )]
    config: Option<String>,

    /// Display only icon (if available) otherwise display name
    #[arg(
        short = 'm',
        long,
        help = "Show only icons when available, fallback to names otherwise"
    )]
    no_icon_names: bool,

    /// Do not display window names, only show icons
    #[arg(short, long, help = "Show only icons, never display window names")]
    no_names: bool,

    /// Remove duplicate entries in workspace names
    #[arg(
        short,
        long,
        help = "Remove duplicate window names from workspace labels"
    )]
    remove_duplicates: bool,

    /// Which window property to use when no alias is found
    #[arg(
        short = 'p',
        long,
        value_enum,
        help = "Window property to use for naming (class/instance/name)",
        value_name = "PROPERTY"
    )]
    display_property: Option<Properties>,

    /// Character used to split the workspace title string
    #[arg(
        short = 'a',
        long,
        help = "Character that separates workspace number from window names",
        value_name = "CHAR"
    )]
    split_at: Option<String>,
}

/// Loads configuration from a TOML file or creates default configuration
fn load_config(config_path: Option<&str>) -> Result<Config, ConfigError> {
    let xdg_config = config_dir()
        .ok_or_else(|| {
            ConfigError::IoError(io::Error::new(
                io::ErrorKind::NotFound,
                "Could not determine config directory",
            ))
        })?
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
        ("focus_fix", args.focus_fix),
    ];

    for (key, value) in options {
        if value {
            config.options.insert(key.to_string(), value);
        }
    }

    // Apply general settings
    if let Some(split_char) = &args.split_at {
        config
            .general
            .insert("split_at".to_string(), split_char.clone());
    }

    if let Some(display_property) = &args.display_property {
        config
            .general
            .insert("display_property".to_string(), display_property.as_str().to_string());
    }
}

/// Sets up the program by processing arguments and initializing configuration
/// Command line arguments take precedence over configuration file settings.
fn setup() -> Result<Config, AppError> {
    let args = Args::parse();

    // Handle deprecated --icons option
    if let Some(icon_set) = &args.icons {
        if icon_set == "awesome" {
            eprintln!("Warning: The --icons option is deprecated and will be removed in a future version.");
            eprintln!("Icons are now configured via the config file in the [icons] section.");
        } else {
            eprintln!("Warning: Invalid --icons value '{}'. Only 'awesome' is supported for backwards compatibility.", icon_set);
        }
    }

    // Set verbose mode if requested
    i3wsr_core::VERBOSE.store(args.verbose, std::sync::atomic::Ordering::Relaxed);

    let mut config = load_config(args.config.as_deref())?;
    apply_args_to_config(&mut config, &args);

    Ok(config)
}

/// Processes window manager events and updates workspace names accordingly
fn handle_event(
    event: Fallible<Event>,
    conn: &mut Connection,
    config: &Config,
    res: &i3wsr_core::regex::Compiled,
) -> Result<(), AppError> {
    match event {
        Ok(Event::Window(e)) => {
            i3wsr_core::handle_window_event(&e, conn, config, res)
                .map_err(|e| AppError::Event(format!("Window event error: {}", e)))?;
        }
        Ok(Event::Workspace(e)) => {
            if e.change == WorkspaceChange::Reload && env::var("SWAYSOCK").is_ok() {
                return Err(AppError::Abort(format!("Config reloaded")));
            }
            i3wsr_core::handle_ws_event(&e, conn, config, res)
                .map_err(|e| AppError::Event(format!("Workspace event error: {}", e)))?;
        }
        Ok(_) => {}
        Err(e) => {
            // Check if it's an UnexpectedEof error (common when i3/sway restarts)
            if let swayipc::Error::Io(io_err) = &e {
                if io_err.kind() == std::io::ErrorKind::UnexpectedEof {
                    return Err(AppError::Abort("Window manager connection lost (EOF), shutting down...".to_string()));
                }
            }
            return Err(AppError::Event(format!("IPC event error: {}", e)));
        }
    }
    Ok(())
}

/// Main event loop that monitors window manager events
/// The program will continue running and handling events until
/// interrupted or an unrecoverable error occurs.
fn run() -> Result<(), AppError> {
    let config = setup()?;
    let res = i3wsr_core::regex::parse_config(&config)?;

    let mut conn = Connection::new()?;
    let subscriptions = [EventType::Window, EventType::Workspace];

    i3wsr_core::update_tree(&mut conn, &config, &res, false)
        .map_err(|e| AppError::Event(format!("Initial tree update failed: {}", e)))?;

    let event_connection = Connection::new()?;
    let events = event_connection.subscribe(&subscriptions)?;

    println!("Started successfully. Listening for events...");

    for event in events {
        if let Err(e) = handle_event(event, &mut conn, &config, &res) {
            match &e {
                // Exit program on abort, this is because when config gets reloaded, we want the
                // old process to exit, letting sway start a new one.
                AppError::Abort(_) => {
                    return Err(e);
                }
                // Continue running despite errors
                _ => eprintln!("Error handling event: {}", e),
            }
        }
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Fatal error: {}", e);
        std::process::exit(1);
    }
}
