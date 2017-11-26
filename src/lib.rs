extern crate i3ipc;
extern crate xcb;

use i3ipc::event::WindowEventInfo;
use i3ipc::event::inner::WindowChange;
use i3ipc::I3Connection;
use std::error::Error;
use xcb::xproto;
use i3ipc::reply::Node;
use i3ipc::reply::NodeType;

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

/// If anyone happens to read this, I'd love some feedback on this function. I
/// can't seem to find much on how to walk a collection like this in a more
/// succinct manner.
fn get_workspace(root: &Node, window_id: u32) -> Option<String>  {
    let mut out = None;
    for output in &root.nodes {
        for container in &output.nodes {
            for workspace in &container.nodes {
                match workspace.nodetype {
                    NodeType::Workspace => {
                        for window in &workspace.nodes {
                            if let Some(id) = window.window {
                                if id as u32 == window_id {
                                    out = workspace.name.to_owned();
                                }
                            }
                        }
                    },
                    _ => ()
                };
            };
        }
    }
    out
}

pub fn handle_window_event(e: WindowEventInfo, x_conn: &xcb::Connection, i3_conn: &mut I3Connection) -> Result<(), Box<Error>> {
    match e.change {
        WindowChange::New => {
            let percent: f64 = e.container.percent.ok_or("1: Failed to get container size percent")?;
            let name: String = e.container.name.ok_or("2: Failed to get container name")?;
            let id: u32 = e.container.window.ok_or("3: Failed to get container id")? as u32;
            if is_normal(&x_conn, id)? {
                let tree = i3_conn.get_tree()?;
                let _class = get_class(&x_conn, id)?;
                if let Some(workspace) = get_workspace(&tree, id) {

                }
            }
        },
        WindowChange::Close => {
            // let percent: f64 = e.container.percent.unwrap_or(1.0);
            // let name: String = e.container.name.unwrap_or("unnamed".to_owned());
            // println!("{}, {}", name, percent);
        },
        _ => ()
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
