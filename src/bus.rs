use crate::logging::{LogHandler, Logger};
use std::io;

const IO_BUFSIZE: usize = 10;

pub struct Bus {
    buffer: [u8; IO_BUFSIZE],
    len: usize,
    consumers: Vec<LogHandler>,
}

impl Bus {
    pub fn new(consumers: Vec<LogHandler>) -> Self {
        Self {
            buffer: [0; IO_BUFSIZE],
            len: 0,
            consumers,
        }
    }

    pub fn flush(&mut self) -> io::Result<()> {
        // Execute all callbacks on the current buffer
        for callback in &mut self.consumers {
            callback.log(&self.buffer[..self.len])?;
        }

        // Reset the buffer after flushing
        self.len = 0;
        Ok(())
    }

    pub fn consume(&mut self, dat: &[u8]) -> io::Result<()> {
        let num_bytes = dat.len();
        let mut bytes_left = num_bytes;

        // if self.read_buffer.len() == self.write_buffer.len() this loop is bounded to
        // at most 2 iterations.
        while bytes_left > 0 {
            // Ensure that we don't write beyond the buffer's capacity
            let available_space = IO_BUFSIZE - self.len;
            let bytes_to_consume = std::cmp::min(available_space, bytes_left);

            // slice out the part of the log we're going to use
            let start_idx = num_bytes - bytes_left;
            let slice = &dat[start_idx..start_idx + bytes_to_consume];

            // Copy the data into the buffer
            self.buffer[self.len..self.len + bytes_to_consume].copy_from_slice(slice);

            // Update the buffer length
            self.len += bytes_to_consume;

            // If the buffer is full, flush it automatically
            if self.len == IO_BUFSIZE {
                self.flush()?;
            }

            // update the log amount left
            bytes_left -= bytes_to_consume;
        }
        Ok(())
    }
}

impl Drop for Bus {
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
