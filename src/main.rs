extern crate i3ipc;
extern crate i3wsr;
extern crate xcb;
use i3ipc::I3EventListener;
use std::process;
use i3ipc::Subscription;
use i3ipc::event::Event;
use i3ipc::I3Connection;

// 1. Setup some sort of listener for some sort of event
// 2. on event, check workspace windows, and change name if necessary
//    A. Rules for ws names:
//       A.1. If single window rename to that window title
//       A.2. if two windows add a split w1|w2, truncate if necessary
//       A.3. If more than two, Name multiple or something
// 3. loop

/// Why? cause I'm learning. Also lets me handle these spesific errors which
/// should exit the program
fn unwrap_connection<T, E: ::std::fmt::Debug>(obj: Result<T, E>) -> T {
    match obj {
        Ok(val) => val,
        Err(e) => {
            eprintln!("Connection error: {:?}", e);
            process::exit(1);
        }
    }
}

fn main() {
    let mut listener = unwrap_connection(I3EventListener::connect());
    let mut i3_conn = unwrap_connection(I3Connection::connect());
    let subs = [Subscription::Window, Subscription::Workspace];
    unwrap_connection(listener.subscribe(&subs));
    let (x_conn, _) = unwrap_connection(xcb::Connection::connect(None));

    for event in listener.listen() {
        match event {
            Ok(Event::WindowEvent(e)) => {
                if let Err(error) = i3wsr::handle_window_event(e, &x_conn, &mut i3_conn) {
                    eprintln!("handle_window_event error: {}", error);
                    // process::exit(1);
                }
            },
            Ok(Event::WorkspaceEvent(e)) => {
                if let Err(error) = i3wsr::handle_ws_event(e, &x_conn, &mut i3_conn) {
                    eprintln!("handle_ws_event error: {}", error);
                    // process::exit(1);
                }
            },
            Err(e) => {
                eprintln!("Error: {}", e);
                process::exit(1);
            },
            _ => ()
        }
    }
}
