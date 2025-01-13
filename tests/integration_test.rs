use std::env;
use std::error::Error;
use swayipc::{Connection, Node};
use i3wsr_core::{Config, update_tree};

#[test]
fn connection_tree() -> Result<(), Box<dyn Error>> {
    env::set_var("DISPLAY", ":99.0");
    let mut conn = Connection::new()?;
    let config = Config::default();
    let res = i3wsr_core::regex::parse_config(&config)?;
    assert!(update_tree(&mut conn, &config, &res, false).is_ok());

    let tree = conn.get_tree()?;
    let workspaces = i3wsr_core::get_workspaces(tree);

    let name = workspaces.first()
        .and_then(|ws| ws.name.as_ref())
        .map(|name| name.to_string())
        .unwrap_or_default();

    assert_eq!(name, String::from("1 Gpick | XTerm"));
    Ok(())
}

#[test]
fn get_title() -> Result<(), Box<dyn Error>> {
    env::set_var("DISPLAY", ":99.0");
    let mut conn = swayipc::Connection::new()?;

    let tree = conn.get_tree()?;
    let mut ws_nodes: Vec<Node> = Vec::new();
    let workspaces = i3wsr_core::get_workspaces(tree);
    for workspace in &workspaces {
        let nodes = workspace.nodes.iter()
            .chain(
                workspace.floating_nodes.iter().flat_map(|fnode| {
                    if !fnode.nodes.is_empty() {
                        fnode.nodes.iter()
                    } else {
                        std::slice::from_ref(fnode).iter()
                    }
                })
            )
            .cloned()
            .collect::<Vec<Node>>();
        ws_nodes.extend(nodes);
    }
    let config = i3wsr_core::Config::default();
    let res = i3wsr_core::regex::parse_config(&config)?;
    let result: Result<Vec<String>, _> = ws_nodes
        .iter()
        .map(|node| i3wsr_core::get_title(node, &config, &res))
        .collect();
    assert_eq!(result?, vec!["Gpick", "XTerm"]);
    Ok(())
}

#[test]
fn collect_titles() -> Result<(), Box<dyn Error>> {
    env::set_var("DISPLAY", ":99.0");
    let mut conn = swayipc::Connection::new()?;
    let tree = conn.get_tree()?;
    let workspaces = i3wsr_core::get_workspaces(tree);
    let mut result: Vec<Vec<String>> = Vec::new();
    let config = i3wsr_core::Config::default();
    let res = i3wsr_core::regex::parse_config(&config)?;
    for workspace in workspaces {
        result.push(i3wsr_core::collect_titles(&workspace, &config, &res));
    }
    let expected = vec![vec!["Gpick", "XTerm"]];
    assert_eq!(result, expected);
    Ok(())
}
