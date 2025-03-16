mod cli;
mod conf;
mod events;
mod services;
use crate::conf::get_service_defs;
use crate::events::setup_epoll;
use crate::services::ServiceRegistry;
use nix::{
    errno::Errno,
    fcntl::{fcntl, FcntlArg},
    sys::{
        epoll::{EpollEvent, EpollTimeout},
        signal::Signal,
        signalfd::SignalFd,
        wait::{waitpid, WaitPidFlag, WaitStatus},
    },
    unistd::read,
};
use services::RunningService;
use std::{
    fs::OpenOptions,
    io::{self, Write},
    os::{fd::RawFd, unix::io::AsRawFd},
    process::exit,
};

fn get_pipe_size(fd: RawFd) -> Result<i32, nix::Error> {
    // Get the pipe buffer size using F_GETPIPE_SZ
    fcntl(fd, FcntlArg::F_GETPIPE_SZ)
}

fn flush_pipe(srvc: &RunningService, fd: RawFd) -> io::Result<()> {
    // Proceed with reading and flushing the pipe
    const BUFSIZE: usize = 1024;
    let mut buffer = [0u8; BUFSIZE]; // Read up to 1024 bytes at a time

    // Open the log file in append mode, creating it if it doesn't exist
    let mut log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(format!("{}.log", &srvc.def.name))
        .map_err(|e| {
            eprintln!("Failed to open log file: {}", e);
            e
        })?;

    loop {
        match read(fd, &mut buffer) {
            Ok(0) => {
                break;
            }
            Ok(n) => {
                log_file.write_all(&buffer[..n])?;
            }
            Err(Errno::EAGAIN) => {
                // we don't mind no more data because we already epoll the fd.
                // If we need to read more data we will be called again.
                break;
            }
            Err(e) => {
                return Err(io::Error::new(io::ErrorKind::Other, e));
            }
        }
    }
    Ok(())
}

fn flush_pipe_w_check(srvc: &RunningService, fd: RawFd) -> io::Result<()> {
    // Check the pipe size before attempting to read
    match get_pipe_size(fd) {
        Ok(pipe_size) => {
            // Set a threshold for when to flush based on the pipe size
            if pipe_size > 1024 {
                flush_pipe(srvc, fd)?;
            }
        }
        Err(e) => {
            eprintln!("Failed to get pipe size: {}", e);
        }
    }
    Ok(())
}

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
    event: &EpollEvent,
    signal_fd: &SignalFd,
    registry: &mut ServiceRegistry,
) -> io::Result<()> {
    if event.data() == signal_fd.as_raw_fd() as u64 {
        handle_event_sigchld(signal_fd, registry)?;
    } else if let Some(srvc) = registry.get_srvc_form_stdout(event.data() as i32) {
        let srvc_locked = srvc.lock().unwrap();
        flush_pipe_w_check(&srvc_locked, srvc_locked.stdout.as_raw_fd())?;
    } else if let Some(srvc) = registry.get_srvc_from_stderr(event.data() as i32) {
        let srvc_locked = srvc.lock().unwrap();
        flush_pipe_w_check(&srvc_locked, srvc_locked.stdout.as_raw_fd())?;
    }
    Ok(())
}

fn handle_events(
    num_fds: usize,
    events: &[EpollEvent],
    signal_fd: &SignalFd,
    registry: &mut ServiceRegistry,
) -> io::Result<()> {
    for event in events.iter().take(num_fds) {
        handle_event(event, signal_fd, registry)?;
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
        handle_events(num_fds, &events, &signal_fd, &mut registry)?;
    }
    println!("All services exited, shutting down cleanly.");
    Ok(())
}
