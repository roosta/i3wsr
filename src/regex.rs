use libre::Regex;
use crate::Config;
use std::error::Error;

pub type Point = (Regex, String);

fn compile((k, v): (&String, &String)) -> Result<Point, Box<dyn Error>> {
    let re = Regex::new(&format!(r"{}", k))?;
    Ok((re, v.to_owned()))
}

pub fn parse_config(config: &Config) -> Result<Vec<Point>, Box<dyn Error>> {
    config.aliases.iter().map(compile).collect()
}
