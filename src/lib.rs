extern crate i3ipc;
extern crate xcb;

use i3ipc::event::WindowEventInfo;
use i3ipc::event::WorkspaceEventInfo;
use i3ipc::event::inner::WindowChange;
use i3ipc::I3Connection;
use std::error::Error;
use xcb::xproto;
use i3ipc::reply::Node;
use i3ipc::reply::NodeType;

/// Gets the window class using XCB
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
    let results: Vec<&str> = result.split('\0').collect();
    Ok(results[1].to_string())
}

/// Checks if window is of type normal
/// Don't want to set WS name on popup windows and such.
fn is_normal(conn: &xcb::Connection, id: u32) -> Result<bool, Box<Error>> {
    let window: xproto::Window = id;
    let ident = xcb::intern_atom(&conn, true, "_NET_WM_WINDOW_TYPE").get_reply()?.atom();
    let reply = xproto::get_property(&conn, false, window, ident, xproto::ATOM_ATOM, 0, 1024).get_reply()?;
    let actual: u32 = reply.value()[0];
    let expected: u32 = xcb::intern_atom(&conn, true, "_NET_WM_WINDOW_TYPE_NORMAL").get_reply()?.atom();
    return Ok(actual == expected);
}

fn get_workspace(tree: &Node, node_id: i64) -> Option<&Node> {
    let mut out: Option<&Node> = None;
    for output in &tree.nodes {
        for container in &output.nodes {
            for workspace in &container.nodes {
                match workspace.nodetype {
                    NodeType::Workspace => {
                        for window in &workspace.nodes {
                            if window.id == node_id {
                                out = Some(workspace);
                            }
                        }
                    },
                    _ => ()
                }
            }
        }
    }
    out
}

fn get_classes(workspace: &Node, x_conn: &xcb::Connection) -> Result<Vec<String>, Box<Error>> {
    let mut window_ids: Vec<u32> = Vec::new();
    for window in &workspace.nodes {
        window_ids.push(window.window.ok_or("asd")? as u32);
    }
    let mut window_classes: Vec<String> = Vec::new();
    for id in window_ids {
        if is_normal(&x_conn, id)? {
            window_classes.push(get_class(&x_conn, id)?);
        }
    }
    Ok(window_classes)
}

fn rename_ws(e: WindowEventInfo, x_conn: &xcb::Connection, i3_conn: &mut I3Connection) -> Result<(), Box<Error>> {
    let node_id = e.container.id;
    let tree = i3_conn.get_tree()?;
    if let Some(workspace) = get_workspace(&tree, node_id) {
        let classes = get_classes(&workspace, &x_conn)?.join("|");
        let old: String = workspace.name.to_owned().ok_or("Failed to get workspace name")?;
        let old_split: Vec<&str> = old.split(' ').collect();
        let new = format!("{} {}", old_split[0], classes);
        let command = format!("rename workspace \"{}\" to \"{}\"", old, new);
        i3_conn.run_command(&command)?;
    }
    Ok(())
}

/// handles new and close window events, to set the workspace name based on content
pub fn handle_window_event(e: WindowEventInfo, x_conn: &xcb::Connection, i3_conn: &mut I3Connection) -> Result<(), Box<Error>> {
    match e.change {
        WindowChange::New => {
            rename_ws(e, x_conn, i3_conn)?;
        },
        WindowChange::Close => {
            rename_ws(e, x_conn, i3_conn)?;
            // rename_ws(e, x_conn, i3_conn)?;
            // remove_from_ws(e, x_conn, i3_conn)?;
            // rm_from_ws(e, x_conn);
        },
        _ => ()
    }
    Ok(())
}

pub fn handle_ws_event(e: WorkspaceEventInfo) {
    // println!("{:#?}", e);

}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
