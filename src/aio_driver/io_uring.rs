use io_uring::{opcode, types, IoUring};

use nix::{
    libc::signalfd_siginfo,
    sys::{
        signal::{SigSet, Signal},
        signalfd::SignalFd,
    },
};
use std::{io, mem, os::unix::io::AsRawFd};

use super::{AsDriver, Notification};
use crate::buffd::BufFd;
use crate::utils::set_fd_nonblocking;

const IO_URING_ENTRIES: u32 = 32;

// This is based on the size of signalfd_siginfo, please do not change.
const IO_URING_SIG_BUF_SIZE: usize = 128;

pub struct IoUringDriver {
    result: Option<i64>,
    mask: SigSet,
    signal_fd: SignalFd,
    signal_buffer: [u8; IO_URING_SIG_BUF_SIZE],
    ring: IoUring,
}

impl IoUringDriver {
    pub fn new() -> Self {
        let signal_buffer = [0; IO_URING_SIG_BUF_SIZE];

        // initialize the sigset
        let mask = SigSet::empty();

        // Create the fd for SIGCHLD
        let signal_fd = SignalFd::new(&mask).unwrap();

        set_fd_nonblocking(signal_fd.as_raw_fd()).expect("Couldn't set signal_fd to O_NONBLOCK");

        // Setup io_uring
        let ring = IoUring::new(IO_URING_ENTRIES).unwrap();

        Self {
            result: None,
            mask,
            signal_fd,
            ring,
            signal_buffer,
        }
    }

    fn load_from_sigbuf(&self, n: usize) -> signalfd_siginfo {
        let mut buffer = mem::MaybeUninit::<signalfd_siginfo>::uninit();
        let size = mem::size_of_val(&buffer);
        let sigbuf = &self.signal_buffer[..n];
        let buffer_ptr = buffer.as_mut_ptr() as *mut u8;
        unsafe {
            // Copy the data from sigbuf into the uninitialized buffer
            std::ptr::copy_nonoverlapping(sigbuf.as_ptr(), buffer_ptr, size);
        }
        unsafe { buffer.assume_init() }
    }
}

impl AsDriver for IoUringDriver {
    fn is_proactive(&self) -> bool {
        true
    }

    fn is_oneshot(&self) -> bool {
        true
    }

    fn proactive_result(&self) -> Option<i64> {
        self.result
    }

    fn register_signal(&mut self, signal: Signal) {
        self.mask.add(signal);
        self.signal_fd.set_mask(&self.mask).unwrap();

        let signal_e = opcode::Read::new(
            types::Fd(self.signal_fd.as_raw_fd()),
            self.signal_buffer.as_mut_ptr(),
            self.signal_buffer.len() as _,
        )
        .build()
        .user_data(self.signal_fd.as_raw_fd() as _);
        unsafe {
            self.ring
                .submission()
                .push(&signal_e)
                .expect("Submission queue is full");
        }
    }

    fn register_fd(&mut self, buf_fd: &mut BufFd) {
        let entry = opcode::Read::new(
            types::Fd(buf_fd.as_raw_fd()),
            buf_fd.as_mut_ptr(),
            buf_fd.capacity() as _,
        )
        .build()
        .user_data(buf_fd.as_raw_fd() as u64);
        unsafe { self.ring.submission().push(&entry).unwrap() };
    }

    fn block_next_notif(&mut self) -> io::Result<Notification> {
        self.ring.submit_and_wait(1)?;
        let cqe = self
            .ring
            .completion()
            .next()
            .expect("No completion entries");
        let usr_data = cqe.user_data();
        self.result = Some(cqe.result() as _);
        if usr_data == self.signal_fd.as_raw_fd() as _ {
            let siginfo = self.load_from_sigbuf(cqe.result() as _);
            Ok(Notification::Signal(Signal::try_from(
                siginfo.ssi_signo as i32,
            )?))
        } else {
            Ok(Notification::File(usr_data as i32))
        }
    }
}
