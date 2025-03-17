mod cli;
mod conf;
mod exec;
mod logging;
mod registry;
mod service;
mod stdio;
mod supervisor;
use crate::supervisor::Supervisor;
use clap::Parser;
use cli::Cli;
use conf::Config;
use std::{fs, io};

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let config: Config = toml::from_str(&fs::read_to_string(cli.config).unwrap()).unwrap();

    let mut supervisor = Supervisor::new(config);

    supervisor.start()
}
