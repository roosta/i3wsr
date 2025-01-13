use crate::Config;
pub use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum RegexError {
    Compilation(regex::Error),
    Pattern(String),
}

impl fmt::Display for RegexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegexError::Compilation(e) => write!(f, "Regex compilation error: {}", e),
            RegexError::Pattern(e) => write!(f, "{}", e),
        }
    }
}

impl Error for RegexError {}

impl From<regex::Error> for RegexError {
    fn from(err: regex::Error) -> Self {
        RegexError::Compilation(err)
    }
}

/// A compiled regex pattern and its corresponding replacement string
pub type Pattern = (Regex, String);

/// Holds compiled regex patterns for different window properties
#[derive(Debug)]
pub struct Compiled {
    pub class: Vec<Pattern>,
    pub instance: Vec<Pattern>,
    pub name: Vec<Pattern>,
    pub app_id: Vec<Pattern>,
}

/// Compiles a single regex pattern from a key-value pair
fn compile_pattern((pattern, replacement): (&String, &String)) -> Result<Pattern, RegexError> {
    Ok((
        Regex::new(pattern).map_err(|e| {
            RegexError::Pattern(format!("Invalid regex pattern '{}': {}", pattern, e))
        })?,
        replacement.to_owned(),
    ))
}

/// Compiles a collection of patterns from a HashMap
fn compile_patterns(patterns: &HashMap<String, String>) -> Result<Vec<Pattern>, RegexError> {
    patterns
        .iter()
        .map(|(k, v)| compile_pattern((k, v)))
        .collect()
}

/// Parses the configuration into compiled regex patterns
pub fn parse_config(config: &Config) -> Result<Compiled, RegexError> {
    Ok(Compiled {
        class: compile_patterns(&config.aliases.class)?,
        instance: compile_patterns(&config.aliases.instance)?,
        name: compile_patterns(&config.aliases.name)?,
        app_id: compile_patterns(&config.aliases.app_id)?,
    })
}
