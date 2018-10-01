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

use std::error::Error;

/// Return the window class based on id.
/// Source: https://stackoverflow.com/questions/44833160/how-do-i-get-the-x-window-class-given-a-window-id-with-rust-xcb
fn get_class(conn: &xcb::Connection, id: u32) -> Result<String, Box<Error>> {
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
        let value: &[u8] = reply.value();
        buf.extend_from_slice(value);
        match reply.bytes_after() {
            0 => break,
            _ => {
                let len = reply.value_len();
                long_offset += len / 4;
            }
        }
    }
    let result = String::from_utf8(buf)?;
    let mut results: Vec<&str> = result.split('\0').collect();
    let error_message = format!("Failed to get a class for window id: {}", id);
    results.pop();
    Ok(results.last().ok_or(error_message)?.to_string())
}

/// Checks if window is of type normal. The problem with this is that not all
/// windows define a type (spotify, I'm looking at you) Also, even if the window
/// type is normal, the class returned will be the same regardless of type, and
/// it won't trigger a change. We do end up doing some redundant calculations by
/// not using this but makes the program much more forgiving.
fn _is_normal(conn: &xcb::Connection, id: u32) -> Result<bool, Box<Error>> {
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
fn get_workspaces(tree: &Node) -> Vec<&Node> {
    let mut out: Vec<&Node> = Vec::new();
    for output in &tree.nodes {
        for container in &output.nodes {
            for workspace in &container.nodes {
                if let NodeType::Workspace = workspace.nodetype {
                    out.push(&workspace);
                }
            }
        }
    }
    out
}

/// get window ids for any depth collection of nodes
fn get_ids(nodes: &mut Vec<Vec<&Node>>) -> Vec<u32> {
    let mut window_ids: Vec<u32> = Vec::new();
    while let Some(next) = nodes.pop() {
        for n in next {
            if !n.nodes.is_empty() {
                nodes.push(n.nodes.iter().collect());
            }
            if let Some(w) = n.window {
                window_ids.push(w as u32);
            }
        }
    }
    window_ids
}

/// Return a collection of window classes
fn get_classes(workspace: &Node, x_conn: &xcb::Connection) -> Result<Vec<String>, Box<Error>> {
    let window_ids: Vec<u32> = {
        let mut f: Vec<u32> = get_ids(&mut vec![workspace.floating_nodes.iter().collect()]);
        let mut n: Vec<u32> = get_ids(&mut vec![workspace.nodes.iter().collect()]);
        n.append(&mut f);
        n
    };
    let mut window_classes: Vec<String> = Vec::new();
    for id in window_ids {
        window_classes.push(get_class(&x_conn, id)?);
    }
    Ok(window_classes)
}

/// Update all workspace names in tree
pub fn update_tree(x_conn: &xcb::Connection, i3_conn: &mut I3Connection) -> Result<(), Box<Error>> {
    let tree = i3_conn.get_tree()?;
    let workspaces = get_workspaces(&tree);
    for workspace in &workspaces {
        let classes = get_classes(&workspace, &x_conn)?.join("|");
        let old: String = workspace.name.to_owned().ok_or_else(|| {
            format!(
                "Failed to get workspace name for workspace: {:#?}",
                workspace
            )
        })?;
        let old_split: Vec<&str> = old.split(' ').collect();
        let new = if classes.is_empty() {
            old_split[0].to_owned()
        } else {
            format!("{} {}", old_split[0], classes)
        };
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
) -> Result<(), Box<Error>> {
    match e.change {
        WindowChange::New | WindowChange::Close | WindowChange::Move => {
            update_tree(x_conn, i3_conn)?;
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
) -> Result<(), Box<Error>> {
    match e.change {
        WorkspaceChange::Empty | WorkspaceChange::Focus => {
            update_tree(x_conn, i3_conn)?;
        }
        _ => (),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use i3ipc::reply::NodeType;
    use std::env;

    #[test]
    fn connection_tree() {
        env::set_var("DISPLAY", ":99.0");
        let (x_conn, _) = super::xcb::Connection::connect(None).unwrap();
        let mut i3_conn = super::I3Connection::connect().unwrap();
        match super::update_tree(&x_conn, &mut i3_conn) {
            Ok(_) => (),
            Err(_) => assert!(false),
        };
        let tree = i3_conn.get_tree().unwrap();
        let mut name: String = String::new();
        for output in &tree.nodes {
            for container in &output.nodes {
                for workspace in &container.nodes {
                    if let NodeType::Workspace = workspace.nodetype {
                        let ws_n = workspace.name.to_owned();
                        if ws_n == Some(String::from("1 Gpick|XTerm")) {
                            name = ws_n.unwrap()
                        }
                    }
                }
            }
        }
        assert_eq!(name, String::from("1 Gpick|XTerm"));
    }

    #[test]
    fn get_class() {
        env::set_var("DISPLAY", ":99.0");
        let (x_conn, _) = super::xcb::Connection::connect(None).unwrap();
        let mut i3_conn = super::I3Connection::connect().unwrap();
        let tree = i3_conn.get_tree().unwrap();
        let mut ids: Vec<u32> = Vec::new();
        let workspaces = super::get_workspaces(&tree);
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
        let result: Vec<String> = ids
            .iter()
            .map(|id| super::get_class(&x_conn, *id).unwrap())
            .collect();
        assert_eq!(result, vec!["Gpick", "XTerm"]);
    }

    #[test]
    fn get_classes() {
        env::set_var("DISPLAY", ":99.0");
        let (x_conn, _) = super::xcb::Connection::connect(None).unwrap();
        let mut i3_conn = super::I3Connection::connect().unwrap();
        let tree = i3_conn.get_tree().unwrap();
        let workspaces = super::get_workspaces(&tree);
        let mut result: Vec<Vec<String>> = Vec::new();
        for workspace in workspaces {
            result.push(super::get_classes(&workspace, &x_conn).unwrap());
        }
        let expected = vec![vec![], vec!["Gpick", "XTerm"]];
        assert_eq!(result, expected);
    }

    #[test]
    fn get_ids() {
        env::set_var("DISPLAY", ":99.0");
        let mut i3_conn = super::I3Connection::connect().unwrap();
        let tree = i3_conn.get_tree().unwrap();
        let workspaces = super::get_workspaces(&tree);
        let mut result: Vec<Vec<u32>> = Vec::new();
        for workspace in workspaces {
            result.push(super::get_ids(&mut vec![workspace.nodes.iter().collect()]));
            result.push(super::get_ids(&mut vec![
                workspace.floating_nodes.iter().collect(),
            ]));
        }
        let result: usize = result.iter().filter(|v| !v.is_empty()).count();
        assert_eq!(result, 2);
    }
}
