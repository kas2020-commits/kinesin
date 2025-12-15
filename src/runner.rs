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
    registry::{reap_services, Registry, ServiceCompletionResult},
    watcher::{AsWatcher, Event},
};

pub fn cleanup<W>(mut registry: Registry, mut bus_map: HashMap<RawFd, Bus>, mut watcher: W)
where
    W: AsWatcher,
{
    // Any services which haven't naturally died must be shut off
    // This can occur if, for example, a mandatory service dies
    if !registry.is_empty() {
        for srvc in &registry {
            match kill(srvc.pid, Signal::SIGTERM) {
                Ok(_) => {
                    println!("Sent SIGTERM to service {}", srvc.name);
                }
                Err(e) => {
                    eprintln!("kill failed with errno {}", e);
                }
            }
        }

        // poll for graceful shutdown every second to reap services until timeout
        'graceful_shutdown_loop: for _ in 0..5 {
            sleep(1);

            for (srvc, result) in reap_services(&mut registry) {
                println!(
                    "service {} gracefully shutdown returning {:?}",
                    srvc.name, result
                );
            }

            if registry.is_empty() {
                break 'graceful_shutdown_loop;
            }
        }

        // no more playing nice guy. Activate kill mode!
        for srvc in registry {
            match kill(srvc.pid, Signal::SIGKILL) {
                Ok(_) => {
                    println!("Sent SIGKILL to service {}", srvc.name);
                }
                Err(e) => {
                    eprintln!("kill failed with errno {}", e);
                }
            }
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
    }

    // drop the watcher before closing any fds as a safety measure
    drop(watcher);

    // cleanup the file descriptors
    for (fd, _) in bus_map.iter_mut() {
        if let Err(e) = close(*fd) {
            eprintln!("closing file failed with errno {}", e);
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
    'eventloop: while !registry.is_empty() {
        match watcher.poll_block() {
            Ok(Some(event)) => match event {
                Event::Signal(sig) => match sig {
                    // SIGCHLD is meant for us
                    Signal::SIGCHLD => {
                        for (srvc, result) in reap_services(&mut registry).into_iter() {
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

                    // forward all other signals as a courtesy
                    _ => {
                        for srvc in &registry {
                            kill(srvc.pid, sig)?;
                        }
                    }
                },

                // push new data to the stream bus
                Event::File(fd, data) => {
                    if let Some(bus) = bus_map.get_mut(&fd) {
                        bus.consume(data);
                    }
                }
            },

            // ignore the nop
            Ok(None) => {}

            // the watcher hitting an I/O error is real bad. For the sake of
            // correctness it's best to (gracefully) die
            Err(e) => {
                eprintln!("watcher failed with error {}", e);
                break 'eventloop;
            }
        }
    }

    cleanup(registry, bus_map, watcher);

    Ok(())
}
