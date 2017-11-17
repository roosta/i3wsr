extern crate i3ipc;
use i3ipc::event::WindowEventInfo;
use i3ipc::event::inner::WindowChange;
use std::process::Command;
use i3ipc::I3Connection;
use std::str;
use std::error::Error;

pub fn handle_window_event(e: WindowEventInfo, connection: &mut I3Connection) -> Result<(), Box<Error>> {

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

            // 1. get tree
            // 2. walk tree until we find a workspace
            // 3. store workspace name,
            // 4. keep walking the workspace tree trying to match container id
            // 5. once identified exit walk
            // 6. rename workspace
            if let Ok(stdout) = str::from_utf8(&output.stdout) {
                if stdout.contains("_NET_WM_WINDOW_TYPE_NORMAL") {
                    let mut wm_class_col: Vec<&str> = stdout
                        .split('\n')
                        .collect();
                    wm_class_col.pop(); // discard the \n

                    if let Some(wm_class) = wm_class_col.pop() {
                        let wm_class: Vec<&str> = wm_class.split('"').collect();
                        let wm_class: &str = wm_class[3];

                        println!("{:#?}", wm_class);
                        // println!("{:#?}", connection.get_tree()?.nodes)
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
