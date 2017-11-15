extern crate i3ipc;
use i3ipc::event::WindowEventInfo;
use i3ipc::event::inner::WindowChange;
use std::process::Command;
use std::str;
use std::error::Error;

pub fn handle_window_event(e: WindowEventInfo) -> Result<(), Box<Error>> {

    match e.change {
        WindowChange::New => {
            let percent: f64 = e.container.percent.ok_or("i3: Failed to get container size percent")?;
            let name: String = e.container.name.ok_or("i3: failed to get container name")?;
            let id: i32 = e.container.window.ok_or("i3: Failed to get container id")?;
            let output = Command::new("xprop")
                .arg("-id")
                .arg(id.to_string())
                .arg("_NET_WM_WINDOW_TYPE")
                .arg("WM_CLASS")
                .output()?;

            if let Ok(stdout) = str::from_utf8(&output.stdout) {
                if stdout.contains("_NET_WM_WINDOW_TYPE_NORMAL") {
                    let wm_class_col: Vec<&str> = stdout
                        .split('\n')
                        .take(2)
                        .collect();

                    if let Some(wm_class) = wm_class_col.last() {
                        let wm_class: Vec<&str> = wm_class.split('"').collect();
                        let wm_class: &str = wm_class.get(1).unwrap_or(&"unknown");

                        println!("{:#?}", wm_class);
                        //     .split('"')
                        //     .collect();
                    }

                }
            }
            // let stdout: Vec<&str> = str::from_utf8(&output.stdout)?
            // .split('\n')
            //     .take(2)
            //     .collect();
            // println!("{:#?}", stdout);
            // if String::from_utf8_lossy(&output.stdout).contains("_NET_WM_WINDOW_TYPE_NORMAL") {
            // }
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
