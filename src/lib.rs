//! # i3wsr - i3/Sway Workspace Renamer
//!
//! Internal library functionality for the i3wsr binary. This crate provides the core functionality
//! for renaming i3/Sway workspaces based on their content.
//!
//! ## Note
//!
//! This is primarily a binary crate. The public functions and types are mainly exposed for:
//! - Use by the binary executable
//! - Testing purposes
//! - Internal organization
//!
//! While you could technically use this as a library, it's not designed or maintained for that purpose.
//!
//! ## Internal Architecture
//!
//! The crate is organized into several main components:
//!
//! - Event handling (`handle_window_event`, `handle_ws_event`)
//! - Workspace management (`update_tree`, `get_workspaces`)
//! - Window title processing (`get_title`, `collect_titles`)
//! - Configuration management (`Config` module)
//! - Regular expression handling (`regex` module)
//!
//! ## Configuration
//!
//! Configuration is handled through the `Config` type, which supports:
//! - Icon mappings for applications
//! - Display options
//! - Separator customization
//! - Regular expression patterns for window matching
//!
//! ## Error Handling
//!
//! Errors are managed through the `AppError` enum, which encompasses:
//! - Configuration errors
//! - IPC connection issues
//! - Regular expression errors
//! - Event handling problems
//! - I/O errors
use swayipc::{
    Connection,
    Node,
    NodeType,
    WindowChange,
    WindowEvent,
    WindowProperties,
    WorkspaceChange,
    WorkspaceEvent,
};
use itertools::Itertools;
extern crate colored;
use colored::Colorize;

pub mod config;
pub mod regex;

pub use config::Config;
use std::error::Error;
use std::fmt;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag to control debug output verbosity.
///
/// This flag is atomic to allow safe concurrent access without requiring mutex locks.
/// It's primarily used by the binary to enable/disable detailed logging of events
/// and commands.
///
/// # Usage
///
/// ```rust
/// use std::sync::atomic::Ordering;
///
/// // Enable verbose output
/// VERBOSE.store(true, Ordering::Relaxed);
///
/// // Check if verbose is enabled
/// if VERBOSE.load(Ordering::Relaxed) {
///     println!("Verbose output enabled");
/// }
/// ```
pub static VERBOSE: AtomicBool = AtomicBool::new(false);

#[derive(Debug)]
pub enum AppError {
    Config(config::ConfigError),
    Connection(swayipc::Error),
    Regex(regex::RegexError),
    Event(String),
    IoError(io::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Config(e) => write!(f, "Configuration error: {}", e),
            AppError::Connection(e) => write!(f, "IPC connection error: {}", e),
            AppError::Regex(e) => write!(f, "Regex compilation error: {}", e),
            AppError::Event(e) => write!(f, "Event handling error: {}", e),
            AppError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl Error for AppError {}

impl From<config::ConfigError> for AppError {
    fn from(err: config::ConfigError) -> Self {
        AppError::Config(err)
    }
}

impl From<swayipc::Error> for AppError {
    fn from(err: swayipc::Error) -> Self {
        AppError::Connection(err)
    }
}

impl From<regex::RegexError> for AppError {
    fn from(err: regex::RegexError) -> Self {
        AppError::Regex(err)
    }
}

impl From<io::Error> for AppError {
    fn from(err: io::Error) -> Self {
        AppError::IoError(err)
    }
}

/// Helper fn to get options via config
fn get_option(config: &Config, key: &str) -> bool {
    config.get_option(key).unwrap_or(false)
}

fn find_alias(
    value: Option<&String>,
    patterns: &[(regex::Regex, String)],
) -> Option<String> {
    value.and_then(|val| patterns.iter().find(|(re, _)| re.is_match(val)).map(|(_, alias)| alias.clone()))
}

fn format_with_icon(icon: &char, title: &str, no_names: bool, no_icon_names: bool) -> String {
    if no_icon_names || no_names {
        icon.to_string()
    } else {
        format!("{} {}", icon, title)
    }
}

pub fn get_title(
    props: &WindowProperties,
    config: &Config,
    res: &regex::Compiled,
) -> Result<String, Box<dyn Error>> {
    let display_prop = config.get_general("display_property").unwrap_or_else(|| "class".to_string());

    // Try to find an alias first
        let title = find_alias(props.title.as_ref(), &res.name)
    .or_else(|| find_alias(props.instance.as_ref(), &res.instance))
    .or_else(|| find_alias(props.class.as_ref(), &res.class))
    // If no alias found, fall back to the configured display property
    .or_else(|| match display_prop.as_str() {
        "name" => props.title.clone(),
        "instance" => props.instance.clone(),
        _ => props.class.clone(),
        })
        .ok_or_else(|| format!("failed to get alias, display_prop {}, or class", display_prop))?;

    let no_names = get_option(config, "no_names");
    let no_icon_names = get_option(config, "no_icon_names");

    Ok(if let Some(icon) = config.get_icon(&title) {
        format_with_icon(&icon, &title, no_names, no_icon_names)
    } else if let Some(default_icon) = config.get_icon("default_icon") {
        format_with_icon(&default_icon, &title, no_names, no_icon_names)
    } else if no_names {
        String::new()
    } else {
        title
    })
}

/// Internal function to filter and collect workspace nodes from the window manager tree.
///
/// This function is public for testing purposes and binary use only.
///
/// # Implementation Note
///
/// Filters out special workspaces (like scratchpad) and collects regular workspaces
/// from the window manager tree structure.
pub fn get_workspaces(tree: Node) -> Vec<Node> {
    let excludes = ["__i3_scratch"];
    tree.nodes.into_iter()  // outputs
        .flat_map(|output| output.nodes)  // containers
        .flat_map(|container| container.nodes)  // workspaces
        .filter(|node| matches!(node.node_type, NodeType::Workspace))
        .filter(|workspace| {
            workspace.name.as_ref()
                .map(|name| !excludes.contains(&name.as_str()))
                .unwrap_or(false)
        })
        .collect()
}

/// get window ids for any depth collection of nodes
pub fn get_properties(mut nodes: Vec<Vec<&Node>>) -> Vec<WindowProperties> {
    let mut window_props = Vec::new();

    while let Some(next) = nodes.pop() {
        for n in next {
            nodes.push(n.nodes.iter().collect());
            if let Some(w) = &n.window_properties {
                window_props.push(w.clone());
            }
        }
    }

    window_props
}

/// Collect a vector of workspace titles
pub fn collect_titles(workspace: &Node, config: &Config, res: &regex::Compiled) -> Vec<String> {
    let window_props = {
        let mut f = get_properties(vec![workspace.floating_nodes.iter().collect()]);
        let mut n = get_properties(vec![workspace.nodes.iter().collect()]);
        n.append(&mut f);
        n
    };

    let mut titles = Vec::new();
    for props in window_props {
        let title = match get_title(&props, config, res) {
            Ok(title) => title,
            Err(e) => {
                eprintln!("get_title error: \"{}\" for workspace {:#?}", e, workspace);
                continue;
            }
        };
        titles.push(title);
    }

    titles
}

fn process_titles(titles: Vec<String>, config: &Config) -> Vec<String> {
    let mut processed = titles;

    if get_option(config, "remove_duplicates") {
        processed = processed.into_iter().unique().collect();
    }

    if get_option(config, "no_names") {
        processed = processed.into_iter()
            .filter(|s| !s.is_empty())
            .collect();
    }

    processed
}

fn get_split_char(config: &Config) -> char {
    config.get_general("split_at")
        .and_then(|s| if s.is_empty() { None } else { s.chars().next() })
        .unwrap_or(' ')
}

fn format_workspace_name(
    initial: &str,
    titles: &str,
    split_at: char,
    config: &Config
) -> String {
    let mut new = String::from(initial);

    // Add colon if needed
    if split_at == ':' && !initial.is_empty() && !titles.is_empty() {
        new.push(':');
    }

    // Add titles if present
    if !titles.is_empty() {
        new.push_str(titles);
    } else if let Some(empty_label) = config.get_general("empty_label") {
        new.push(' ');
        new.push_str(&empty_label);
    }

    new
}

/// Internal function to update all workspace names based on their current content.
///
/// This function is public for testing purposes and binary use only.
///
/// # Implementation Note
///
/// Core functionality that:
/// 1. Retrieves current window manager tree
/// 2. Processes each workspace's contents
/// 3. Generates new names based on configuration
/// 4. Sends rename commands when necessary
///
/// # Error Handling
///
/// Returns errors for:
/// - Failed IPC communication
/// - Invalid workspace names
/// - Command execution failures
pub fn update_tree(
    conn: &mut Connection,
    config: &Config,
    res: &regex::Compiled,
) -> Result<(), Box<dyn Error>> {
    let tree = conn.get_tree()?;
    let separator = config.get_general("separator").unwrap_or_else(|| " | ".to_string());
    let split_at = get_split_char(config);

    for workspace in get_workspaces(tree) {
        // Get the old workspace name
        let old = workspace.name.as_ref().ok_or_else(|| {
            format!("Failed to get workspace name for workspace: {:#?}", workspace)
        })?;

        // Process titles
        let titles = collect_titles(&workspace, config, res);
        let titles = process_titles(titles, config);
        let titles = if !titles.is_empty() {
            format!(" {}", titles.join(&separator))
        } else {
            String::new()
        };

        // Get initial part of workspace name
        let initial = old.split(split_at).next().unwrap_or("");

        // Format new workspace name
        let new = format_workspace_name(initial, &titles, split_at, config);

        // Only send command if name changed
        if old != &new {
            let command = format!("rename workspace \"{}\" to \"{}\"", old, new);
            if VERBOSE.load(Ordering::Relaxed) {
                println!("{} {}", "[COMMAND]".blue(), command);
            }
            conn.run_command(command)?;
        }
    }
    Ok(())
}

/// Internal event handler for window-related events from the window manager.
///
/// This function is public for use by the binary executable only.
///
/// # Implementation Note
///
/// Processes various window events (new, close, move, title changes) and updates
/// workspace names accordingly. This is a core part of the event loop in the main binary.
///
/// # Events Handled
///
/// - `WindowChange::New`: New window created
/// - `WindowChange::Close`: Window closed
/// - `WindowChange::Move`: Window moved between workspaces
/// - `WindowChange::Title`: Window title changed
pub fn handle_window_event(
    e: &WindowEvent,
    conn: &mut Connection,
    config: &Config,
    res: &regex::Compiled,
) -> Result<(), AppError> {
    if VERBOSE.load(Ordering::Relaxed) {
        println!("{} Change: {:?}, Container: {:?}", "[WINDOW EVENT]".yellow(), e.change, e.container);
    }
    match e.change {
        WindowChange::New | WindowChange::Close | WindowChange::Move | WindowChange::Title => {
            update_tree(conn, config, res)
                .map_err(|e| AppError::Event(format!("Tree update failed: {}", e)))?;
        }
        _ => (),
    }
    Ok(())
}

/// Internal event handler for workspace-related events from the window manager.
///
/// This function is public for use by the binary executable only.
///
/// # Implementation Note
///
/// Processes workspace events (empty, focus changes) and updates workspace names
/// as needed. This is a core part of the event loop in the main binary.
///
/// # Events Handled
///
/// - `WorkspaceChange::Empty`: Workspace becomes empty
/// - `WorkspaceChange::Focus`: Workspace focus changed
pub fn handle_ws_event(
    e: &WorkspaceEvent,
    conn: &mut Connection,
    config: &Config,
    res: &regex::Compiled,
) -> Result<(), AppError> {
    if VERBOSE.load(Ordering::Relaxed) {
        println!("{} Change: {:?}, Current: {:?}, Old: {:?}",
           "[WORKSPACE EVENT]".green(), e.change, e.current, e.old);
    }
    match e.change {
        WorkspaceChange::Empty | WorkspaceChange::Focus => {
            update_tree(conn, config, res)
                .map_err(|e| AppError::Event(format!("Tree update failed: {}", e)))?;
        }
        _ => (),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use regex::Regex;

    #[test]
    fn test_find_alias() {
        let patterns = vec![
            (Regex::new(r"Firefox").unwrap(), "firefox".to_string()),
            (Regex::new(r"Chrome").unwrap(), "chrome".to_string()),
        ];

        // Test matching case
        let binding = "Firefox".to_string();
        let value = Some(&binding);
        assert_eq!(super::find_alias(value, &patterns), Some("firefox".to_string()));

        // Test non-matching case
        let binding = "Safari".to_string();
        let value = Some(&binding);
        assert_eq!(super::find_alias(value, &patterns), None);

        // Test None case
        let value: Option<&String> = None;
        assert_eq!(super::find_alias(value, &patterns), None);
    }

    #[test]
    fn test_format_with_icon() {
        let icon = '🦊';
        let title = "Firefox";

        // Test normal case
        assert_eq!(
            super::format_with_icon(&icon, title, false, false),
            "🦊 Firefox"
        );

        // Test no_names = true
        assert_eq!(
            super::format_with_icon(&icon, title, true, false),
            "🦊"
        );

        // Test no_icon_names = true
        assert_eq!(
            super::format_with_icon(&icon, title, false, true),
            "🦊"
        );

        // Test both flags true
        assert_eq!(
            super::format_with_icon(&icon, title, true, true),
            "🦊"
        );
    }

    #[test]
    fn test_process_titles() {
        let mut config = super::Config::default();

        // Test with no options enabled
        let titles = vec!["Firefox".to_string(), "Firefox".to_string(), "Chrome".to_string(), "".to_string()];
        assert_eq!(
            super::process_titles(titles.clone(), &config),
            titles
        );

        // Test with remove_duplicates
        config.set_option("remove_duplicates".to_string(), true);
        let titles = vec!["Firefox".to_string(), "Firefox".to_string(), "Chrome".to_string(), "".to_string()];
        assert_eq!(
            super::process_titles(titles.clone(), &config),
            vec!["Firefox".to_string(), "Chrome".to_string(), "".to_string()]
        );

        // Test with no_names
        config.set_option("no_names".to_string(), true);
        let titles = vec!["Firefox".to_string(), "Chrome".to_string(), "".to_string()];
        assert_eq!(
            super::process_titles(titles.clone(), &config),
            vec!["Firefox".to_string(), "Chrome".to_string()]
        );
    }

    #[test]
    fn test_get_split_char() {
        let mut config = super::Config::default();

        // Test default (space)
        assert_eq!(super::get_split_char(&config), ' ');

        // Test with custom split char
        config.set_general("split_at".to_string(), ":".to_string());
        assert_eq!(super::get_split_char(&config), ':');

        // Test with empty string
        config.set_general("split_at".to_string(), "".to_string());
        assert_eq!(super::get_split_char(&config), ' ');
    }

    #[test]
    fn test_format_workspace_name() {
        let mut config = super::Config::default();

        // Test normal case with space
        assert_eq!(
            super::format_workspace_name("1", " Firefox Chrome", ' ', &config),
            "1 Firefox Chrome"
        );

        // Test with colon separator
        assert_eq!(
            super::format_workspace_name("1", " Firefox Chrome", ':', &config),
            "1: Firefox Chrome"
        );

        // Test empty titles with no empty_label
        assert_eq!(
            super::format_workspace_name("1", "", ':', &config),
            "1"
        );

        // Test empty titles with empty_label
        config.set_general("empty_label".to_string(), "Empty".to_string());
        assert_eq!(
            super::format_workspace_name("1", "", ':', &config),
            "1 Empty"
        );

        // Test empty initial
        assert_eq!(
            super::format_workspace_name("", " Firefox Chrome", ':', &config),
            " Firefox Chrome"
        );
    }

}
