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
fn main() {
    let mut listener = I3EventListener::connect().ok().expect("Failed to connect to listener");
    let mut i3_conn = I3Connection::connect().ok().expect("Failed to connect to i3");
    let subs = [Subscription::Window];
    listener.subscribe(&subs).ok().expect("Failed to subscribe to i3 window events");

    let (x_conn, _) = xcb::Connection::connect(None).expect("Failed to connect to X");
    for event in listener.listen() {
        match event {
            Ok(Event::WindowEvent(e)) => {
                if let Err(e) = i3wsr::handle_window_event(e, &x_conn, &mut i3_conn) {
                    eprintln!("handle_window_event error: {}", e);
                    process::exit(1);
                }
            },
            Err(e) => eprintln!("Error: {}", e),
            _ => unreachable!()
        }
    }
}
