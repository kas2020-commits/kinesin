//! The supervisor provides a framework-like experience for using this codebase.
//! Another benifit of this data structure is that it scopes a bounded generic
//! type, ensuring that the code using the AIO driver backend is not accidentally
//! tied to a specific implementation.
use std::{collections::HashMap, io, os::fd::RawFd};

use nix::sys::signal::Signal;

use crate::{
    aio_driver::{AsDriver, Notification},
    bus::Bus,
    registry::Registry,
};

pub struct Supervisor<T> {
    registry: Registry,
    bus_map: HashMap<RawFd, Bus>,
    driver: T,
}

impl<T> Supervisor<T>
where
    T: AsDriver,
{
    pub fn new(registry: Registry, bus_map: HashMap<RawFd, Bus>, driver: T) -> Self {
        Self {
            registry,
            bus_map,
            driver,
        }
    }

    fn handle_fd(&mut self, fd: i32) -> io::Result<()> {
        let is_proactive = self.driver.is_proactive();
        if let Some(srvc) = self.registry.get_by_fd_mut(fd) {
            let buf_fd = if srvc.stdout.as_raw_fd() == fd {
                &mut srvc.stdout
            } else if srvc.stderr.as_raw_fd() == fd {
                &mut srvc.stderr
            } else {
                unreachable!()
            };
            if is_proactive {
                let res = self.driver.proactive_result().unwrap();
                buf_fd.set_len(res as _);
            } else {
                buf_fd.read()?;
            }
            let dat = buf_fd.data();
            if let Some(bus) = self.bus_map.get_mut(&fd) {
                bus.consume(dat).unwrap();
            }
            if self.driver.is_oneshot() {
                self.driver.register_fd(buf_fd);
            }
        }
        Ok(())
    }

    pub fn run(&mut self) -> io::Result<()> {
        while !self.registry.is_empty() {
            let notif = self.driver.block_next_notif()?;
            match notif {
                Notification::Signal(sig) => match sig {
                    Signal::SIGCHLD => {
                        let _ = self.registry.reap_children();
                    }
                    _ => todo!(),
                },
                Notification::File(fd) => {
                    self.handle_fd(fd)?;
                }
            }
        }
        Ok(())
    }
}
