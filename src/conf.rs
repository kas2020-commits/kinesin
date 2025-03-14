use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Service {
    pub name: String,
    pub cmd: String,
    pub args: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default = "default_cfg_ver")]
    pub version: u32,
    pub services: Vec<Service>,
}

fn default_cfg_ver() -> u32 {
    1
}
