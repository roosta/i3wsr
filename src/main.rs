extern crate i3ipc;
use i3ipc::{event::Event, I3Connection, I3EventListener, Subscription};

extern crate xcb;

extern crate i3wsr;

extern crate exitfailure;
use exitfailure::ExitFailure;

fn main() -> Result<(), ExitFailure> {
    let mut listener = I3EventListener::connect()?;
    let subs = [Subscription::Window, Subscription::Workspace];
    listener.subscribe(&subs)?;

    let (x_conn, _) = xcb::Connection::connect(None)?;
    let mut i3_conn = I3Connection::connect()?;
    i3wsr::update_tree(&x_conn, &mut i3_conn)?;

    for event in listener.listen() {
        match event? {
            Event::WindowEvent(e) => {
                if let Err(error) = i3wsr::handle_window_event(&e, &x_conn, &mut i3_conn) {
                    eprintln!("handle_window_event error: {}", error);
                }
            }
            Event::WorkspaceEvent(e) => {
                if let Err(error) = i3wsr::handle_ws_event(&e, &x_conn, &mut i3_conn) {
                    eprintln!("handle_ws_event error: {}", error);
                }
            }
            _ => {}
        }
    }

    Ok(())
}
