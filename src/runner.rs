//! The supervisor provides a framework-like experience for using this codebase.
//! Another benifit of this data structure is that it scopes a bounded generic
//! type, ensuring that the code using the AIO watcher backend is not accidentally
//! tied to a specific implementation.
use std::{collections::HashMap, io, os::fd::RawFd};

use nix::{sys::signal::Signal, unistd::close};

use crate::{
    bus::Bus,
    registry::Registry,
    watcher::{AsWatcher, Event},
};

pub fn handle_event(
    event: Event,
    registry: &mut Registry,
    bus_map: &mut HashMap<RawFd, Bus>,
) -> io::Result<()> {
    match event {
        Event::Signal(sig) => match sig {
            Signal::SIGCHLD => {
                for srvc in registry.reap_children() {
                    // we don't drop the bus here because this signal may have been
                    // caught before the final event on the relevent fds. This flush
                    // is here in case 1 service dies much earlier than other(s)
                    if let Some(stdout) = srvc.stdout {
                        if let Some(bus) = bus_map.get_mut(&stdout) {
                            bus.flush()?;
                        }
                    }
                    if let Some(stderr) = srvc.stderr {
                        if let Some(bus) = bus_map.get_mut(&stderr) {
                            bus.flush()?;
                        }
                    }
                }
            }
            _ => todo!(),
        },
        Event::File(fd, data) => {
            if let Some(bus) = bus_map.get_mut(&fd) {
                bus.consume(data)?;
            }
        }
    }
    Ok(())
}

pub fn run<W>(
    registry: &mut Registry,
    bus_map: &mut HashMap<RawFd, Bus>,
    watcher: &mut W,
) -> io::Result<()>
where
    W: AsWatcher,
{
    // block on events
    while !registry.services.is_empty() {
        if let Some(event) = watcher.poll_block()? {
            handle_event(event, registry, bus_map)?;
        }
    }

    // flush out the remaining events until no more events exist
    while let Some(event) = watcher.poll_no_block()? {
        handle_event(event, registry, bus_map)?;
    }

    // flush the buses
    for (fd, bus) in bus_map.iter_mut() {
        bus.flush()?;
        close(*fd)?;
    }
    Ok(())
}
