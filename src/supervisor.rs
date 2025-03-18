use io_uring::{cqueue, opcode, types, IoUring};
use nix::{
    libc::signalfd_siginfo,
    sys::{
        signal::{SigSet, Signal},
        signalfd::SignalFd,
    },
};
use std::{io, mem, os::unix::io::AsRawFd};

use crate::{conf::Config, registry::Registry};

const IO_URING_ENTRIES: u32 = 32;

// This is based on the size of signalfd_siginfo, please do not change.
const IO_URING_SIG_BUF_SIZE: usize = 128;

pub struct Supervisor {
    config: Config,
    registry: Registry,
    signal_fd: SignalFd,
    signal_buffer: [u8; IO_URING_SIG_BUF_SIZE],
    ring: IoUring,
}

impl Supervisor {
    pub fn new(config: Config) -> Self {
        let registry = Registry::new();
        let signal_buffer = [0; IO_URING_SIG_BUF_SIZE];

        // Setup the Signal Set
        let mut sigset = SigSet::empty();
        sigset.add(Signal::SIGCHLD);

        // Block SIGCHLD so it doesn't interrupt other syscalls
        sigset.thread_block().unwrap();

        // Create the fd for SIGCHLD
        let signal_fd = SignalFd::new(&sigset).unwrap();

        // Setup io_uring
        let ring = IoUring::new(IO_URING_ENTRIES).unwrap();

        Self {
            config,
            registry,
            signal_fd,
            ring,
            signal_buffer,
        }
    }

    fn submit_signal_to_ring(&mut self) {
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

    fn first_setup(&mut self) {
        self.submit_signal_to_ring();
        self.registry.start_services(&self.config);
        for srvc in &self.registry {
            let mut srvc_locked = srvc.lock().unwrap();
            unsafe {
                self.ring
                    .submission()
                    .push(&srvc_locked.stdout.entry())
                    .unwrap();
                self.ring
                    .submission()
                    .push(&srvc_locked.stderr.entry())
                    .unwrap();
            }
        }
    }

    fn handle_cqe(&mut self, cqe: cqueue::Entry) -> io::Result<()> {
        let usr_data = cqe.user_data();
        if (usr_data as i32) == self.signal_fd.as_raw_fd() {
            let mut buffer = mem::MaybeUninit::<signalfd_siginfo>::uninit();
            let size = mem::size_of_val(&buffer);
            let sigbuf = &self.signal_buffer[..cqe.result() as _];
            let buffer_ptr = buffer.as_mut_ptr() as *mut u8;
            unsafe {
                // Copy the data from sigbuf into the uninitialized buffer
                std::ptr::copy_nonoverlapping(sigbuf.as_ptr(), buffer_ptr, size);
            }
            let siginfo = unsafe { buffer.assume_init() };

            match siginfo.ssi_signo {
                x if x == Signal::SIGCHLD as u32 => {
                    self.registry.reap_children();
                }
                _ => {}
            }

            self.submit_signal_to_ring();
        } else if let Some(srvc) = self.registry.get_srvc_form_stdout(usr_data as _) {
            let mut srvc_locked = srvc.lock().unwrap();
            srvc_locked.stdout.pipe(cqe.result() as _)?;
            unsafe {
                self.ring
                    .submission()
                    .push(&srvc_locked.stdout.entry())
                    .unwrap()
            };
        } else if let Some(srvc) = self.registry.get_srvc_from_stderr(usr_data as _) {
            let mut srvc_locked = srvc.lock().unwrap();
            srvc_locked.stderr.pipe(cqe.result() as _)?;
            unsafe {
                self.ring
                    .submission()
                    .push(&srvc_locked.stderr.entry())
                    .unwrap()
            };
        } else {
            eprintln!("Supervisor failed to match to a handler");
        }
        Ok(())
    }

    pub fn run(&mut self) -> io::Result<()> {
        self.first_setup();
        while !self.registry.is_empty() {
            self.ring.submit_and_wait(1)?;
            let cqe = self.ring.completion().next().expect("");
            self.handle_cqe(cqe)?;
        }
        Ok(())
    }
}
