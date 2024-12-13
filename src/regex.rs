use crate::Config;
pub use regex::Regex;
use std::error::Error;
use std::collections::HashMap;

/// A compiled regex pattern and its corresponding replacement string
pub type Pattern = (Regex, String);

/// Holds compiled regex patterns for different window properties
#[derive(Debug)]
pub struct Compiled {
    pub class: Vec<Pattern>,
    pub instance: Vec<Pattern>,
    pub name: Vec<Pattern>,
}

/// Compiles a single regex pattern from a key-value pair
fn compile_pattern((pattern, replacement): (&String, &String)) -> Result<Pattern, Box<dyn Error>> {
    Ok((
        Regex::new(pattern).map_err(|e| format!("Invalid regex pattern '{}': {}", pattern, e))?,
        replacement.to_owned(),
    ))
}

/// Compiles a collection of patterns from a HashMap
fn compile_patterns(patterns: &HashMap<String, String>) -> Result<Vec<Pattern>, Box<dyn Error>> {
    patterns
        .iter()
        .map(|(k, v)| compile_pattern((k, v)))
        .collect()
}

/// Parses the configuration into compiled regex patterns
pub fn parse_config(config: &Config) -> Result<Compiled, Box<dyn Error>> {
    Ok(Compiled {
        class: compile_patterns(&config.aliases.class)?,
        instance: compile_patterns(&config.aliases.instance)?,
        name: compile_patterns(&config.aliases.name)?,
    })
}
