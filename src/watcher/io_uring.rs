use io_uring::{opcode, types, IoUring};

use nix::{
    errno::Errno,
    libc::signalfd_siginfo,
    sys::{
        signal::{SigSet, Signal},
        signalfd::SignalFd,
    },
};
use std::{
    collections::HashMap,
    io, mem,
    os::{fd::RawFd, unix::io::AsRawFd},
};

use super::{AsWatcher, Event};
use crate::buffd::BufFd;
use crate::utils::set_fd_nonblocking;

const IO_URING_ENTRIES: u32 = 32;

// This is based on the size of signalfd_siginfo, please do not change.
const IO_URING_SIG_BUF_SIZE: usize = 128;

pub struct IoUringWatcher {
    mask: SigSet,
    signal_fd: SignalFd,
    signal_buffer: Box<[u8; IO_URING_SIG_BUF_SIZE]>,
    ring: IoUring,
    fdstore: HashMap<RawFd, BufFd>,
}

impl IoUringWatcher {
    pub fn new() -> Self {
        let signal_buffer = Box::new([0; IO_URING_SIG_BUF_SIZE]);

        // initialize the sigset
        let mask = SigSet::empty();

        // Create the fd for SIGCHLD
        let signal_fd = SignalFd::new(&mask).unwrap();

        set_fd_nonblocking(signal_fd.as_raw_fd()).expect("Couldn't set signal_fd to O_NONBLOCK");

        // Setup io_uring
        let ring = IoUring::new(IO_URING_ENTRIES).unwrap();

        let fdstore = HashMap::new();

        Self {
            mask,
            signal_fd,
            ring,
            signal_buffer,
            fdstore,
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

    fn poll_internal(&mut self, wait: bool) -> io::Result<Option<Event>> {
        if wait {
            self.ring.submit_and_wait(1).expect("blocking failed");
        } else {
            self.ring.submit().expect("submitting failed");
        }

        let cqe_wrapped = self.ring.completion().next();

        if cqe_wrapped.is_none() {
            return Ok(None);
        }

        let cqe = cqe_wrapped.unwrap();

        let usr_data = cqe.user_data();

        if usr_data == self.signal_fd.as_raw_fd() as u64 {
            let siginfo = self.load_from_sigbuf(cqe.result() as _);

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

            Ok(Some(Event::Signal(Signal::try_from(
                siginfo.ssi_signo as i32,
            )?)))
        } else {
            let _res = cqe.result();

            match _res {
                0 => {
                    self.fdstore.remove(&(usr_data as _));
                    Ok(None)
                }
                x if x < 0 => {
                    let bad = Errno::result(x).err().unwrap();
                    eprintln!("{}", bad);
                    panic!();
                }
                n if n > 0 => {
                    let buf_fd = self.fdstore.get_mut(&(usr_data as _)).unwrap();
                    buf_fd.set_len(n as _);
                    let entry = opcode::Read::new(
                        types::Fd(buf_fd.as_raw_fd()),
                        buf_fd.as_mut_ptr(),
                        buf_fd.capacity() as _,
                    )
                    .build()
                    .user_data(buf_fd.as_raw_fd() as u64);
                    unsafe {
                        self.ring
                            .submission()
                            .push(&entry)
                            .expect("coudln't push new entry")
                    };
                    Ok(Some(Event::File(usr_data as i32, buf_fd.data())))
                }
                _ => todo!(),
            }
        }
    }
}

impl AsWatcher for IoUringWatcher {
    fn watch_signal(&mut self, signal: Signal) {
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

    fn watch_fd(&mut self, fd: RawFd, buffsize: usize) {
        self.fdstore.insert(fd, BufFd::new(fd, buffsize));
        let buf_fd = self.fdstore.get_mut(&fd).unwrap();
        let entry = opcode::Read::new(types::Fd(fd), buf_fd.as_mut_ptr(), buf_fd.capacity() as _)
            .build()
            .user_data(buf_fd.as_raw_fd() as u64);
        unsafe { self.ring.submission().push(&entry).unwrap() };
    }

    fn poll_block(&mut self) -> io::Result<Option<Event>> {
        self.poll_internal(true)
    }

    fn poll_no_block(&mut self) -> io::Result<Option<Event>> {
        self.poll_internal(false)
    }
}
