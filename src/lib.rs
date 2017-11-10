extern crate i3ipc;
use i3ipc::event::WindowEventInfo;
use i3ipc::event::inner::WindowChange;
use std::io;
use std::error::Error;
use std::io::Write;

pub fn handle_window_event(e: WindowEventInfo) -> Result<(), &'static str> {

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    match e.change {
        WindowChange::New => {
            let percent: f64 = e.container.percent.unwrap_or(1.0);
            let name: String = e.container.name.unwrap_or("unnamed".to_owned());
            let id: i32 = match e.container.window {
                Some(id) => id,
                None => return Err("Failed to get a window id"),
            };

            // if id != 0 {
            //     stdin.read_line("xprop -id")
            // }
            // println!("{}, {}", name, percent);
            // println!("{:#?}", e);
        },
        WindowChange::Close => {
            let percent: f64 = e.container.percent.unwrap_or(1.0);
            let name: String = e.container.name.unwrap_or("unnamed".to_owned());
            println!("{}, {}", name, percent);
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
