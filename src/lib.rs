extern crate i3ipc;

use i3ipc::event::WindowEventInfo;
use i3ipc::event::inner::WindowChange;
// use std::process::Command;
// use std::str;
use std::error::Error;
use std::ffi::CString;
use std::os::raw::*;
use std::ptr;
// use std::mem;

pub fn handle_window_event(e: WindowEventInfo) -> Result<(), Box<Error>> {
    match e.change {
        WindowChange::New => {
            // let percent: f64 = e.container.percent.ok_or("Failed to get container size percent")?;
            // let name: String = e.container.name.ok_or("Failed to get container name")?;
            let id: c_ulong = e.container.window.ok_or("Failed to get container id")? as u64;


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
