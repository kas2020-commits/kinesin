use std::fs;

use clap::Parser;
use serde::{Deserialize, Serialize};

use crate::{cli::Cli, service_def::ServiceDef};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceConf {
    pub name: String,
    pub cmd: String,
    pub args: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default = "default_cfg_ver")]
    pub version: u32,
    pub services: Vec<ServiceConf>,
}

fn default_cfg_ver() -> u32 {
    1
}

pub fn get_service_defs() -> Vec<ServiceDef> {
    let cli = Cli::parse();

    let config_str = fs::read_to_string(cli.config).expect("Couldn't  read config file");

    let config: Config = serde_json::from_str(&config_str).unwrap();

    config.services.iter().map(ServiceDef::new).collect()
}
