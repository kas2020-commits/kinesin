use nix::sys::{
    epoll::{Epoll, EpollCreateFlags, EpollEvent, EpollFlags, EpollTimeout},
    signal::{SigSet, Signal},
    signalfd::SignalFd,
};
use std::{io, os::unix::io::AsRawFd};

use crate::{conf::Config, registry::Registry};

pub struct Supervisor {
    config: Config,
    registry: Registry,
    event_buffer: [EpollEvent; 10],
    signal_fd: SignalFd,
    epoll: Epoll,
}

impl Supervisor {
    pub fn new(config: Config) -> Self {
        let registry = Registry::new();
        let event_buffer = [EpollEvent::empty(); 10];

        // Setup the Signal Set
        let mut sigset = SigSet::empty();
        sigset.add(Signal::SIGCHLD);

        // Block SIGCHLD so it doesn't interrupt other syscalls
        sigset.thread_block().unwrap();

        // Create the fd for SIGCHLD
        let signal_fd = SignalFd::new(&sigset).unwrap();

        // create epoll
        let epoll = Epoll::new(EpollCreateFlags::EPOLL_CLOEXEC).unwrap();

        // Register the signal fd with epoll
        let event = EpollEvent::new(EpollFlags::EPOLLIN, signal_fd.as_raw_fd() as u64);
        epoll.add(&signal_fd, event).unwrap();

        Self {
            config,
            registry,
            event_buffer,
            signal_fd,
            epoll,
        }
    }

    fn handle_events(&mut self, num_fds: usize) -> io::Result<()> {
        let events = &self.event_buffer[..num_fds];
        for event in events {
            let data = event.data();
            if data == self.signal_fd.as_raw_fd() as u64 {
                if let Some(signal) = self.signal_fd.read_signal()? {
                    if signal.ssi_signo == Signal::SIGCHLD as u32 {
                        self.registry.reap_children();
                    }
                }
            } else if let Some(srvc) = self.registry.get_srvc_form_stdout(data as i32) {
                let mut srvc = srvc.lock().unwrap();
                srvc.stdout.read()?;
            } else if let Some(srvc) = self.registry.get_srvc_from_stderr(data as i32) {
                let mut srvc = srvc.lock().unwrap();
                srvc.stderr.read()?;
            }
        }
        Ok(())
    }

    pub fn run(&mut self) -> io::Result<()> {
        self.registry.start_services(&self.config);
        for srvc in &self.registry {
            let srvc_locked = srvc.lock().unwrap();
            self.epoll.add(
                &srvc_locked.stdout.fd,
                EpollEvent::new(EpollFlags::EPOLLIN, srvc_locked.stdout.as_raw_fd() as u64),
            )?;
            self.epoll.add(
                &srvc_locked.stderr.fd,
                EpollEvent::new(EpollFlags::EPOLLIN, srvc_locked.stderr.as_raw_fd() as u64),
            )?;
        }
        while !self.registry.is_empty() {
            let num_fds = self
                .epoll
                .wait(&mut self.event_buffer, EpollTimeout::NONE)?;
            self.handle_events(num_fds)?;
        }
        Ok(())
    }
}
