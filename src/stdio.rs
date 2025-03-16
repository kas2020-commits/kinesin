use crate::logging::{Log, LogHandler, Logger};
use std::io;

const BUFSIZE: usize = 128;

pub struct StdIoBuf {
    buf: [u8; BUFSIZE],
    len: usize,
    callbacks: Vec<LogHandler>,
}

impl StdIoBuf {
    pub fn new(callbacks: Vec<LogHandler>) -> Self {
        Self {
            buf: [0; BUFSIZE],
            len: 0,
            callbacks,
        }
    }

    pub fn flush(&mut self) -> io::Result<()> {
        // Execute all callbacks on the current buffer
        for callback in &mut self.callbacks {
            callback.log(&self.buf[..self.len])?;
        }

        // Reset the buffer after flushing
        self.len = 0;
        Ok(())
    }

    pub fn write(&mut self, log: Log) -> io::Result<()> {
        let mut log_left = log.len();

        while log_left > 0 {
            // Ensure that we don't write beyond the buffer's capacity
            let available_space = BUFSIZE - self.len;
            let data_to_write = std::cmp::min(available_space, log_left);

            // slice out the part of the log we're going to use
            let start_idx = log.len() - log_left;
            let slice = &log[start_idx..start_idx + data_to_write];

            // Copy the data into the buffer
            self.buf[self.len..self.len + data_to_write].copy_from_slice(slice);

            // Update the buffer length
            self.len += data_to_write;

            // If the buffer is full, flush it automatically
            if self.len == BUFSIZE {
                self.flush()?;
            }

            // update the log amount left
            log_left -= data_to_write;
        }

        Ok(())
    }
}

impl Drop for StdIoBuf {
    fn drop(&mut self) {
        // Ensure the buffer is flushed when the struct is dropped
        if self.len > 0 {
            match self.flush() {
                Ok(_) => (),
                Err(e) => {
                    eprintln!("Failed to flush buffer: {}", e);
                }
            }
        }
    }
}
