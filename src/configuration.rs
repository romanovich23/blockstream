use crate::blockchain::configuration::Configuration;
use regex::Regex;
use std::{env, fs};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Error reading the file: {0}")]
    FileReadError(String),
    #[error("Error parsing YAML: {0}")]
    YamlParseError(String),
}

fn substitute_env_variables(contents: &str) -> String {
    let re = Regex::new(r"\$\{([^:}]+):?([^}]*)\}").unwrap();
    re.replace_all(contents, |caps: &regex::Captures| {
        let var_name = &caps[1];
        let default_value = &caps[2];
        env::var(var_name).unwrap_or_else(|_| default_value.to_string())
    })
    .to_string()
}

fn load_config_by_filename(filename: &str) -> Result<Configuration, ConfigError> {
    let contents = fs::read_to_string(filename)
        .map_err(|_| ConfigError::FileReadError(filename.to_string()))?;
    let substituted = substitute_env_variables(&contents);
    serde_yaml::from_str(&substituted)
        .map_err(|_| ConfigError::YamlParseError(filename.to_string()))
}

pub fn load_config(env: Option<String>) -> Result<Configuration, ConfigError> {
    let filename = match env {
        Some(env) => format!("resources/application-{}.yml", env),
        None => "resources/application.yml".to_string(),
    };
    load_config_by_filename(&filename)
}
