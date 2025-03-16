use std::{
    fs::{File, OpenOptions},
    io::{self, Write},
};

pub type Log<'a> = &'a [u8];

pub trait Logger {
    fn log(&mut self, log: Log) -> io::Result<()>;
}

pub struct FileLogHandler {
    file: File,
}

impl FileLogHandler {
    pub fn new(path: String) -> io::Result<Self> {
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
}

impl Logger for FileLogHandler {
    fn log(&mut self, log: Log) -> io::Result<()> {
        self.file.write_all(log)?;
        Ok(())
    }
}

pub enum LogHandler {
    File(FileLogHandler),
}

impl Logger for LogHandler {
    fn log(&mut self, log: Log) -> io::Result<()> {
        match self {
            Self::File(x) => x.log(log),
        }
    }
}
