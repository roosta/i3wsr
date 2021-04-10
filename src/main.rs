extern crate i3ipc;
use i3ipc::{event::Event, I3Connection, I3EventListener, Subscription};

extern crate xcb;

extern crate i3wsr;

extern crate exitfailure;
use exitfailure::ExitFailure;

#[macro_use]
extern crate clap;
use clap::{App, Arg};

use i3wsr::config;

fn main() -> Result<(), ExitFailure> {
    let matches = App::new("i3wsr - i3 workspace renamer")
        .version(crate_version!())
        .author("Daniel Berg <mail@roosta.sh>")
        .arg(
            Arg::with_name("icons")
                .long("icons")
                .short("i")
                .help("Sets icons to be used")
                .possible_values(&["awesome"])
                .takes_value(true)
        )
        .arg(
            Arg::with_name("no-icon-names")
                .long("no-icon-names")
                .short("m")
                .help("Display only icon (if available) otherwise display name"),
        )
        .arg(
            Arg::with_name("no-names")
                .long("no-names")
                .short("n")
                .help("Do not display names")
        )
        .arg(
            Arg::with_name("config")
                .long("config")
                .short("c")
                .help("Path to toml config file")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("remove-duplicates")
                .long("remove-duplicates")
                .short("r")
                .help("Remove duplicate entries in workspace")
        )
        .arg(
            Arg::with_name("wm_property")
                .long("wm_property")
                .short("p")
                .help("Which window property to use when matching alias, icons")
                .possible_values(&["class", "instance", "name"])
                .takes_value(true)
        )
        .get_matches();

    let icons = matches.value_of("icons").unwrap_or("");
    let no_icon_names = matches.is_present("no-icon-names");
    let no_names = matches.is_present("no-names");
    let remove_duplicates = matches.is_present("remove-duplicates");
    let wm_property = matches.is_present("wm_property");
    let mut config = match matches.value_of("config") {
        Some(filename) => {
            let file_config = match config::read_toml_config(filename) {
                Ok(config) => config,
                Err(e) => panic!("Could not parse config file\n {}", e),
            };
            config::Config {
                icons: file_config
                    .icons
                    .into_iter()
                    .chain(i3wsr::icons::get_icons(&icons))
                    .collect(),
                aliases: file_config.aliases,
                general: file_config.general,
                options: file_config.options
            }
        }
        None => config::Config {
            icons: i3wsr::icons::get_icons(&icons),
            aliases: config::EMPTY_MAP.clone(),
            general: config::EMPTY_MAP.clone(),
            options: config::EMPTY_OPT_MAP.clone(),
        },
    };

    if no_icon_names {
        config.options.insert("no_icon_names".to_string(), no_icon_names);
    }
    if no_names {
        config.options.insert("no_names".to_string(), no_names);
    }
    if remove_duplicates {
        config.options.insert("remove_duplicates".to_string(), remove_duplicates);
    }
    if wm_property {
        let v = matches.value_of("wm_property").unwrap_or("class");
        config.general.insert("wm_property".to_string(), v.to_string());
    }

    let res = i3wsr::regex::parse_config(&config)?;
    let mut listener = I3EventListener::connect()?;
    let subs = [Subscription::Window, Subscription::Workspace];
    listener.subscribe(&subs)?;

    let (x_conn, _) = xcb::Connection::connect(None)?;
    let mut i3_conn = I3Connection::connect()?;
    i3wsr::update_tree(&x_conn, &mut i3_conn, &config, &res)?;

    for event in listener.listen() {
        match event? {
            Event::WindowEvent(e) => {
                if let Err(error) = i3wsr::handle_window_event(&e, &x_conn, &mut i3_conn, &config, &res) {
                    eprintln!("handle_window_event error: {}", error);
                }
            }
            Event::WorkspaceEvent(e) => {
                if let Err(error) = i3wsr::handle_ws_event(&e, &x_conn, &mut i3_conn, &config, &res) {
                    eprintln!("handle_ws_event error: {}", error);
                }
            }
            _ => {}
        }
    }

    Ok(())
}
