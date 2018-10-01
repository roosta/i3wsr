extern crate i3ipc;
use i3ipc::{event::Event, I3Connection, I3EventListener, Subscription};

extern crate xcb;

extern crate i3wsr;

use std::{fmt::Debug, process::exit};

/// Why? cause I'm learning. Also lets me handle these spesific errors which
/// should exit the program
fn unwrap_connection<T, E: Debug>(obj: Result<T, E>) -> T {
    match obj {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Connection error: {:?}", e);
            exit(1);
        }
    }
}

fn main() {
    let mut listener = unwrap_connection(I3EventListener::connect());
    let mut i3_conn = unwrap_connection(I3Connection::connect());
    let subs = [Subscription::Window, Subscription::Workspace];
    unwrap_connection(listener.subscribe(&subs));
    let (x_conn, _) = unwrap_connection(xcb::Connection::connect(None));

    if let Err(error) = i3wsr::update_tree(&x_conn, &mut i3_conn) {
        eprintln!("Failed initial tree update with error: {}", error);
        exit(1);
    }

    for event in listener.listen() {
        match event {
            Ok(Event::WindowEvent(e)) => {
                if let Err(error) = i3wsr::handle_window_event(&e, &x_conn, &mut i3_conn) {
                    eprintln!("handle_window_event error: {}", error);
                }
            }
            Ok(Event::WorkspaceEvent(e)) => {
                if let Err(error) = i3wsr::handle_ws_event(&e, &x_conn, &mut i3_conn) {
                    eprintln!("handle_ws_event error: {}", error);
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                exit(1);
            }
            _ => (),
        }
    }
}
