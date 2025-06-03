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
use itertools::Itertools;
use swayipc::{
    Connection, Node, NodeType, WindowChange, WindowEvent, WorkspaceChange, WorkspaceEvent,
};
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
/// i3wsr_core::VERBOSE.store(true, Ordering::Relaxed);
///
/// // Check if verbose is enabled
/// if i3wsr_core::VERBOSE.load(Ordering::Relaxed) {
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
    Abort(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Config(e) => write!(f, "Configuration error: {}", e),
            AppError::Connection(e) => write!(f, "IPC connection error: {}", e),
            AppError::Regex(e) => write!(f, "Regex compilation error: {}", e),
            AppError::Event(e) => write!(f, "Event handling error: {}", e),
            AppError::IoError(e) => write!(f, "IO error: {}", e),
            AppError::Abort(e) => write!(f, "Abort signal, stopping program: {}", e),
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

fn find_alias(value: Option<&String>, patterns: &[(regex::Regex, String)]) -> Option<String> {
    value.and_then(|val| {
        patterns
            .iter()
            .find(|(re, _)| re.is_match(val))
            .map(|(_, alias)| alias.clone())
    })
}

fn format_with_icon(icon: &str, title: &str, no_names: bool, no_icon_names: bool) -> String {
    if no_icon_names || no_names {
        icon.to_string()
    } else {
        format!("{} {}", icon, title)
    }
}

/// Gets a window title by trying to find an alias for the window, eventually falling back on
/// class, or app_id, depending on platform.
pub fn get_title(
    node: &Node,
    config: &Config,
    res: &regex::Compiled,
) -> Result<String, Box<dyn Error>> {
    let display_prop = config
        .get_general("display_property")
        .unwrap_or_else(|| "class".to_string());

    let title = match &node.window_properties {
        // Xwayland / Xorg
        Some(props) => {
            // First try to find an alias using the window properties
            let alias = find_alias(props.title.as_ref(), &res.name)
                .or_else(|| find_alias(props.instance.as_ref(), &res.instance))
                .or_else(|| find_alias(props.class.as_ref(), &res.class));

            // If no alias found, use the configured display property
            let title = alias.or_else(|| {
                let prop_value = match display_prop.as_str() {
                    "name" => props.title.clone(),
                    "instance" => props.instance.clone(),
                    _ => props.class.clone(),
                };
                prop_value
            });

            title.ok_or_else(|| {
                format!(
                    "No title found: tried aliases and display_prop '{}'",
                    display_prop
                )
            })?
        }
        // Wayland
        None => {
            let alias = find_alias(node.name.as_ref(), &res.name)
                .or_else(|| find_alias(node.app_id.as_ref(), &res.app_id));

            let title = alias.or_else(|| {
                let prop_value = match display_prop.as_str() {
                    "name" => node.name.clone(),
                    _ => node.app_id.clone(),
                };
                prop_value
            });
            title.ok_or_else(|| {
                format!(
                    "No title found: tried aliases and display_prop '{}'",
                    display_prop
                )
            })?
        }
    };

    // Try to find an alias first
    let no_names = get_option(config, "no_names");
    let no_icon_names = get_option(config, "no_icon_names");

    Ok(if let Some(icon) = config.get_icon(&title) {
        format_with_icon(&icon, &title, no_names, no_icon_names)
    } else if let Some(default_icon) = config.get_general("default_icon") {
        format_with_icon(&default_icon, &title, no_names, no_icon_names)
    } else if no_names {
        String::new()
    } else {
        title
    })
}

/// Filters out special workspaces (like scratchpad) and collects regular workspaces
/// from the window manager tree structure.
pub fn get_workspaces(tree: Node) -> Vec<Node> {
    let excludes = ["__i3_scratch", "__sway_scratch"];

    // Helper function to recursively find workspaces in a node
    fn find_workspaces(node: Node, excludes: &[&str]) -> Vec<Node> {
        let mut workspaces = Vec::new();

        // If this is a workspace node that's not excluded, add it
        if matches!(node.node_type, NodeType::Workspace) {
            if let Some(name) = &node.name {
                if !excludes.contains(&name.as_str()) {
                    workspaces.push(node.clone());
                }
            }
        }

        // Recursively check child nodes
        for child in node.nodes {
            workspaces.extend(find_workspaces(child, excludes));
        }

        workspaces
    }

    // Start the recursive search from the root
    find_workspaces(tree, &excludes)
}

/// Collect a vector of workspace titles, recursively traversing all nested nodes
pub fn collect_titles(workspace: &Node, config: &Config, res: &regex::Compiled) -> Vec<String> {
    fn collect_nodes<'a>(node: &'a Node, nodes: &mut Vec<&'a Node>) {
        // Add the current node if it has window properties or app_id
        if node.window_properties.is_some() || node.app_id.is_some() {
            nodes.push(node);
        }

        // Recursively collect from regular nodes
        for child in &node.nodes {
            collect_nodes(child, nodes);
        }

        // Recursively collect from floating nodes
        for child in &node.floating_nodes {
            collect_nodes(child, nodes);
        }
    }

    let mut all_nodes = Vec::new();
    collect_nodes(workspace, &mut all_nodes);

    let mut titles = Vec::new();
    for node in all_nodes {
        let title = match get_title(node, config, res) {
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

/// Applies options on titles, like remove duplicates
fn apply_options(titles: Vec<String>, config: &Config) -> Vec<String> {
    let mut processed = titles;

    if get_option(config, "remove_duplicates") {
        processed = processed.into_iter().unique().collect();
    }

    if get_option(config, "no_names") {
        processed = processed.into_iter().filter(|s| !s.is_empty()).collect();
    }

    processed
}

fn get_split_char(config: &Config) -> char {
    config
        .get_general("split_at")
        .and_then(|s| if s.is_empty() { None } else { s.chars().next() })
        .unwrap_or(' ')
}

fn format_workspace_name(initial: &str, titles: &str, split_at: char, config: &Config) -> String {
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
/// This function is public for testing purposes and binary use only.
///
/// Update all workspace names in tree
pub fn update_tree(
    conn: &mut Connection,
    config: &Config,
    res: &regex::Compiled,
    focus: bool,
) -> Result<(), Box<dyn Error>> {
    let tree = conn.get_tree()?;
    let separator = config
        .get_general("separator")
        .unwrap_or_else(|| " | ".to_string());
    let split_at = get_split_char(config);

    for workspace in get_workspaces(tree) {
        // Get the old workspace name
        let old = workspace.name.as_ref().ok_or_else(|| {
            format!(
                "Failed to get workspace name for workspace: {:#?}",
                workspace
            )
        })?;

        // Process titles
        let titles = collect_titles(&workspace, config, res);
        let titles = apply_options(titles, config);
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
                if let Some(output) = &workspace.output {
                    println!("{} Workspace on output: {}", "[INFO]".cyan(), output);
                }
            }

            // Focus on flag, fix for moving floating windows across multiple monitors
            if focus {
                let focus_cmd = format!("workspace \"{}\"", old);
                conn.run_command(&focus_cmd)?;
            }

            // Then rename it
            conn.run_command(&command)?;
        }
    }
    Ok(())
}

/// Processes various window events (new, close, move, title changes) and updates
/// workspace names accordingly. This is a core part of the event loop in the main binary.
pub fn handle_window_event(
    e: &WindowEvent,
    conn: &mut Connection,
    config: &Config,
    res: &regex::Compiled,
) -> Result<(), AppError> {
    if VERBOSE.load(Ordering::Relaxed) {
        println!(
            "{} Change: {:?}, Container: {:?}",
            "[WINDOW EVENT]".yellow(),
            e.change,
            e.container
        );
    }
    match e.change {
        WindowChange::New
        | WindowChange::Close
        | WindowChange::Move
        | WindowChange::Title
        | WindowChange::Floating => {
            update_tree(conn, config, res, false)
                .map_err(|e| AppError::Event(format!("Tree update failed: {}", e)))?;
        }
        _ => (),
    }
    Ok(())
}

/// Processes workspace events (empty, focus changes) and updates workspace names
/// as needed. This is a core part of the event loop in the main binary.
pub fn handle_ws_event(
    e: &WorkspaceEvent,
    conn: &mut Connection,
    config: &Config,
    res: &regex::Compiled,
) -> Result<(), AppError> {
    if VERBOSE.load(Ordering::Relaxed) {
        println!(
            "{} Change: {:?}, Current: {:?}, Old: {:?}",
            "[WORKSPACE EVENT]".green(),
            e.change,
            e.current,
            e.old
        );
    }

    let focus_fix = get_option(config, "focus_fix");

    match e.change {
        WorkspaceChange::Empty | WorkspaceChange::Focus => {
            update_tree(conn, config, res, e.change == WorkspaceChange::Focus && focus_fix)
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
        assert_eq!(
            super::find_alias(value, &patterns),
            Some("firefox".to_string())
        );

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
        let icon = "🦊";
        let title = "Firefox";

        // Test normal case
        assert_eq!(
            super::format_with_icon(&icon, title, false, false),
            "🦊 Firefox"
        );

        // Test no_names = true
        assert_eq!(super::format_with_icon(&icon, title, true, false), "🦊");

        // Test no_icon_names = true
        assert_eq!(super::format_with_icon(&icon, title, false, true), "🦊");

        // Test both flags true
        assert_eq!(super::format_with_icon(&icon, title, true, true), "🦊");
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
        assert_eq!(super::format_workspace_name("1", "", ':', &config), "1");

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
