extern crate i3ipc;
use i3ipc::{event::Event, I3Connection, I3EventListener, Subscription};

extern crate xcb;

extern crate i3wsr;

extern crate exitfailure;
use exitfailure::ExitFailure;

extern crate clap;
use clap::{App, Arg};
use i3wsr::Options;

fn main() -> Result<(), ExitFailure> {
    let matches = App::new("i3wsr - i3 workspace renamer")
       .version("1.0")
       .author("Daniel Berg")
        .arg(Arg::with_name("icons")
            .long("icons")
            .help("Sets icons to be used (e.g. awesome)")
            .takes_value(true))
        .arg(Arg::with_name("no-names")
            .long("no-names")
            .help("Set to no to display only icons (if available)"))
       .get_matches();

    let icons = matches.value_of("icons").unwrap_or("").to_string();
    let no_names = matches.is_present("no-names");
    let options = Options {
        icons: icons,
        names: !no_names,
    };

    let mut listener = I3EventListener::connect()?;
    let subs = [Subscription::Window, Subscription::Workspace];
    listener.subscribe(&subs)?;

    let (x_conn, _) = xcb::Connection::connect(None)?;
    let mut i3_conn = I3Connection::connect()?;
    i3wsr::update_tree(&x_conn, &mut i3_conn, &options)?;

    for event in listener.listen() {
        match event? {
            Event::WindowEvent(e) => {
                if let Err(error) = i3wsr::handle_window_event(&e, &x_conn, &mut i3_conn, &options) {
                    eprintln!("handle_window_event error: {}", error);
                }
            }
            Event::WorkspaceEvent(e) => {
                if let Err(error) = i3wsr::handle_ws_event(&e, &x_conn, &mut i3_conn, &options) {
                    eprintln!("handle_ws_event error: {}", error);
                }
            }
            _ => {}
        }
    }

    Ok(())
}
