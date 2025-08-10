use nix::sys::{
    epoll::{Epoll, EpollCreateFlags, EpollEvent, EpollFlags, EpollTimeout},
    signal::{SigSet, Signal},
    signalfd::SignalFd,
};
use std::{
    collections::HashMap,
    io,
    os::{
        fd::{BorrowedFd, RawFd},
        unix::io::AsRawFd,
    },
};

use crate::{buffd::BufFd, utils::set_fd_nonblocking};

use super::{AsWatcher, Event};

pub struct EpollWatcher {
    event_buffer: [EpollEvent; 1],
    signal_fd: SignalFd,
    epoll: Epoll,
    fdstore: HashMap<RawFd, BufFd>,
}

impl EpollWatcher {
    pub fn new() -> Self {
        let event_buffer = [EpollEvent::empty(); 1];

        // Create the fd for SIGCHLD
        let signal_fd = SignalFd::new(&SigSet::all()).unwrap();

        // create epoll
        let epoll = Epoll::new(EpollCreateFlags::EPOLL_CLOEXEC).unwrap();

        set_fd_nonblocking(signal_fd.as_raw_fd()).expect("Couldn't set signal_fd to O_NONBLOCK");

        // Register the signal fd with epoll
        let event = EpollEvent::new(EpollFlags::EPOLLIN, signal_fd.as_raw_fd() as _);
        epoll.add(&signal_fd, event).unwrap();

        let fdstore = HashMap::new();

        Self {
            event_buffer,
            signal_fd,
            epoll,
            fdstore,
        }
    }

    fn epoll(&mut self, timeout: EpollTimeout) -> io::Result<Option<Event>> {
        let num_fds = self.epoll.wait(&mut self.event_buffer, timeout)?;

        if num_fds > 1 {
            eprintln!("Epoll reported more FDs than was given in the buffer.");
            panic!();
        }

        if num_fds == 0 {
            return Ok(None);
        }

        let event = self.event_buffer[0];
        let data = event.data();

        if data == self.signal_fd.as_raw_fd() as u64 {
            let siginfo =
                (self.signal_fd.read_signal()?).expect("failed to interpret signal from file");

            Ok(Some(Event::Signal(Signal::try_from(
                siginfo.ssi_signo as i32,
            )?)))
        } else {
            if let Some(buf_fd) = self.fdstore.get_mut(&(data as _)) {
                if buf_fd.read(None)? > 0 {
                    Ok(Some(Event::File(data as _, buf_fd.data())))
                } else {
                    Ok(None)
                }
            } else {
                eprintln!("received an event for an fd not in the store...?");
                panic!();
            }
        }
    }
}

impl AsWatcher for EpollWatcher {
    fn watch_fd(&mut self, fd: RawFd, buffsize: usize) {
        if self.fdstore.contains_key(&fd) {
            eprintln!("fd is already being watched!");
            return;
        }
        let buf_fd = BufFd::new(fd, buffsize);
        let borrowed_fd = unsafe { BorrowedFd::borrow_raw(fd) };
        // register interest of fd to kernel
        self.epoll
            .add(borrowed_fd, EpollEvent::new(EpollFlags::EPOLLIN, fd as _))
            .unwrap();
        // become owner of fd and its userspace buffer
        self.fdstore.insert(fd, buf_fd);
    }

    fn poll_block(&mut self) -> io::Result<Option<Event>> {
        self.epoll(EpollTimeout::NONE)
    }

    fn poll_no_block(&mut self) -> io::Result<Option<Event>> {
        self.epoll(EpollTimeout::ZERO)
    }
}
