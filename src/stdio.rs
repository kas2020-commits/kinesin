use nix::{
    errno::Errno,
    sys::epoll::{Epoll, EpollEvent, EpollFlags},
    unistd::read,
};

use crate::logging::{LogHandler, Logger};
use std::{
    io,
    os::fd::{AsRawFd, OwnedFd, RawFd},
};

const BUFSIZE: usize = 128;

pub struct StdIo {
    pub fd: OwnedFd,
    read_buffer: [u8; BUFSIZE],
    write_buffer: [u8; BUFSIZE],
    len: usize,
    callbacks: Vec<LogHandler>,
}

impl StdIo {
    pub fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }

    pub fn add_to_epoll(&self, epoll: &Epoll) -> io::Result<()> {
        let event = EpollEvent::new(EpollFlags::EPOLLIN, self.fd.as_raw_fd() as u64);
        epoll.add(&self.fd, event)?;
        Ok(())
    }

    pub fn new(fd: OwnedFd, callbacks: Vec<LogHandler>) -> Self {
        Self {
            fd,
            read_buffer: [0; BUFSIZE],
            write_buffer: [0; BUFSIZE],
            len: 0,
            callbacks,
        }
    }

    pub fn read(&mut self) -> io::Result<usize> {
        let mut bytes_read = 0;

        loop {
            match read(self.fd.as_raw_fd(), &mut self.read_buffer) {
                Ok(0) => {
                    break;
                }
                Ok(n) => {
                    self.pipe(n)?;
                    bytes_read += n;
                }
                Err(Errno::EAGAIN) => {
                    // we don't mind no more data because we already epoll the fd.
                    // If we need to read more data we will be called again.
                    break;
                }
                Err(e) => {
                    return Err(io::Error::new(io::ErrorKind::Other, e));
                }
            }
        }

        Ok(bytes_read)
    }

    pub fn flush(&mut self) -> io::Result<()> {
        // Execute all callbacks on the current buffer
        for callback in &mut self.callbacks {
            callback.log(&self.write_buffer[..self.len])?;
        }

        // Reset the buffer after flushing
        self.len = 0;
        Ok(())
    }

    fn pipe(&mut self, n: usize) -> io::Result<()> {
        let read_buffer_len = self.read_buffer[..n].len();
        let mut log_left = read_buffer_len;

        // if self.read_buffer.len() == self.write_buffer.len() this loop is bounded to
        // at most 2 iterations.
        while log_left > 0 {
            // Ensure that we don't write beyond the buffer's capacity
            let available_space = BUFSIZE - self.len;
            let data_to_write = std::cmp::min(available_space, log_left);

            // slice out the part of the log we're going to use
            let start_idx = read_buffer_len - log_left;
            let slice = &self.read_buffer[..n][start_idx..start_idx + data_to_write];

            // Copy the data into the buffer
            self.write_buffer[self.len..self.len + data_to_write].copy_from_slice(slice);

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
