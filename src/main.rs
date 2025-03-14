mod cli;
mod conf;
mod events;
mod services;
use crate::conf::Config;
use crate::events::setup_fds;
use crate::services::start_service;
use clap::Parser;
use cli::Cli;
use nix::libc;
use nix::sys::signal::Signal;
use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};
use std::collections::HashSet;
use std::os::unix::io::AsRawFd;
use std::{fs, io};

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let config_str = fs::read_to_string(cli.config)?;
    let config: Config = toml::from_str(&config_str).unwrap();

    let mut pids = HashSet::new();

    let (signal_fd, epoll_fd) = setup_fds()?;

    // Start all services and collect PIDs
    for service in &config.services {
        match start_service(service) {
            Ok(pid) => {
                println!("Started '{}' with PID {}", service.name, pid);
                pids.insert(pid);
            }
            Err(err) => eprintln!("Failed to start '{}': {:?}", service.name, err),
        }
    }

    // === Event Loop ===
    while !pids.is_empty() {
        let mut events: [libc::epoll_event; 10] = unsafe { std::mem::zeroed() };
        let nfds = unsafe {
            libc::epoll_wait(
                epoll_fd,
                events.as_mut_ptr(),
                events.len() as i32,
                -1, // Block indefinitely
            )
        };

        if nfds < 0 {
            if nix::errno::Errno::last() == nix::errno::Errno::EINTR {
                continue; // Ignore signals that aren't from epoll
            } else {
                return Err(std::io::Error::last_os_error().into());
            }
        }

        for i in 0..nfds as usize {
            if events[i].u64 == signal_fd.as_raw_fd() as u64 {
                // === Read from signalfd ===
                let info = signal_fd.read_signal()?;
                if let Some(signal) = info {
                    if signal.ssi_signo == Signal::SIGCHLD as u32 {
                        // === Reap child processes ===
                        loop {
                            match waitpid(None, Some(WaitPidFlag::WNOHANG)) {
                                Ok(WaitStatus::Exited(pid, status)) => {
                                    println!("Child {} exited with status {}", pid, status);
                                    pids.remove(&pid);
                                }
                                Ok(WaitStatus::Signaled(pid, sig, _)) => {
                                    println!("Child {} killed by signal {:?}", pid, sig);
                                    pids.remove(&pid);
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
                }
            }
        }
    }

    println!("All services exited, shutting down cleanly.");
    Ok(())
}
