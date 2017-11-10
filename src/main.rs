extern crate i3ipc;
extern crate i3wsr;
use i3ipc::I3EventListener;
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
    // establish connection.
    let mut listener = I3EventListener::connect().unwrap();
    let mut connection = I3Connection::connect().unwrap();

    let subs = [Subscription::Window];
    listener.subscribe(&subs).unwrap();

    // println!("tree: {:#?}", connection.get_tree());
    // handle them
    for event in listener.listen() {
        match event.unwrap() {
            Event::WindowEvent(e) => i3wsr::handle_window_event(e),
            _ => unreachable!()
        }
    }
}
