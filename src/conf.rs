use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceConf {
    pub name: String,
    pub exec: Vec<String>,
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
