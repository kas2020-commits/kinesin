//! An owned file descriptor with a co-located buffer attached.
//!
//! This abstraction comes in handy when needing to bundle together a file
//! descriptor with an associated bytes buffer to perform I/O, where the driver
//! of the I/O is abstracted away. The benifit is that we provide a stable,
//! stack-allocated buffer that is garunteed to have the same lifetime
//! as the file descriptor itself, meaning as long as you can still reference
//! the file descriptor owned by this struct, you can also reference the buffer
//! you'd provide to the kernel to get your data I/O performed.
use nix::errno::Errno;

use std::os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd};

const IO_BUFSIZE: usize = 10;

#[derive(Debug)]
pub struct BufFd {
    fd: OwnedFd,
    buffer: [u8; IO_BUFSIZE],
    curr_len: usize,
}

impl BufFd {
    pub fn new(fd: OwnedFd) -> Self {
        let input_buffer = [0; IO_BUFSIZE];
        Self {
            fd,
            buffer: input_buffer,
            curr_len: 0,
        }
    }

    pub fn as_fd(&self) -> BorrowedFd {
        self.fd.as_fd()
    }

    pub fn capacity(&self) -> usize {
        IO_BUFSIZE
    }

    pub fn set_len(&mut self, n: usize) {
        self.curr_len = n;
    }

    pub fn len(&self) -> usize {
        self.curr_len
    }

    pub fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.buffer.as_ptr()
    }

    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.buffer.as_mut_ptr()
    }

    pub fn data(&self) -> &[u8] {
        &self.buffer[..self.curr_len]
    }

    pub fn read(&mut self) -> Result<(), Errno> {
        match nix::unistd::read(self.fd.as_raw_fd(), &mut self.buffer) {
            Ok(0) => {
                self.curr_len = 0;
                Ok(())
            }
            Ok(n) => {
                self.curr_len = n;
                Ok(())
            }
            Err(nix::errno::Errno::EAGAIN) => {
                // we don't mind no more data because we already epoll the fd.
                // If we need to read more data we will be called again.
                self.curr_len = 0;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }
}
