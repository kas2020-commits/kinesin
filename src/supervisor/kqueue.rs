use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

use crate::{conf::Config, registry::Registry};
use kqueue_sys::{kevent, kqueue, EventFilter, EventFlag, FilterFlag};
use nix::errno::Errno;
use nix::sys::signal::{SigSet, Signal};

pub struct Supervisor {
    config: Config,
    registry: Registry,
    kq: OwnedFd,
}

impl Supervisor {
    pub fn new(config: Config) -> Self {
        let registry = Registry::new();
        let kq_fd = unsafe { kqueue() };

        if kq_fd == -1 {
            eprintln!("Couldn't initialize the kqueue: {}", Errno::last());
            panic!();
        }

        let kq = unsafe { std::os::fd::OwnedFd::from_raw_fd(kq_fd) };

        // Setup the Signal Set
        let mut sigset = SigSet::empty();
        sigset.add(Signal::SIGCHLD);

        // This makes the thread no longer get interupted for signals in our
        // sigset, preventing split-brain issues by letting us respond to the
        // signal as a notification instead of as a special case.
        sigset.thread_block().unwrap();

        let sigev = kevent::new(
            Signal::SIGCHLD as _,
            EventFilter::EVFILT_SIGNAL,
            EventFlag::EV_ADD, // this is implicitly added anyways
            FilterFlag::empty(),
        );

        let changelist = [sigev];

        if unsafe {
            kevent(
                kq.as_raw_fd(),
                changelist.as_ptr() as _,
                changelist.len() as _,
                core::ptr::null_mut(),
                0,
                std::ptr::null(),
            )
        } == -1
        {
            eprintln!("Couldn't register signal kevent: {}", Errno::last());
            panic!();
        }

        Self {
            config,
            registry,
            kq,
        }
    }

    fn first_setup(&mut self) {
        self.registry.start_services(&self.config);
        for srvc in &self.registry {
            let srvc_locked = srvc.lock().unwrap();

            let stdoutev = kevent::new(
                srvc_locked.stdout.as_raw_fd() as _,
                EventFilter::EVFILT_READ,
                EventFlag::EV_ADD,
                FilterFlag::empty(),
            );

            let stderrev = kevent::new(
                srvc_locked.stderr.as_raw_fd() as _,
                EventFilter::EVFILT_READ,
                EventFlag::EV_ADD,
                FilterFlag::empty(),
            );

            let changelist = [stdoutev, stderrev];

            if unsafe {
                kevent(
                    self.kq.as_raw_fd(),
                    changelist.as_ptr() as _,
                    changelist.len() as _,
                    core::ptr::null_mut(),
                    0,
                    std::ptr::null(),
                )
            } == -1
            {
                eprintln!("Couldn't register signal kevent: {}", Errno::last());
                panic!();
            }
        }
    }

    fn handle_kevent(&mut self, event: &kevent) -> io::Result<()> {
        if (event.filter == EventFilter::EVFILT_SIGNAL) && (event.ident == Signal::SIGCHLD as _) {
            self.registry.reap_children();
        } else if let Some(srvc) = self.registry.get_srvc_form_stdout(event.ident as i32) {
            let mut srvc = srvc.lock().unwrap();
            srvc.stdout.read()?;
        } else if let Some(srvc) = self.registry.get_srvc_from_stderr(event.ident as i32) {
            let mut srvc = srvc.lock().unwrap();
            srvc.stderr.read()?;
        } else {
            eprintln!("got unexpected event: {:#?}", event);
        }
        Ok(())
    }

    pub fn run(&mut self) -> io::Result<()> {
        self.first_setup();
        let mut eventlist: [kevent; 1] = unsafe { std::mem::zeroed() };
        while !self.registry.is_empty() {
            if unsafe {
                kevent(
                    self.kq.as_raw_fd(),
                    core::ptr::null(),
                    0,
                    eventlist.as_mut_ptr(),
                    eventlist.len() as _,
                    std::ptr::null(),
                )
            } == -1
            {
                eprintln!("Couldn't register signal kevent: {}", Errno::last());
                panic!();
            }
            let event = eventlist[0];
            self.handle_kevent(&event)?;
        }
        Ok(())
    }
}
