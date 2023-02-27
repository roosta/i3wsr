extern crate xcb;
use xcb::xproto;
extern crate itertools;
use itertools::Itertools;

extern crate regex as libre;

extern crate i3ipc;
use i3ipc::{
    event::{
        inner::{WindowChange, WorkspaceChange},
        WindowEventInfo, WorkspaceEventInfo,
    },
    reply::{Node, NodeType},
    I3Connection,
};

#[macro_use]
extern crate failure_derive;
extern crate failure;
use failure::Error;

extern crate serde;

#[macro_use]
extern crate lazy_static;

extern crate toml;

extern crate encoding;
use encoding::{Encoding, DecoderTrap};
use encoding::all::ISO_8859_1;

pub mod config;
pub mod icons;
pub mod regex;

use config::Config;

#[derive(Debug, Fail)]
enum LookupError {
    #[fail(display = "Failed to get a class for window id: {}", _0)]
    WindowClass(u32),
    #[fail(display = "Failed to get a instance for window id: {}", _0)]
    WindowInstance(u32),
    #[fail(display = "Failed to get title for workspace: {:#?}", _0)]
    WorkspaceTitle(Box<Node>),
}

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
    prop: xproto::Atom,
) -> Result<String, Error> {
    let window: xproto::Window = id;

    let cookie = xproto::get_property(
        &conn,
        false,
        window,
        prop,
        xproto::ATOM_STRING,
        0,
        1024,
    );

    let reply = cookie.get_reply()?;
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
    res: &Vec<regex::Point>,
) -> Result<String, Error> {

    let use_prop = match config.general.get("wm_property") {
        Some(prop) => prop,
        None => "class",
    };

    let reply = get_property(&conn, id, xproto::ATOM_WM_CLASS)?;
    let mut reply = reply.split('\0');

    // Store vm_instance, leaving only class in results
    let wm_instance = reply
        .next()
        .ok_or_else(|| LookupError::WindowInstance(id))?;

    // Store wm_class
    let wm_class = reply.next().ok_or_else(|| LookupError::WindowClass(id))?;

    // Set target from options
    let target = match use_prop {
        "class" => wm_class.to_string(),
        "instance" => wm_instance.to_string(),
        "name" => {
            let name = get_property(&conn, id, xproto::ATOM_WM_NAME)?;
            if name.is_empty() {
                wm_class.to_string()
            } else {
                name
            }

        },
        _ => wm_class.to_string()
    };

    // Check for aliases using pre-compiled regex
    let title = {
        let mut filtered = res.iter().filter(|(re, _)| {
            re.is_match(&target)
        });
        match filtered.next() {
            Some((_, alias)) => alias,
            None => &target
        }
    };

    // either use icon for wm_instance, or fall back to icon for class
    let key = if config.icons.contains_key(title) {
        title
    } else {
        wm_class
    };

    let no_names = get_option(&config, "no_names");
    let no_icon_names = get_option(&config, "no_icon_names");

    // Format final result
    Ok(match config.icons.get(key) {
        Some(icon) => {
            if no_icon_names || no_names {
                format!("{}", icon)
            } else {
                format!("{} {}", icon, title)
            }
        }
        None => match config.general.get("default_icon") {
            Some(default_icon) => {
                if no_icon_names || no_names  {
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

/// Checks if window is of type normal. The problem with this is that not all
/// windows define a type (spotify, I'm looking at you) Also, even if the window
/// type is normal, the class returned will be the same regardless of type, and
/// it won't trigger a change. We do end up doing some redundant calculations by
/// not using this but makes the program much more forgiving.
fn _is_normal(conn: &xcb::Connection, id: u32) -> Result<bool, Error> {
    let window: xproto::Window = id;
    let ident = xcb::intern_atom(&conn, true, "_NET_WM_WINDOW_TYPE")
        .get_reply()?
        .atom();
    let reply = xproto::get_property(&conn, false, window, ident, xproto::ATOM_ATOM, 0, 1024)
        .get_reply()?;
    let actual: u32 = reply.value()[0];
    let expected: u32 = xcb::intern_atom(&conn, true, "_NET_WM_WINDOW_TYPE_NORMAL")
        .get_reply()?
        .atom();
    Ok(actual == expected)
}

/// return a collection of workspace nodes
fn get_workspaces(tree: Node) -> Vec<Node> {
    let mut out = Vec::new();

    for output in tree.nodes {
        for container in output.nodes {
            for workspace in container.nodes {
                if let NodeType::Workspace = workspace.nodetype {
                    out.push(workspace);
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
    res: &Vec<regex::Point>,
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
    res: &Vec<regex::Point>,
) -> Result<(), Error> {
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
            titles.into_iter().filter(|s| !s.is_empty()).collect::<Vec<String>>()
        } else {
            titles
        };
        let titles = titles.join(separator);
        let titles = if !titles.is_empty() {
            format!(" {}", titles)
        } else {
            titles
        };

        let old: String = workspace
            .name
            .to_owned()
            .ok_or_else(|| LookupError::WorkspaceTitle(Box::new(workspace)))?;

        let mut new = old.split(' ').next().unwrap().to_owned();

        if !titles.is_empty() {
            new.push_str(&titles);
        }

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
    res: &Vec<regex::Point>,
) -> Result<(), Error> {
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
    res: &Vec<regex::Point>,
) -> Result<(), Error> {
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
    use failure::Error;
    use i3ipc::reply::NodeType;
    use std::env;

    #[test]
    fn connection_tree() -> Result<(), Error> {
        env::set_var("DISPLAY", ":99.0");
        let (x_conn, _) = super::xcb::Connection::connect(None)?;
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
    fn get_title() -> Result<(), Error> {
        env::set_var("DISPLAY", ":99.0");
        let (x_conn, _) = super::xcb::Connection::connect(None)?;
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
    fn collect_titles() -> Result<(), Error> {
        env::set_var("DISPLAY", ":99.0");
        let (x_conn, _) = super::xcb::Connection::connect(None)?;
        let mut i3_conn = super::I3Connection::connect()?;
        let tree = i3_conn.get_tree()?;
        let workspaces = super::get_workspaces(tree);
        let mut result: Vec<Vec<String>> = Vec::new();
        let config = super::Config::default();
        let res = super::regex::parse_config(&config)?;
        for workspace in workspaces {
            result.push(super::collect_titles(&workspace, &x_conn, &config, &res));
        }
        let expected = vec![vec![], vec!["Gpick", "XTerm"]];
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn get_ids() -> Result<(), Error> {
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
