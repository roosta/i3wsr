extern crate i3ipc;
use i3ipc::event::WindowEventInfo;
pub fn handle_window_event(e: WindowEventInfo) {
    println!("window event: {:#?}", e)
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
