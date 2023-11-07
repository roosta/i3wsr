use encoding::all::ISO_8859_1;
use encoding::{DecoderTrap, Encoding};
use i3ipc::{
    event::{
        inner::{WindowChange, WorkspaceChange},
        WindowEventInfo, WorkspaceEventInfo,
    },
    reply::{Node, NodeType},
    I3Connection,
};
use itertools::Itertools;
use xcb::{x, XidNew};

pub mod config;
pub mod icons;
pub mod regex;
use config::Config;
use std::error::Error;


/// Helper fn to get options via config
fn get_option(config: &Config, key: &str) -> bool {
    return match config.options.get(key) {
        Some(v) => *v,
        None => false,
    };
}

/// Return window property based on id.
fn get_property(
    conn: &xcb::Connection,
    id: u32,
    property: x::Atom,
) -> Result<String, Box<dyn Error>> {
    let window = unsafe { XidNew::new(id) };
    let cookie = conn.send_request(&x::GetProperty {
        delete: false,
        window,
        property,
        r#type: x::ATOM_STRING,
        long_offset: 0,
        long_length: 1024,
    });

    let reply = conn.wait_for_reply(cookie)?;
    if let Ok(s) = std::str::from_utf8(reply.value()) {
        Ok(s.to_string())
    } else {
        let decoded = ISO_8859_1.decode(reply.value(), DecoderTrap::Strict);
        match decoded {
            Ok(s) => Ok(s),
            Err(_) => Ok(String::new()),
        }
    }
}

/// Gets a window title, depends on wm_property config opt
fn get_title(
    conn: &xcb::Connection,
    id: u32,
    config: &Config,
    res: &regex::Compiled
) -> Result<String, Box<dyn Error>> {

    let reply = get_property(&conn, id, x::ATOM_WM_CLASS)?;
    let result: Vec<&str> = reply.split('\0').collect();

    // Store wm_class
    // use pattern matching for vector slice to extract class depending on position
    let [wm_class, wm_instance] = match result[..] {
        [class] => [class, ""],
        [instance, class] => [class, instance],
        [instance, class, ..] => [class, instance],
        _ => Err(format!("failed to get a instance for window id {}", id))?,
    };

    // Store window name, fall back to class
    let wm_name = {
        let name = get_property(&conn, id, x::ATOM_WM_NAME)?;
        if name.is_empty() {
            wm_class.to_string()
        } else {
            name
        }
    };

    // Check for aliases using pre-compiled regex
    let title = {
        let mut filtered_classes =
            res.class.iter().filter(|(re, _)| re.is_match(&wm_class));

        let mut filtered_instances =
            res.instance.iter().filter(|(re, _)| re.is_match(&wm_instance));

        let mut filtered_names =
            res.name.iter().filter(|(re, _)| re.is_match(&wm_name));

        match filtered_names.next() {
            Some((_, alias)) => alias,
            None => match filtered_instances.next() {
                Some((_, alias)) => alias,
                None => match filtered_classes.next() {
                    Some((_, alias)) => alias,
                    None => wm_class
                }
            }
        }
    };

    let no_names = get_option(&config, "no_names");
    let no_icon_names = get_option(&config, "no_icon_names");

    // Format final result
    Ok(match config.icons.get(title) {
        Some(icon) => {
            if no_icon_names || no_names {
                format!("{}", icon)
            } else {
                format!("{} {}", icon, title)
            }
        }
        None => match config.general.get("default_icon") {
            Some(default_icon) => {
                if no_icon_names || no_names {
                    format!("{}", default_icon)
                } else {
                    format!("{} {}", default_icon, title)
                }
            }
            None => {
                if no_names {
                    String::new()
                } else {
                    format!("{}", title)
                }
            }
        },
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
fn get_ids(mut nodes: Vec<Vec<&Node>>) -> Vec<u32> {
    let mut window_ids = Vec::new();

    while let Some(next) = nodes.pop() {
        for n in next {
            nodes.push(n.nodes.iter().collect());
            if let Some(w) = n.window {
                window_ids.push(w as u32);
            }
        }
    }

    window_ids
}

/// Collect a vector of workspace titles
fn collect_titles(
    workspace: &Node,
    x_conn: &xcb::Connection,
    config: &Config,
    res: &regex::Compiled,
) -> Vec<String> {
    let window_ids = {
        let mut f = get_ids(vec![workspace.floating_nodes.iter().collect()]);
        let mut n = get_ids(vec![workspace.nodes.iter().collect()]);
        n.append(&mut f);
        n
    };

    let mut titles = Vec::new();
    for id in window_ids {
        let title = match get_title(&x_conn, id, config, res) {
            Ok(title) => title,
            Err(e) => {
                eprintln!("get_title error: {}", e);
                continue;
            }
        };
        titles.push(title);
    }

    titles
}

/// Update all workspace names in tree
pub fn update_tree(
    x_conn: &xcb::Connection,
    i3_conn: &mut I3Connection,
    config: &Config,
    res: &regex::Compiled
) -> Result<(), Box<dyn Error>> {
    let tree = i3_conn.get_tree()?;
    for workspace in get_workspaces(tree) {
        let separator = match config.general.get("separator") {
            Some(s) => s,
            None => " | ",
        };

        let titles = collect_titles(&workspace, &x_conn, config, res);
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
        let titles = titles.join(separator);
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
        let split_at = match config.general.get("split_at") {
            Some(s) => {
                if !s.is_empty() {
                    s.chars().next().unwrap()
                } else {
                    ' '
                }
            },
            None => ' ',
        };

        // Get the initial element we want to keep
        let initial = match old.split(split_at).next() {
            Some(i) => i,
            None => ""
        };

        let mut new: String = String::from(initial);

        // if we do split on colon we need to insert a new one, cause it gets split out
        if split_at == ':' && !initial.is_empty() {
            new.push(':');
        } else {

        }
        // Push new window titles to new string
        if !titles.is_empty() {
            new.push_str(&titles);
        }

        if titles.is_empty() {
            match config.general.get("empty_label") {
                Some(default_label) => {
                    new.push_str(" ");
                    new.push_str(default_label);
                }
                None => ()
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
    x_conn: &xcb::Connection,
    i3_conn: &mut I3Connection,
    config: &Config,
    res: &regex::Compiled
) -> Result<(), Box<dyn Error>> {
    match e.change {
        WindowChange::New | WindowChange::Close | WindowChange::Move | WindowChange::Title => {
            update_tree(x_conn, i3_conn, config, res)?;
        }
        _ => (),
    }
    Ok(())
}

/// handles ws events,
pub fn handle_ws_event(
    e: &WorkspaceEventInfo,
    x_conn: &xcb::Connection,
    i3_conn: &mut I3Connection,
    config: &Config,
    res: &regex::Compiled
) -> Result<(), Box<dyn Error>> {
    match e.change {
        WorkspaceChange::Empty | WorkspaceChange::Focus => {
            update_tree(x_conn, i3_conn, config, res)?;
        }
        _ => (),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use i3ipc::reply::NodeType;
    use std::env;
    use std::error::Error;
    use xcb::Connection;

    #[test]
    fn connection_tree() -> Result<(), Box<dyn Error>> {
        env::set_var("DISPLAY", ":99.0");
        let (x_conn, _) = Connection::connect(None)?;
        let mut i3_conn = super::I3Connection::connect()?;
        let config = super::Config::default();
        let res = super::regex::parse_config(&config)?;
        assert!(super::update_tree(&x_conn, &mut i3_conn, &config, &res).is_ok());
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
        let (x_conn, _) = Connection::connect(None)?;
        let mut i3_conn = super::I3Connection::connect()?;
        let tree = i3_conn.get_tree()?;
        let mut ids: Vec<u32> = Vec::new();
        let workspaces = super::get_workspaces(tree);
        for workspace in &workspaces {
            for node in &workspace.nodes {
                if let Some(w) = node.window {
                    ids.push(w as u32);
                }
            }
            for node in &workspace.floating_nodes {
                for n in &node.nodes {
                    if let Some(w) = n.window {
                        ids.push(w as u32);
                    }
                }
            }
        }
        let config = super::Config::default();
        let res = super::regex::parse_config(&config)?;
        let result: Result<Vec<String>, _> = ids
            .iter()
            .map(|id| super::get_title(&x_conn, *id, &config, &res))
            .collect();
        assert_eq!(result?, vec!["Gpick", "XTerm"]);
        Ok(())
    }

    #[test]
    fn collect_titles() -> Result<(), Box<dyn Error>> {
        env::set_var("DISPLAY", ":99.0");
        let (x_conn, _) = Connection::connect(None)?;
        let mut i3_conn = super::I3Connection::connect()?;
        let tree = i3_conn.get_tree()?;
        let workspaces = super::get_workspaces(tree);
        let mut result: Vec<Vec<String>> = Vec::new();
        let config = super::Config::default();
        let res = super::regex::parse_config(&config)?;
        for workspace in workspaces {
            result.push(super::collect_titles(&workspace, &x_conn, &config, &res));
        }
        let expected = vec![vec!["Gpick", "XTerm"]];
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn get_ids() -> Result<(), Box<dyn Error>> {
        env::set_var("DISPLAY", ":99.0");
        let mut i3_conn = super::I3Connection::connect()?;
        let tree = i3_conn.get_tree()?;
        let workspaces = super::get_workspaces(tree);
        let mut result: Vec<Vec<u32>> = Vec::new();
        for workspace in workspaces {
            result.push(super::get_ids(vec![workspace.nodes.iter().collect()]));
            result.push(super::get_ids(vec![workspace
                .floating_nodes
                .iter()
                .collect()]));
        }
        let result: usize = result.iter().filter(|v| !v.is_empty()).count();
        assert_eq!(result, 2);
        Ok(())
    }
}
