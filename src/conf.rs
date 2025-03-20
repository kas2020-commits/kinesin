use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Producer {
    StdOut(String),
    StdErr(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ConsumerKind {
    Log(PathBuf),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Consumer {
    pub consumes: Producer,
    pub kind: ConsumerKind,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceConf {
    pub name: String,

    #[serde(default = "default_cfg_stdout")]
    pub stdout: bool,

    #[serde(default = "default_cfg_stderr")]
    pub stderr: bool,

    pub exec: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default = "default_cfg_ver")]
    pub version: u32,
    pub service: Vec<ServiceConf>,
    pub consumer: Vec<Consumer>,
}

fn default_cfg_ver() -> u32 {
    1
}

fn default_cfg_stdout() -> bool {
    true
}

fn default_cfg_stderr() -> bool {
    true
}
