use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

use kqueue_sys::{kevent, kqueue, EventFilter, EventFlag, FilterFlag};
use nix::errno::Errno;
use nix::sys::signal::Signal;

use super::AsDriver;

pub struct KqueueDriver {
    kq: OwnedFd,
}

impl KqueueDriver {
    pub fn new() -> Self {
        let kq_fd = unsafe { kqueue() };
        if kq_fd == -1 {
            eprintln!("Couldn't initialize the kqueue: {}", Errno::last());
            panic!();
        }
        let kq = unsafe { std::os::fd::OwnedFd::from_raw_fd(kq_fd) };
        Self { kq }
    }
}

impl AsDriver for KqueueDriver {
    fn is_proactive(&self) -> bool {
        false
    }

    fn proactive_result(&self) -> Option<i32> {
        None
    }

    fn is_oneshot(&self) -> bool {
        false
    }

    fn register_signal(&mut self, signal: Signal) {
        let sigev = kevent::new(
            signal as _,
            EventFilter::EVFILT_SIGNAL,
            EventFlag::EV_ADD, // this is implicitly added anyways
            FilterFlag::empty(),
        );
        let changelist = [sigev];
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

    fn register_fd(&mut self, buf_fd: &mut crate::buffd::BufFd) {
        let event = kevent::new(
            buf_fd.as_raw_fd() as _,
            EventFilter::EVFILT_READ,
            EventFlag::EV_ADD,
            FilterFlag::empty(),
        );
        let changelist = [event];
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

    fn block_next_notif(&mut self) -> io::Result<super::Notification> {
        let mut eventlist: [kevent; 1] = unsafe { std::mem::zeroed() };
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
        if event.filter == EventFilter::EVFILT_SIGNAL {
            Ok(super::Notification::Signal(Signal::try_from(
                event.ident as i32,
            )?))
        } else {
            Ok(super::Notification::File(event.ident as _))
        }
    }
}
