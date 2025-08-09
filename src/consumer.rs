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
    StdOut,
    StdErr,
}

impl Consumer {
    pub fn write(&mut self, bytes: &[u8]) -> io::Result<()> {
        match self {
            Self::File(x) => x.write(bytes),
            Self::StdOut => {
                let stdout = io::stdout();
                let mut handle = stdout.lock();
                handle.write(bytes)?;
                Ok(())
            }
            Self::StdErr => {
                let stderr = io::stderr();
                let mut handle = stderr.lock();
                handle.write(bytes)?;
                Ok(())
            }
        }
    }
}
