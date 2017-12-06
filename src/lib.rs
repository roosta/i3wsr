extern crate i3ipc;
extern crate xcb;

use i3ipc::event::WindowEventInfo;
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

fn get_workspace(tree: &Node, window_id: u32) -> Option<&Node> {
    let mut out: Option<&Node> = None;
    for output in &tree.nodes {
        for container in &output.nodes {
            for workspace in &container.nodes {
                match workspace.nodetype {
                    NodeType::Workspace => {
                        for window in &workspace.nodes {
                            if let Some(id) = window.window {
                                if id as u32 == window_id {
                                    out = Some(workspace);
                                }
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
        window_classes.push(get_class(&x_conn, id)?);
    }
    Ok(window_classes)
}

/// handles new and close window events, to set the workspace name based on content
pub fn handle_window_event(e: WindowEventInfo, x_conn: &xcb::Connection, i3_conn: &mut I3Connection) -> Result<(), Box<Error>> {
    match e.change {
        WindowChange::New => {
            // println!("{:#?}", e);
            let percent: f64 = e.container.percent.ok_or("1: Failed to get container size percent")?;
            let active_window_id: u32 = e.container.window.ok_or("3: Failed to get window id")? as u32;
            if is_normal(&x_conn, active_window_id)? {
                let tree = i3_conn.get_tree()?;
                // let config = i3_conn.get_config()?;
                // let asd: Vec<&str> = config.config.split("\"").collect();
                if let Some(workspace) = get_workspace(&tree, active_window_id) {
                    let classes = get_classes(&workspace, &x_conn)?.join("|");
                    let ws_name: String = workspace.name.to_owned().ok_or("Failed to get workspace name")?;
                    let asd: Vec<&str> = ws_name.split(":").collect();
                    let prefix: &str = &asd[..2].join(":");
                    // let name: &str = &asd.get(2).unwrap_or(&"");
                    // let name: &str = &asd[2..];
                    // let prefix: Vec<&str> = ws_name.split(":").take(2).collect();
                    // let name: String = ws_name.split(":").last().;
                    // let name:
                    let command = format!("rename workspace {} to {}",
                                          ws_name,
                                          format!("{}:{}", prefix, classes));
                    println!("{:?}", command);
                }
                // if let Some(workspace) = get_workspace(&tree, window_id) {
                // if percent == 0.5 {
                // let command = format!("rename workspace {} to {}",
                // workspace,
                // format!("{}:{}", workspace, class));
                // println!("{:?}", command);
                // i3_conn.run_command(&command)?;
                // }
                // get_windows(&tree, window_id);
                // let outcomes = match percent {
                //     1.0 => i3_conn.run_command(&format!("rename workspace {} to {}", workspace, class))?,
                //     _ =>
                // };
                // println!("{:#?}", outcome);

                // }
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
