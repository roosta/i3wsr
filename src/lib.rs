extern crate i3ipc;
extern crate x11;

use i3ipc::event::WindowEventInfo;
use i3ipc::event::inner::WindowChange;
// use std::process::Command;
// use std::str;
use std::error::Error;
use std::ffi::CString;
use std::os::raw::*;
use x11::xlib;
use std::ptr;
// use std::mem;

// Atom netWmName;
// Atom utf8;
// Atom actType;
// int nItems;
// unsigned long nItems, bytes;
// netWmName = XInternAtom(dsp, "_NET_WM_NAME", False);
// utf8 = XInternAtom(dsp, "UTF8_STRING", False);

// XGetWindowProperty(dsp, win, netWmName, 0, 0x77777777, False, utf8, &actType, &actFormat, &nItems, &bytes, (unsigned char **) &data);
pub fn handle_window_event(e: WindowEventInfo) -> Result<(), Box<Error>> {
    match e.change {
        WindowChange::New => {
            // let percent: f64 = e.container.percent.ok_or("Failed to get container size percent")?;
            // let name: String = e.container.name.ok_or("Failed to get container name")?;
            let id: c_ulong = e.container.window.ok_or("Failed to get container id")? as u64;

            unsafe {
                let display = xlib::XOpenDisplay(ptr::null());
                if display.is_null() {
                    panic!("XOpenDisplay failed");
                }
                let net_wm_name_str = CString::new("_NET_WM_NAME").unwrap();
                let net_wm_name = xlib::XInternAtom(display, net_wm_name_str.as_ptr(), xlib::False);
                let utf_str = CString::new("UTF8_STRING").unwrap();
                let utf8 = xlib::XInternAtom(display, utf_str.as_ptr(), xlib::False);
                let mut act_type: c_ulong = 0;
                let mut act_format: c_int = 0;
                let mut n_items: c_ulong = 0;
                let mut bytes: u64 = 0;
                let mut data: u8 = 0;

                xlib::XGetWindowProperty(display,
                                         id,
                                         net_wm_name,
                                         0,
                                         0x77777777,
                                         xlib::False,
                                         utf8,
                                         act_type as *mut c_ulong,
                                         act_format as *mut c_int,
                                         n_items as *mut u64,
                                         bytes as *mut u64,
                                         data as *mut _);
                // println!();
                println!("{:?}", data);
            }

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
