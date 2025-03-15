use std::path::PathBuf;

use clap::{command, Parser};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// pass an initial grid state as an ASCII file
    #[arg(short, default_value = "rs-init.json", long, value_name = "FILE")]
    pub config: PathBuf,
}
