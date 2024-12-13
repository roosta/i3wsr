use i3ipc::{
    event::{
        inner::{WindowChange, WorkspaceChange},
        WindowEventInfo, WorkspaceEventInfo,
    },
    reply::{Node, NodeType, WindowProperty},
    I3Connection,
};
use itertools::Itertools;
use std::collections::HashMap;

pub mod config;
pub mod icons;
pub mod regex;

use config::Config;
use std::error::Error;

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
    props: &HashMap<WindowProperty, String>,
    config: &Config,
    res: &regex::Compiled,
) -> Result<String, Box<dyn Error>> {
    let display_prop = config.get_general("display_property").unwrap_or_else(|| "class".to_string());

    // Try to find an alias first
    let title = find_alias(props.get(&WindowProperty::Title), &res.name)
        .or_else(|| find_alias(props.get(&WindowProperty::Instance), &res.instance))
        .or_else(|| find_alias(props.get(&WindowProperty::Class), &res.class))
        // If no alias found, fall back to the configured display property
        .or_else(|| match display_prop.as_str() {
            "name" => props.get(&WindowProperty::Title).map(|s| s.to_string()),
            "instance" => props.get(&WindowProperty::Instance).map(|s| s.to_string()),
            _ => props.get(&WindowProperty::Class).map(|s| s.to_string()),
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
                if let NodeType::Workspace = workspace.nodetype {
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
fn get_properties(mut nodes: Vec<Vec<&Node>>) -> Vec<HashMap<WindowProperty, String>> {
    let mut window_props = Vec::new();

    while let Some(next) = nodes.pop() {
        for n in next {
            nodes.push(n.nodes.iter().collect());
            if let Some(w) = &n.window_properties {
                window_props.push(w.to_owned());
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

/// Update all workspace names in tree
pub fn update_tree(
    i3_conn: &mut I3Connection,
    config: &Config,
    res: &regex::Compiled,
) -> Result<(), Box<dyn Error>> {
    let tree = i3_conn.get_tree()?;
    for workspace in get_workspaces(tree) {
        let separator = config.get_general("separator").unwrap_or_else(|| " | ".to_string());

        let titles = collect_titles(&workspace, config, res);
        let titles = if get_option(&config, "remove_duplicates") {
            titles.into_iter().unique().collect()
        } else {
            titles
        };
        let titles = if get_option(&config, "no_names") {
            titles
                .into_iter()
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>()
        } else {
            titles
        };
        let titles = titles.join(&separator);
        let titles = if !titles.is_empty() {
            format!(" {}", titles)
        } else {
            titles
        };
        let old: String = workspace.name.to_owned().ok_or_else(|| {
            format!(
                "Failed to get workspace name for workspace: {:#?}",
                workspace
            )
        })?;

        // Get split_at arg
        let split_at = match config.get_general("split_at") {
            Some(s) => {
                if !s.is_empty() {
                    s.chars().next().unwrap()
                } else {
                    ' '
                }
            }
            None => ' ',
        };

        // Get the initial element we want to keep
        let initial = match old.split(split_at).next() {
            Some(i) => i,
            None => "",
        };

        let mut new: String = String::from(initial);

        // if we do split on colon we need to insert a new one, cause it gets split out
        if split_at == ':' && !initial.is_empty() && !titles.is_empty() {
            new.push(':');
        }
        // Push new window titles to new string
        if !titles.is_empty() {
            new.push_str(&titles);
        }

        if titles.is_empty() {
            match config.get_general("empty_label") {
                Some(default_label) => {
                    new.push_str(" ");
                    new.push_str(&default_label);
                }
                None => (),
            }
        }

        // Dispatch to i3
        if old != new {
            let command = format!("rename workspace \"{}\" to \"{}\"", old, new);
            i3_conn.run_command(&command)?;
        }
    }
    Ok(())
}

/// handles new and close window events, to set the workspace name based on content
pub fn handle_window_event(
    e: &WindowEventInfo,
    i3_conn: &mut I3Connection,
    config: &Config,
    res: &regex::Compiled,
) -> Result<(), Box<dyn Error>> {
    match e.change {
        WindowChange::New | WindowChange::Close | WindowChange::Move | WindowChange::Title => {
            update_tree(i3_conn, config, res)?;
        }
        _ => (),
    }
    Ok(())
}

/// handles ws events,
pub fn handle_ws_event(
    e: &WorkspaceEventInfo,
    i3_conn: &mut I3Connection,
    config: &Config,
    res: &regex::Compiled,
) -> Result<(), Box<dyn Error>> {
    match e.change {
        WorkspaceChange::Empty | WorkspaceChange::Focus => {
            update_tree(i3_conn, config, res)?;
        }
        _ => (),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use i3ipc::reply::{NodeType, WindowProperty};
    use std::collections::HashMap;
    use std::env;
    use std::error::Error;

    #[test]
    fn connection_tree() -> Result<(), Box<dyn Error>> {
        env::set_var("DISPLAY", ":99.0");
        let mut i3_conn = super::I3Connection::connect()?;
        let config = super::Config::default();
        let res = super::regex::parse_config(&config)?;
        assert!(super::update_tree(&mut i3_conn, &config, &res).is_ok());
        let tree = i3_conn.get_tree()?;
        let mut name: String = String::new();
        for output in &tree.nodes {
            for container in &output.nodes {
                for workspace in &container.nodes {
                    if let NodeType::Workspace = workspace.nodetype {
                        let ws_n = workspace.name.to_owned();
                        name = ws_n.unwrap();
                    }
                }
            }
        }
        assert_eq!(name, String::from("1 Gpick | XTerm"));
        Ok(())
    }

    #[test]
    fn get_title() -> Result<(), Box<dyn Error>> {
        env::set_var("DISPLAY", ":99.0");
        let mut i3_conn = super::I3Connection::connect()?;

        let tree = i3_conn.get_tree()?;
        let mut properties: Vec<HashMap<WindowProperty, String>> = Vec::new();
        let workspaces = super::get_workspaces(tree);
        for workspace in &workspaces {
            let window_props = {
                let mut f = super::get_properties(vec![workspace.floating_nodes.iter().collect()]);
                let mut n = super::get_properties(vec![workspace.nodes.iter().collect()]);
                n.append(&mut f);
                n
            };
            for p in window_props {
                properties.push(p);
            }
        }
        let config = super::Config::default();
        let res = super::regex::parse_config(&config)?;
        let result: Result<Vec<String>, _> = properties
            .iter()
            .map(|props| super::get_title(&props, &config, &res))
            .collect();
        assert_eq!(result?, vec!["Gpick", "XTerm"]);
        Ok(())
    }

    #[test]
    fn collect_titles() -> Result<(), Box<dyn Error>> {
        env::set_var("DISPLAY", ":99.0");
        let mut i3_conn = super::I3Connection::connect()?;
        let tree = i3_conn.get_tree()?;
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
    fn get_properties() -> Result<(), Box<dyn Error>> {
        env::set_var("DISPLAY", ":99.0");
        let mut i3_conn = super::I3Connection::connect()?;
        let tree = i3_conn.get_tree()?;
        let workspaces = super::get_workspaces(tree);
        let mut result: Vec<HashMap<WindowProperty, String>> = Vec::new();
        for workspace in workspaces {
            let window_props = {
                let mut f = super::get_properties(vec![workspace.floating_nodes.iter().collect()]);
                let mut n = super::get_properties(vec![workspace.nodes.iter().collect()]);
                n.append(&mut f);
                n
            };
            for props in window_props {
                result.push(props)
            }
        }
        let result: usize = result.iter().filter(|v| !v.is_empty()).count();
        assert_eq!(result, 2);
        Ok(())
    }
}
