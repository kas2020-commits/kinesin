use super::{AsWatcher, Event};
use crate::buffd::BufFd;
use nix::libc::timespec;
use nix::sys::event::{EventFilter, EventFlag, FilterFlag, KEvent, Kqueue};
use nix::sys::signal::Signal;
use std::collections::{HashMap, HashSet};
use std::io;
use std::os::fd::RawFd;

const NO_TIME_WAIT: timespec = unsafe { std::mem::zeroed() };

pub struct KqueueWatcher {
    kq: Kqueue,
    sigstore: HashSet<Signal>,
    fdstore: HashMap<RawFd, BufFd>,
}

impl KqueueWatcher {
    pub fn new() -> Self {
        let kq = Kqueue::new().unwrap();
        let sigstore = HashSet::new();
        let fdstore = HashMap::new();
        Self {
            kq,
            sigstore,
            fdstore,
        }
    }

    fn poll_internal(&mut self, block: bool) -> io::Result<Option<Event>> {
        let mut eventlist: [KEvent; 1] = unsafe { std::mem::zeroed() };
        let num_events = self
            .kq
            .kevent(
                &[],
                &mut eventlist,
                if block { None } else { Some(NO_TIME_WAIT) },
            )
            .unwrap();
        if num_events == 0 {
            return Ok(None);
        }
        let ev = eventlist[0];
        if ev.filter().unwrap() == EventFilter::EVFILT_SIGNAL {
            Ok(Some(Event::Signal(Signal::try_from(ev.ident() as i32)?)))
        } else if let Some(buf_fd) = self.fdstore.get_mut(&(ev.ident() as _)) {
            if buf_fd.read(Some(ev.data() as _))? > 0 {
                Ok(Some(Event::File(ev.ident() as _, buf_fd.data())))
            } else {
                Ok(None)
            }
        } else {
            println!("Received an event for an fd not in the store...?");
            panic!();
        }
    }
}

impl AsWatcher for KqueueWatcher {
    fn watch_signal(&mut self, signal: Signal) {
        if self.sigstore.contains(&signal) {
            eprintln!("signal already being watched");
            return;
        }
        let ev = KEvent::new(
            signal as _,
            EventFilter::EVFILT_SIGNAL,
            EventFlag::EV_ADD,
            FilterFlag::empty(),
            0,
            0,
        );
        let changelist = [ev];
        self.kq
            .kevent(&changelist, &mut [], Some(NO_TIME_WAIT))
            .unwrap();
    }

    fn watch_fd(&mut self, fd: RawFd, buffsize: usize) {
        if self.fdstore.contains_key(&fd) {
            eprintln!("fd is already being watched!");
            return;
        }
        let buf_fd = BufFd::new(fd, buffsize);
        self.fdstore.insert(fd, buf_fd);
        let ev = KEvent::new(
            fd as _,
            EventFilter::EVFILT_READ,
            EventFlag::EV_ADD,
            FilterFlag::empty(),
            0,
            0,
        );
        let changelist = [ev];
        self.kq
            .kevent(&changelist, &mut [], Some(NO_TIME_WAIT))
            .unwrap();
    }

    fn poll_block(&mut self) -> io::Result<Option<Event>> {
        self.poll_internal(true)
    }

    fn poll_no_block(&mut self) -> io::Result<Option<Event>> {
        self.poll_internal(false)
    }
}
