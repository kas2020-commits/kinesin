mod cli;
mod conf;
mod events;
mod services;
use crate::conf::get_service_defs;
use crate::events::setup_epoll;
use crate::services::ServiceRegistry;
use nix::sys::{
    epoll::{EpollEvent, EpollTimeout},
    signal::Signal,
    signalfd::SignalFd,
    wait::{waitpid, WaitPidFlag, WaitStatus},
};
use std::{io, os::unix::io::AsRawFd, process::exit};

fn reap_children(registry: &mut ServiceRegistry) {
    loop {
        match waitpid(None, Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::Exited(pid, status)) => {
                if status != 0 {
                    eprintln!("Critical Service Failed. Must Terminate...");
                    exit(status);
                }
                registry.drop(pid);
            }
            Ok(WaitStatus::Signaled(pid, _, _)) => {
                registry.drop(pid);
            }
            Ok(WaitStatus::StillAlive) => break,
            Err(nix::errno::Errno::ECHILD) => break, // No more children
            Err(e) => {
                eprintln!("Error in waitpid: {:?}", e);
                break;
            }
            _ => {}
        }
    }
}

fn handle_event_sigchld(signal_fd: &SignalFd, registry: &mut ServiceRegistry) -> io::Result<()> {
    if let Some(signal) = signal_fd.read_signal()? {
        if signal.ssi_signo == Signal::SIGCHLD as u32 {
            reap_children(registry);
        }
    }
    Ok(())
}

fn handle_event(
    event: EpollEvent,
    signal_fd: &SignalFd,
    registry: &mut ServiceRegistry,
) -> io::Result<()> {
    if event.data() == signal_fd.as_raw_fd() as u64 {
        handle_event_sigchld(signal_fd, registry)?;
    }
    Ok(())
}

fn handle_events(
    num_fds: usize,
    events: &[EpollEvent],
    signal_fd: &SignalFd,
    registry: &mut ServiceRegistry,
) -> io::Result<()> {
    for i in 0..num_fds as usize {
        handle_event(events[i], signal_fd, registry)?;
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let service_defs = get_service_defs();
    let mut registry = ServiceRegistry::new(&service_defs);
    let (signal_fd, epoll) = setup_epoll()?;
    while !registry.is_empty() {
        let mut events = [EpollEvent::empty(); 10];
        let num_fds = epoll.wait(&mut events, EpollTimeout::NONE)?;
        handle_events(num_fds, &events, &signal_fd, &mut registry)?;
    }
    println!("All services exited, shutting down cleanly.");
    Ok(())
}
