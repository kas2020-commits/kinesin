use nix::{
    poll::{poll, PollFd, PollFlags, PollTimeout},
    sys::{
        signal::{SigSet, Signal},
        signalfd::SignalFd,
    },
};
use std::{
    collections::HashMap,
    io,
    os::{
        fd::{AsFd, BorrowedFd, RawFd},
        unix::io::AsRawFd,
    },
};

use crate::{buffd::BufFd, utils::set_fd_nonblocking};

use super::{AsWatcher, Event};

pub struct PollWatcher {
    signal_fd: SignalFd,
    fdstore: HashMap<RawFd, BufFd>,
}

impl PollWatcher {
    pub fn new() -> Self {
        println!("using poll");

        // Create the fd for SIGCHLD
        let signal_fd = SignalFd::new(&SigSet::all()).unwrap();

        set_fd_nonblocking(signal_fd.as_raw_fd()).expect("Couldn't set signal_fd to O_NONBLOCK");

        let fdstore = HashMap::new();

        Self { signal_fd, fdstore }
    }

    fn poll(&'_ mut self, timeout: PollTimeout) -> std::io::Result<Option<Event<'_>>> {
        let mut fds: Vec<PollFd<'_>> = self
            .fdstore
            .keys()
            .map(|fd| PollFd::new(unsafe { BorrowedFd::borrow_raw(*fd) }, PollFlags::POLLIN))
            .collect();

        fds.push(PollFd::new(self.signal_fd.as_fd(), PollFlags::POLLIN));

        let num_fds = poll(fds.as_mut_slice(), timeout)?;

        assert!(num_fds >= 1);

        let is_signal_raised = fds
            .iter()
            .filter(|fd| fd.any().unwrap_or(false))
            .any(|fd| fd.as_fd().as_raw_fd() == self.signal_fd.as_raw_fd());

        let maybe_pollfd = fds.iter().find(|fd| fd.any().unwrap_or(false));

        // prioritize signals first
        if is_signal_raised {
            let siginfo =
                (self.signal_fd.read_signal()?).expect("failed to interpret signal from file");

            Ok(Some(Event::Signal(Signal::try_from(
                siginfo.ssi_signo as i32,
            )?)))
        } else if let Some(pollfd) = maybe_pollfd {
            let fd = pollfd.as_fd().as_raw_fd();
            if let Some(buf_fd) = self.fdstore.get_mut(&fd) {
                if buf_fd.read(None)? > 0 {
                    Ok(Some(Event::File(fd, buf_fd.data())))
                } else {
                    Ok(None)
                }
            } else {
                panic!("received an event for an fd not in the store");
            }
        } else {
            panic!("Woke up for an event but couldn't find any");
        }
    }
}

impl AsWatcher for PollWatcher {
    fn watch_fd(&mut self, fd: std::os::unix::prelude::RawFd, buffsize: usize) {
        if self.fdstore.contains_key(&fd) {
            eprintln!("fd is already being watched!");
            return;
        }
        let buf_fd = BufFd::new(fd, buffsize);
        self.fdstore.insert(fd, buf_fd);
    }

    fn poll_block(&'_ mut self) -> io::Result<Option<Event<'_>>> {
        self.poll(PollTimeout::NONE)
    }

    fn poll_no_block(&'_ mut self) -> std::io::Result<Option<Event<'_>>> {
        self.poll(PollTimeout::ZERO)
    }
}
