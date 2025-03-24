//! An owned file descriptor to a stream-oriented file and a co-located buffer.
//!
//! This abstraction comes in handy when needing to bundle together a file
//! descriptor with an associated bytes buffer to perform I/O, where the driver
//! of the I/O is abstracted away. The benifit is that we provide a stable,
//! stack-allocated buffer that is garunteed to have the same lifetime
//! as the file descriptor itself, meaning as long as you can still reference
//! the file descriptor owned by this struct, you can also reference the buffer
//! you'd provide to the kernel to get your data I/O performed.
use nix::fcntl::{self, OFlag};

#[cfg(not(feature = "io-uring"))]
use nix::errno::Errno;

use std::os::fd::{AsRawFd, RawFd};

const IO_BUFSIZE: usize = 10;

#[derive(Debug)]
pub struct BufFd {
    fd: RawFd,
    buffer: Box<[u8; IO_BUFSIZE]>,
    curr_len: usize,
}

impl BufFd {
    pub fn new(fd: RawFd) -> Self {
        let input_buffer = Box::new([0; IO_BUFSIZE]);
        if fcntl::OFlag::from_bits(fcntl::fcntl(fd.as_raw_fd(), fcntl::FcntlArg::F_GETFL).unwrap())
            .unwrap()
            .intersection(OFlag::O_NONBLOCK)
            != OFlag::O_NONBLOCK
        {
            eprintln!("O_NONBLOCK flag not set on fd. I/O operations may block!");
        }
        Self {
            fd,
            buffer: input_buffer,
            curr_len: 0,
        }
    }

    #[cfg(feature = "io-uring")]
    pub fn capacity(&self) -> usize {
        IO_BUFSIZE
    }

    #[cfg(feature = "io-uring")]
    pub fn as_raw_fd(&self) -> RawFd {
        self.fd.as_raw_fd()
    }

    #[cfg(feature = "io-uring")]
    pub fn set_len(&mut self, n: usize) {
        self.curr_len = n
    }

    #[cfg(feature = "io-uring")]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.buffer.as_mut_ptr()
    }

    pub fn data(&self) -> &[u8] {
        &self.buffer[..self.curr_len]
    }

    #[cfg(not(feature = "io-uring"))]
    pub fn read(&mut self, bytes_ready: Option<usize>) -> Result<usize, Errno> {
        match nix::unistd::read(self.fd.as_raw_fd(), self.buffer.as_mut_slice()) {
            Ok(0) => {
                self.curr_len = 0;
                Ok(0)
            }
            Ok(n) => {
                if let Some(num_bytes_ready) = bytes_ready {
                    if n != num_bytes_ready {
                        eprintln!(
                            "Was told {} bytes were ready, but read {} bytes instead",
                            num_bytes_ready, n
                        );
                    }
                }
                self.curr_len = n;
                Ok(n)
            }
            Err(nix::errno::Errno::EAGAIN) => {
                // we don't mind no more data because we already epoll the fd.
                // If we need to read more data we will be called again.
                self.curr_len = 0;
                Ok(0)
            }
            Err(e) => Err(e),
        }
    }
}
