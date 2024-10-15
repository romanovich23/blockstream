use regex::Regex;
use serde::Deserialize;
use std::{env, fs};

#[derive(Debug, Deserialize)]
pub struct Filter {
    pub contract_address: String,
    pub event_signature: String,
}

#[derive(Debug, Deserialize)]
pub struct Network {
    pub url: String,
    pub chain_id: u64,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub network: Network,
    pub filters: Vec<Filter>,
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

fn load_config_by_filename(filename: &str) -> Config {
    let contents = fs::read_to_string(filename).expect("Error reading the file");
    let substituted = substitute_env_variables(&contents);
    serde_yaml::from_str(&substituted).expect("Error parsing YAML")
}

pub fn load_config(env: Option<String>) -> Config {
    let filename: String = match env {
        Some(env) => format!("resources/application-{}.yml", env),
        None => "resources/application.yml".to_string(),
    };
    load_config_by_filename(&filename)
}
