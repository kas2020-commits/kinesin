//! Defines Supported Consumers.
//!
//! Consumers recieve byte streams from the Bus, at which point they can chose
//! to do whatever they want with that data. Since different consumers must live
//! together in the same container, an overall Consumer enum has to exist which
//! encapsulates at runtime the differences between the real consumers.
use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
};

use crate::conf::ConsumerKind;

pub struct FileLogger {
    file: File,
}

impl FileLogger {
    pub fn new<T>(path: T) -> io::Result<Self>
    where
        T: AsRef<std::path::Path>,
    {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|e| {
                eprintln!("Failed to open log file: {}", e);
                e
            })?;

        Ok(Self { file })
    }

    fn write(&mut self, log: &[u8]) -> io::Result<()> {
        self.file.write_all(log)?;
        Ok(())
    }
}

pub enum Consumer {
    File(FileLogger),
    StdOut(io::Stdout),
    StdErr(io::Stderr),
}

impl Consumer {
    pub fn from_conf(conf: ConsumerKind) -> io::Result<Self> {
        match conf {
            ConsumerKind::Log(path) => Ok(Self::File(FileLogger::new(path)?)),
            ConsumerKind::StdOut => Ok(Self::StdOut(io::stdout())),
            ConsumerKind::StdErr => Ok(Self::StdErr(io::stderr())),
        }
    }

    pub fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        match self {
            Self::File(file) => file.write(bytes),
            Self::StdOut(stdout) => {
                stdout.lock().write_all(bytes)?;
                Ok(())
            }
            Self::StdErr(stderr) => {
                stderr.lock().write_all(bytes)?;
                Ok(())
            }
        }
    }
}
