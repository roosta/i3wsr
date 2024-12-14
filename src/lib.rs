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

pub mod config;
pub mod regex;

use config::Config;
use std::error::Error;
use std::fmt;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};

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

fn get_title(
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

/// return a collection of workspace nodes
fn get_workspaces(tree: Node) -> Vec<Node> {
    let mut out = Vec::new();

    for output in tree.nodes {
        for container in output.nodes {
            for workspace in container.nodes {
                if let NodeType::Workspace = workspace.node_type {
                    match &workspace.name {
                        Some(name) => {
                            if !name.eq("__i3_scratch") {
                                out.push(workspace);
                            }
                        }
                        None => (),
                    }
                }
            }
        }
    }

    out
}

/// get window ids for any depth collection of nodes
fn get_properties(mut nodes: Vec<Vec<&Node>>) -> Vec<WindowProperties> {
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
fn collect_titles(workspace: &Node, config: &Config, res: &regex::Compiled) -> Vec<String> {
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

/// Update all workspace names in tree
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
                println!("[COMMAND] {}", command);
            }
            conn.run_command(command)?;
        }
    }
    Ok(())
}

/// handles new and close window events, to set the workspace name based on content
pub fn handle_window_event(
    e: &WindowEvent,
    conn: &mut Connection,
    config: &Config,
    res: &regex::Compiled,
) -> Result<(), AppError> {
    if VERBOSE.load(Ordering::Relaxed) {
        println!("[WINDOW EVENT] Change: {:?}, Container: {:?}", e.change, e.container);
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

/// handles ws events,
pub fn handle_ws_event(
    e: &WorkspaceEvent,
    conn: &mut Connection,
    config: &Config,
    res: &regex::Compiled,
) -> Result<(), AppError> {
    if VERBOSE.load(Ordering::Relaxed) {
        println!("[WORKSPACE EVENT] Change: {:?}, Current: {:?}, Old: {:?}",
            e.change, e.current, e.old);
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
    use swayipc::{NodeType, WindowProperties};
    use std::env;
    use std::error::Error;
    use regex::Regex;

    #[test]
    fn connection_tree() -> Result<(), Box<dyn Error>> {
        env::set_var("DISPLAY", ":99.0");
        let mut conn = swayipc::Connection::new()?;
        let config = super::Config::default();
        let res = super::regex::parse_config(&config)?;
        assert!(super::update_tree(&mut conn, &config, &res).is_ok());
        let tree = conn.get_tree()?;
        let mut name: String = String::new();
        for output in &tree.nodes {
            for container in &output.nodes {
                for workspace in &container.nodes {
                    if let NodeType::Workspace = workspace.node_type {
                        if let Some(ws_name) = &workspace.name {
                            name = ws_name.clone();
                        }
                    }
                }
            }
        }
        assert_eq!(name, String::from("1 Gpick | XTerm"));
        Ok(())
    }

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

    // TODO Can't seem to instantiate WindowProperties, so I can set custom props
    // #[test]
    // fn test_get_title_with_different_display_props() -> Result<(), Box<dyn Error>> {
    //     let props = WindowProperties {
    //         class: Some("TestClass".to_string()),
    //         instance: Some("TestInstance".to_string()),
    //         title: Some("TestTitle".to_string()),
    //         window_role: None,
    //         window_type: None,
    //         transient_for: None
    //     };
    //
    //     let mut config = super::Config::default();
    //     let res = super::regex::parse_config(&config)?;
    //
    //     // Test with display_property = "class"
    //     config.set_general("display_property".to_string(), "class".to_string());
    //     assert!(super::get_title(&props, &config, &res)?.contains("TestClass"));
    //
    //     // Test with display_property = "instance"
    //     config.set_general("display_property".to_string(), "instance".to_string());
    //     assert!(super::get_title(&props, &config, &res)?.contains("TestInstance"));
    //
    //     // Test with display_property = "name"
    //     config.set_general("display_property".to_string(), "name".to_string());
    //     assert!(super::get_title(&props, &config, &res)?.contains("TestTitle"));
    //
    //     Ok(())
    // }

    #[test]
    fn get_title() -> Result<(), Box<dyn Error>> {
        env::set_var("DISPLAY", ":99.0");
        let mut conn = swayipc::Connection::new()?;

        let tree = conn.get_tree()?;
        let mut properties: Vec<WindowProperties> = Vec::new();
        let workspaces = super::get_workspaces(tree);
        for workspace in &workspaces {
            let window_props = {
                let mut f = super::get_properties(vec![workspace.floating_nodes.iter().collect()]);
                let mut n = super::get_properties(vec![workspace.nodes.iter().collect()]);
                n.append(&mut f);
                n
            };
            properties.extend(window_props);
        }
        let config = super::Config::default();
        let res = super::regex::parse_config(&config)?;
        let result: Result<Vec<String>, _> = properties
            .iter()
            .map(|props| super::get_title(props, &config, &res))
            .collect();
        assert_eq!(result?, vec!["Gpick", "XTerm"]);
        Ok(())
    }

    #[test]
    fn collect_titles() -> Result<(), Box<dyn Error>> {
        env::set_var("DISPLAY", ":99.0");
        let mut conn = swayipc::Connection::new()?;
        let tree = conn.get_tree()?;
        let workspaces = super::get_workspaces(tree);
        let mut result: Vec<Vec<String>> = Vec::new();
        let config = super::Config::default();
        let res = super::regex::parse_config(&config)?;
        for workspace in workspaces {
            result.push(super::collect_titles(&workspace, &config, &res));
        }
        let expected = vec![vec!["Gpick", "XTerm"]];
        assert_eq!(result, expected);
        Ok(())
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

    #[test]
    fn get_properties() -> Result<(), Box<dyn Error>> {
        env::set_var("DISPLAY", ":99.0");
        let mut conn = swayipc::Connection::new()?;
        let tree = conn.get_tree()?;
        let workspaces = super::get_workspaces(tree);
        let mut result: Vec<WindowProperties> = Vec::new();
        for workspace in workspaces {
            let window_props = {
                let mut f = super::get_properties(vec![workspace.floating_nodes.iter().collect()]);
                let mut n = super::get_properties(vec![workspace.nodes.iter().collect()]);
                n.append(&mut f);
                n
            };
            result.extend(window_props);
        }
        let result: usize = result.iter().filter(|v| v.class.is_some() || v.instance.is_some() || v.title.is_some()).count();
        assert_eq!(result, 2);
        Ok(())
    }
}
