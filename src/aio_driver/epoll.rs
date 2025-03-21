use nix::sys::{
    epoll::{Epoll, EpollCreateFlags, EpollEvent, EpollFlags, EpollTimeout},
    signal::{SigSet, Signal},
    signalfd::SignalFd,
};
use std::{io, os::unix::io::AsRawFd};

use crate::utils::set_fd_nonblocking;

use super::{AioDriver, Notification};

pub struct EpollDriver {
    mask: SigSet,
    event_buffer: [EpollEvent; 1],
    signal_fd: SignalFd,
    epoll: Epoll,
}

impl EpollDriver {
    pub fn new() -> Self {
        let event_buffer = [EpollEvent::empty(); 1];

        // Setup the Signal Set
        let mask = SigSet::empty();

        // Create the fd for SIGCHLD
        let signal_fd = SignalFd::new(&mask).unwrap();

        // create epoll
        let epoll = Epoll::new(EpollCreateFlags::EPOLL_CLOEXEC).unwrap();

        set_fd_nonblocking(signal_fd.as_raw_fd()).expect("Couldn't set signal_fd to O_NONBLOCK");

        // Register the signal fd with epoll
        let event = EpollEvent::new(EpollFlags::EPOLLIN, signal_fd.as_raw_fd() as u64);
        epoll.add(&signal_fd, event).unwrap();

        Self {
            mask,
            event_buffer,
            signal_fd,
            epoll,
        }
    }
}

impl AioDriver for EpollDriver {
    fn is_proactive(&self) -> bool {
        false
    }

    fn is_oneshot(&self) -> bool {
        false
    }

    fn proactive_result(&self) -> Option<i32> {
        None
    }

    fn register_signal(&mut self, signal: Signal) {
        self.mask.add(signal);
        self.signal_fd.set_mask(&self.mask).unwrap();
    }

    fn register_fd(&mut self, buf_fd: &mut crate::buffd::BufFd) {
        self.epoll
            .add(
                buf_fd.as_fd(),
                EpollEvent::new(EpollFlags::EPOLLIN, buf_fd.as_raw_fd() as u64),
            )
            .unwrap();
    }

    fn block_next_notif(&mut self) -> io::Result<Notification> {
        let num_fds = self
            .epoll
            .wait(&mut self.event_buffer, EpollTimeout::NONE)?;

        if num_fds > 1 {
            eprintln!("Epoll reported more FDs than was given in the buffer.");
            panic!();
        }

        let event = self.event_buffer[0];
        let data = event.data();

        if data == self.signal_fd.as_raw_fd() as u64 {
            let siginfo = self.signal_fd.read_signal()?;
            if let Some(sig) = siginfo {
                Ok(Notification::Signal(Signal::try_from(
                    sig.ssi_signo as i32,
                )?))
            } else {
                panic!();
            }
        } else {
            Ok(Notification::File(data as _))
        }
    }
}
