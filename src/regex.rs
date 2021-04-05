use failure::Error;
use libre::Regex;
use Config;

pub type Point = (Regex, String);

fn compile((k, v): (&String, &String)) -> Result<Point, Error> {
    let re = Regex::new(&format!(r"{}", k))?;
    Ok((re, v.to_owned()))
}

pub fn parse_config(config: &Config) -> Result<Vec<Point>, Error> {
    config.aliases.iter().map(compile).collect()
}
