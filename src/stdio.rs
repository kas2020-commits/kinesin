use io_uring::{opcode, squeue, types};

use crate::logging::{LogHandler, Logger};
use std::{
    io,
    os::fd::{AsRawFd, OwnedFd, RawFd},
};

const BUFSIZE: usize = 10;

pub struct StdIo {
    pub fd: OwnedFd,
    flush_buffer: [u8; BUFSIZE],
    ring_buffer: [u8; BUFSIZE],
    len: usize,
    callbacks: Vec<LogHandler>,
}

impl StdIo {
    pub fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }

    pub fn new(fd: OwnedFd, callbacks: Vec<LogHandler>) -> Self {
        Self {
            fd,
            flush_buffer: [0; BUFSIZE],
            ring_buffer: [0; BUFSIZE],
            len: 0,
            callbacks,
        }
    }

    pub fn entry(&mut self) -> squeue::Entry {
        opcode::Read::new(
            types::Fd(self.fd.as_raw_fd()),
            self.ring_buffer.as_mut_ptr(),
            self.ring_buffer.len() as _,
        )
        .build()
        .user_data(self.fd.as_raw_fd() as u64)
    }

    pub fn flush(&mut self) -> io::Result<()> {
        // Execute all callbacks on the current buffer
        for callback in &mut self.callbacks {
            callback.log(&self.flush_buffer[..self.len])?;
        }

        // Reset the buffer after flushing
        self.len = 0;
        Ok(())
    }

    pub fn pipe(&mut self, n: usize) -> io::Result<()> {
        let ring_buf_len = self.ring_buffer[..n].len();
        let mut log_left = ring_buf_len;
        // if self.read_buffer.len() == self.write_buffer.len() this loop is bounded to
        // at most 2 iterations.
        while log_left > 0 {
            // Ensure that we don't write beyond the buffer's capacity
            let available_space = BUFSIZE - self.len;
            let data_to_write = std::cmp::min(available_space, log_left);

            // slice out the part of the log we're going to use
            let start_idx = ring_buf_len - log_left;
            let slice = &self.ring_buffer[..n][start_idx..start_idx + data_to_write];

            // Copy the data into the buffer
            self.flush_buffer[self.len..self.len + data_to_write].copy_from_slice(slice);

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

impl Drop for StdIo {
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
