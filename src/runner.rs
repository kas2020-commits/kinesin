//! The supervisor provides a framework-like experience for using this codebase.
//! Another benifit of this data structure is that it scopes a bounded generic
//! type, ensuring that the code using the AIO watcher backend is not accidentally
//! tied to a specific implementation.
use std::{collections::HashMap, io, os::fd::RawFd};

use nix::{
    sys::signal::{kill, Signal},
    unistd::{close, sleep},
};

use crate::{
    bus::Bus,
    registry::{Registry, ServiceCompletionResult},
    service::Service,
    watcher::{AsWatcher, Event},
};

pub enum EventResult {
    ServiceCompletion((Service, ServiceCompletionResult)),
}

pub fn handle_event(
    event: Event,
    registry: &mut Registry,
    bus_map: &mut HashMap<RawFd, Bus>,
) -> io::Result<Option<Box<[EventResult]>>> {
    match event {
        Event::Signal(sig) => match sig {
            Signal::SIGCHLD => Ok(Some(
                registry
                    .reap_children()
                    .into_iter()
                    .map(EventResult::ServiceCompletion)
                    .collect(),
            )),
            _ => {
                for srvc in &registry.services {
                    kill(srvc.pid, sig)?;
                }

                Ok(None)
            }
        },
        Event::File(fd, data) => {
            if let Some(bus) = bus_map.get_mut(&fd) {
                bus.consume(data);
            }
            Ok(None)
        }
    }
}

pub fn run<W>(
    mut registry: Registry,
    mut bus_map: HashMap<RawFd, Bus>,
    mut watcher: W,
) -> io::Result<()>
where
    W: AsWatcher,
{
    // main event loop
    'eventloop: while !registry.services.is_empty() {
        if let Ok(Some(event)) = watcher.poll_block() {
            if let Ok(Some(completed_services)) = handle_event(event, &mut registry, &mut bus_map) {
                for event_result in completed_services.iter() {
                    match event_result {
                        EventResult::ServiceCompletion((srvc, result)) => {
                            match result {
                                ServiceCompletionResult::Status(status) => {
                                    println!(
                                        "service {} returned with status {}",
                                        srvc.name, status
                                    )
                                }
                                ServiceCompletionResult::Signal(signal) => {
                                    println!("service {} died from signal {:?}", srvc.name, signal)
                                }
                            }
                            if srvc.must_be_up {
                                println!("Mandatory service dropped. Exiting loop...");
                                break 'eventloop;
                            }
                        }
                    }
                }
            }
        }
    }

    // Any services which haven't naturally died must be shut off
    // This can occur if, for example, a mandatory service dies
    if !registry.services.is_empty() {
        for srvc in &registry.services {
            println!("requesting service {} to gracefully exit", srvc.name);
            if let Err(e) = kill(srvc.pid, Signal::SIGTERM) {
                eprintln!("kill failed with errno {}", e);
            }
        }

        // give each process a bit of time to gracefully shutdown
        sleep(5);

        // collect the gracefully-shutdown services
        for (srvc, result) in registry.reap_children() {
            println!(
                "service {} gracefully shutdown returning {:?}",
                srvc.name, result
            );
        }

        // no more playing nice guy. Activate kill mode!
        for srvc in registry.services {
            if let Err(e) = kill(srvc.pid, Signal::SIGKILL) {
                eprintln!("kill failed with errno {}", e);
            }
            println!("service {} was forcefully killed", srvc.name);
        }
    }

    // flush out the remaining events until no more events exist
    // Since all watched files have been closed, this is a bounded loop.
    while let Ok(Some(event)) = watcher.poll_no_block() {
        match event {
            Event::File(fd, data) => {
                if let Some(bus) = bus_map.get_mut(&fd) {
                    bus.consume(data);
                }
            }
            // skip any remaining signals during shutdown
            _ => {}
        }
        // handle_event(event, &mut registry, &mut bus_map)?;
    }

    // drop the watcher before closing any fds as a safety measure
    drop(watcher);

    // cleanup the file descriptors
    for (fd, _) in bus_map.iter_mut() {
        if let Err(e) = close(*fd) {
            eprintln!("closing file failed with errno {}", e);
        }
    }

    Ok(())
}
