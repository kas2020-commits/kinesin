//! The Serializable configuration data structures used for setup.
use std::{ffi::CString, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ProducerConf {
    StdOut(String),
    StdErr(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ConsumerKind {
    Log(PathBuf),
    StdOut,
    StdErr,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConsumerConf {
    pub consumes: ProducerConf,
    pub kind: ConsumerKind,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SourceConf {
    #[serde(default = "default_src_watch")]
    pub watch: bool,

    #[serde(default = "default_read_bufsize")]
    pub read_bufsize: usize,

    #[serde(default = "default_bus_bufsize")]
    pub bus_bufsize: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceConf {
    pub name: String,

    #[serde(default = "default_src_config")]
    pub stdout: SourceConf,

    #[serde(default = "default_src_config")]
    pub stderr: SourceConf,

    pub exec: Vec<CString>,

    #[serde(default = "default_cfg_env")]
    pub env: Vec<CString>,

    #[serde(default = "default_must_be_up")]
    pub must_be_up: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    #[serde(default = "default_cfg_ver")]
    pub version: u32,
    pub service: Vec<ServiceConf>,
    pub consumer: Vec<ConsumerConf>,
}

fn default_src_config() -> SourceConf {
    SourceConf {
        watch: default_src_watch(),
        read_bufsize: default_read_bufsize(),
        bus_bufsize: default_bus_bufsize(),
    }
}

fn default_cfg_ver() -> u32 {
    1
}

fn default_must_be_up() -> bool {
    true
}

fn default_src_watch() -> bool {
    true
}

fn default_read_bufsize() -> usize {
    2048
}

fn default_bus_bufsize() -> usize {
    0
}

fn default_cfg_env() -> Vec<CString> {
    Vec::new()
}
