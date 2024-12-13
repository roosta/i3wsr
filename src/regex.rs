use crate::Config;
pub use regex::Regex;
use std::error::Error;

pub type Point = (Regex, String);
pub struct Compiled {
    pub class: Vec<Point>,
    pub instance: Vec<Point>,
    pub name: Vec<Point>,
}

fn compile((k, v): (&String, &String)) -> Result<Point, Box<dyn Error>> {
    let re = Regex::new(&format!(r"{}", k))?;
    Ok((re, v.to_owned()))
}

pub fn parse_config(config: &Config) -> Result<Compiled, Box<dyn Error>> {
    let classes = match config.aliases.class.iter().map(compile).collect() {
        Ok(v) => v,
        Err(e) => Err(e)?,
    };
    let instances = match config.aliases.instance.iter().map(compile).collect() {
        Ok(v) => v,
        Err(e) => Err(e)?,
    };
    let names = match config.aliases.name.iter().map(compile).collect() {
        Ok(v) => v,
        Err(e) => Err(e)?,
    };
    return Ok(Compiled {
        class: classes,
        instance: instances,
        name: names,
    });
}
