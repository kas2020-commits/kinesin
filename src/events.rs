use nix::sys::{
    epoll::{Epoll, EpollCreateFlags, EpollEvent, EpollFlags},
    signal::{SigSet, Signal},
    signalfd::SignalFd,
};
use std::{io, os::unix::io::AsRawFd};

use crate::services::ServiceRegistry;

pub fn setup_epoll(registry: &ServiceRegistry) -> io::Result<(SignalFd, Epoll)> {
    // Setup the Signal Set
    let mut sigset = SigSet::empty();
    sigset.add(Signal::SIGCHLD);

    // Block SIGCHLD so it doesn't interrupt other syscalls
    sigset.thread_block()?;

    // Create the fd for SIGCHLD
    let signal_fd = SignalFd::new(&sigset)?;

    // create epoll
    let epoll = Epoll::new(EpollCreateFlags::EPOLL_CLOEXEC)?;

    // Register the signal fd with epoll
    let event = EpollEvent::new(EpollFlags::EPOLLIN, signal_fd.as_raw_fd() as u64);
    epoll.add(&signal_fd, event)?;

    for srvc in registry {
        let stdout_event = EpollEvent::new(
            EpollFlags::EPOLLIN,
            srvc.lock().unwrap().stdout.as_raw_fd() as u64,
        );
        let stderr_event = EpollEvent::new(
            EpollFlags::EPOLLIN,
            srvc.lock().unwrap().stderr.as_raw_fd() as u64,
        );
        epoll.add(&srvc.lock().unwrap().stdout, stdout_event)?;
        epoll.add(&srvc.lock().unwrap().stderr, stderr_event)?;
    }

    Ok((signal_fd, epoll))
}
