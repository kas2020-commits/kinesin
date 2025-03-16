mod cli;
mod conf;
mod events;
mod exec_args;
mod logging;
mod service_def;
mod service_registry;
mod services;
mod stdio;
use crate::conf::get_service_defs;
use crate::events::setup_epoll;
use crate::service_registry::ServiceRegistry;
use nix::sys::{
    epoll::{EpollEvent, EpollTimeout},
    signal::Signal,
    signalfd::SignalFd,
};
use std::{io, os::unix::io::AsRawFd};

// fn get_pipe_size(fd: RawFd) -> Result<i32, nix::Error> {
//     // Get the pipe buffer size using F_GETPIPE_SZ
//     fcntl(fd, FcntlArg::F_GETPIPE_SZ)
// }

fn handle_signal_fd(signal_fd: &SignalFd, registry: &mut ServiceRegistry) -> io::Result<()> {
    if let Some(signal) = signal_fd.read_signal()? {
        if signal.ssi_signo == Signal::SIGCHLD as u32 {
            registry.reap_children();
        }
    }
    Ok(())
}

fn handle_epoll_event(
    event: &EpollEvent,
    signal_fd: &SignalFd,
    registry: &mut ServiceRegistry,
) -> io::Result<()> {
    let data = event.data();
    if data == signal_fd.as_raw_fd() as u64 {
        handle_signal_fd(signal_fd, registry)?;
    } else if let Some(srvc) = registry.get_srvc_form_stdout(data as i32) {
        let mut srvc = srvc.lock().unwrap();
        srvc.flush_stdout_pipe()?;
    } else if let Some(srvc) = registry.get_srvc_from_stderr(data as i32) {
        let mut srvc = srvc.lock().unwrap();
        srvc.flush_stderr_pipe()?;
    }
    Ok(())
}

fn handle_epoll_events(
    num_fds: usize,
    events: &[EpollEvent],
    signal_fd: &SignalFd,
    registry: &mut ServiceRegistry,
) -> io::Result<()> {
    for event in events.iter().take(num_fds) {
        handle_epoll_event(event, signal_fd, registry)?;
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let service_defs = get_service_defs();
    let mut registry = ServiceRegistry::new(&service_defs);
    let (signal_fd, epoll) = setup_epoll(&registry)?;
    while !registry.is_empty() {
        let mut events = [EpollEvent::empty(); 10];
        let num_fds = epoll.wait(&mut events, EpollTimeout::NONE)?;
        handle_epoll_events(num_fds, &events, &signal_fd, &mut registry)?;
    }
    println!("All services exited, shutting down cleanly.");
    Ok(())
}
