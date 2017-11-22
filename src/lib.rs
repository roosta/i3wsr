extern crate i3ipc;
extern crate xcb;

use i3ipc::event::WindowEventInfo;
use i3ipc::event::inner::WindowChange;
use std::error::Error;
use xcb::xproto;

fn get_class(conn: &xcb::Connection, id: u32) -> String {
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
        match cookie.get_reply() {
            Ok(reply) => {
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
            Err(err) => {
                println!("{:?}", err);
                break;
            }
        }
    }
    let result = String::from_utf8(buf).unwrap();
    let results: Vec<&str> = result.split('\0').collect();
    results[1].to_string()
}

/// Checks if window with id is of type normal
fn is_normal(conn: &xcb::Connection, id: u32) -> Result<bool, Box<Error>> {
    let window: xproto::Window = id;
    let ident = xcb::intern_atom(&conn, true, "_NET_WM_WINDOW_TYPE").get_reply()?.atom();
    let reply = xproto::get_property(&conn, false, window, ident, xproto::ATOM_ATOM, 0, 1024).get_reply()?;
    let actual: u32 = reply.value()[0];
    let expected: u32 = xcb::intern_atom(&conn, true, "_NET_WM_WINDOW_TYPE_NORMAL").get_reply()?.atom();
    return Ok(actual == expected);
}

pub fn handle_window_event(e: WindowEventInfo, x_conn: &xcb::Connection) -> Result<(), Box<Error>> {
    match e.change {
        WindowChange::New => {
            let percent: f64 = e.container.percent.ok_or("Failed to get container size percent")?;
            let name: String = e.container.name.ok_or("Failed to get container name")?;
            let id: u32 = e.container.window.ok_or("Failed to get container id")? as u32;
            if is_normal(&x_conn, id)? {
                println!("{}", get_class(&x_conn, id));
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
