extern crate xcb;
use xcb::xproto;

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

#[macro_use]
extern crate lazy_static;
mod icons;

pub struct Options {
    pub icons: String,
    pub names: bool,
}

#[derive(Debug, Fail)]
enum LookupError {
    #[fail(display = "Failed to get a class for window id: {}", _0)]
    WindowClass(u32),
    #[fail(display = "Failed to get name for workspace: {:#?}", _0)]
    WorkspaceName(Box<Node>),
}

fn get_icon(options: &str, class: &str) -> Option<String> {
    match icons::get_icons(options) {
        Some(icons) => match icons.get(class) {
            Some(icon) => Some(icon.to_string()),
            None => None,
        },
        None => None,
    }
}

/// Return the window class based on id.
/// Source: https://stackoverflow.com/questions/44833160/how-do-i-get-the-x-window-class-given-a-window-id-with-rust-xcb
fn get_class(conn: &xcb::Connection, id: u32, options: &Options) -> Result<String, Error> {
    let window: xproto::Window = id;
    let long_length: u32 = 8;
    let mut long_offset: u32 = 0;
    let mut buf = Vec::new();

    loop {
        let cookie = xproto::get_property(
            &conn,
            false,
            window,
            xproto::ATOM_WM_CLASS,
            xproto::ATOM_STRING,
            long_offset,
            long_length,
        );

        let reply = cookie.get_reply()?;
        buf.extend_from_slice(reply.value());

        if reply.bytes_after() == 0 {
            break;
        }

        long_offset += reply.value_len() / 4;
    }

    let result = String::from_utf8(buf)?;
    let mut results = result.split('\0');
    results.next_back();
    let mut results_with_icons = results.map(|class| {
        let icon = get_icon(&options.icons, class);
        match icon {
            Some(icon) => {
                if options.names {
                    format!(" {} {} ", icon, class)
                } else {
                    format!(" {} ", icon)
                }
            }
            None => format!(" {} ", class),
        }
    });

    Ok(results_with_icons
        .next_back()
        .ok_or_else(|| LookupError::WindowClass(id))?
        .to_string())
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

/// Return a collection of window classes
fn get_classes(workspace: &Node, x_conn: &xcb::Connection, options: &Options) -> Result<Vec<String>, Error> {
    let window_ids = {
        let mut f = get_ids(vec![workspace.floating_nodes.iter().collect()]);
        let mut n = get_ids(vec![workspace.nodes.iter().collect()]);
        n.append(&mut f);
        n
    };

    let mut window_classes = Vec::new();
    for id in window_ids {
        window_classes.push(get_class(&x_conn, id, options)?);
    }

    Ok(window_classes)
}

/// Update all workspace names in tree
pub fn update_tree(x_conn: &xcb::Connection, i3_conn: &mut I3Connection, options: &Options) -> Result<(), Error> {
    let tree = i3_conn.get_tree()?;
    for workspace in get_workspaces(tree) {
        let classes = get_classes(&workspace, &x_conn, options)?.join("|");

        let old: String = workspace
            .name
            .to_owned()
            .ok_or_else(|| LookupError::WorkspaceName(Box::new(workspace)))?;

        let mut new = old.split(' ').next().unwrap().to_owned();

        if !classes.is_empty() {
            new.push_str(&classes);
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
    options: &Options,
) -> Result<(), Error> {
    match e.change {
        WindowChange::New | WindowChange::Close | WindowChange::Move => {
            update_tree(x_conn, i3_conn, options)?;
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
    options: &Options,
) -> Result<(), Error> {
    match e.change {
        WorkspaceChange::Empty | WorkspaceChange::Focus => {
            update_tree(x_conn, i3_conn, options)?;
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
        assert!(super::update_tree(&x_conn, &mut i3_conn).is_ok());
        let tree = i3_conn.get_tree()?;
        let mut name: String = String::new();
        for output in &tree.nodes {
            for container in &output.nodes {
                for workspace in &container.nodes {
                    if let NodeType::Workspace = workspace.nodetype {
                        let ws_n = workspace.name.to_owned();
                        if ws_n == Some(String::from("1 Gpick|XTerm")) {
                            name = ws_n.unwrap();
                        }
                    }
                }
            }
        }
        assert_eq!(name, String::from("1 Gpick|XTerm"));
        Ok(())
    }

    #[test]
    fn get_class() -> Result<(), Error> {
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
        let result: Result<Vec<String>, _> = ids
            .iter()
            .map(|id| super::get_class(&x_conn, *id))
            .collect();
        assert_eq!(result?, vec!["Gpick", "XTerm"]);
        Ok(())
    }

    #[test]
    fn get_classes() -> Result<(), Error> {
        env::set_var("DISPLAY", ":99.0");
        let (x_conn, _) = super::xcb::Connection::connect(None)?;
        let mut i3_conn = super::I3Connection::connect()?;
        let tree = i3_conn.get_tree()?;
        let workspaces = super::get_workspaces(tree);
        let mut result: Vec<Vec<String>> = Vec::new();
        for workspace in workspaces {
            result.push(super::get_classes(&workspace, &x_conn)?);
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
            result.push(super::get_ids(vec![workspace.floating_nodes.iter().collect()]));
        }
        let result: usize = result.iter().filter(|v| !v.is_empty()).count();
        assert_eq!(result, 2);
        Ok(())
    }
}
