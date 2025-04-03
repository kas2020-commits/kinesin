//! Uses clap to define the CLI interface declaratively.
use std::path::PathBuf;

use clap::{command, Parser};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// pass an initial grid state as an ASCII file
    #[arg(short, default_value = "kinesin.toml", long, value_name = "FILE")]
    pub config: PathBuf,
}
