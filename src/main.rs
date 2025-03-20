mod buffd;
mod bus;
mod cli;
mod conf;
mod exec;
mod logging;
mod registry;
mod service;
mod supervisor;
mod utils;
use crate::cli::Cli;
use crate::logging::{FileLogHandler, LogHandler};
use crate::registry::Registry;
use crate::supervisor::{Notification, Supervisor, SupervisorTrait};
use bus::Bus;
use clap::Parser;
use conf::Config;
use nix::sys::signal::{SigSet, Signal};
use std::collections::HashMap;
use std::os::fd::RawFd;
use std::{fs, io};

fn run<T: SupervisorTrait>(
    registry: &mut Registry,
    bus_map: &mut HashMap<RawFd, Bus>,
    supervisor: &mut T,
) -> io::Result<()> {
    let is_proactive = supervisor.is_proactive();
    while !registry.is_empty() {
        let notif = supervisor.block_next_notif()?;
        match notif {
            Notification::Signal(sig) => match sig {
                Signal::SIGCHLD => {
                    let _ = registry.reap_children();
                }

                _ => todo!(),
            },
            Notification::File(fd) => {
                if let Some(srvc) = registry.get_by_fd_mut(fd) {
                    let buf_fd = if srvc.stdout.as_raw_fd() == fd {
                        &mut srvc.stdout
                    } else if srvc.stderr.as_raw_fd() == fd {
                        &mut srvc.stderr
                    } else {
                        unreachable!()
                    };
                    if is_proactive {
                        let res = supervisor.proactive_result().unwrap();
                        buf_fd.set_len(res as _);
                    } else {
                        buf_fd.read()?;
                    }
                    let dat = buf_fd.data();
                    if let Some(bus) = bus_map.get_mut(&fd) {
                        bus.consume(dat).unwrap();
                    }
                    if supervisor.is_oneshot() {
                        supervisor.register_fd(buf_fd);
                    }
                }
            }
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let config: Config = match cli.config.as_path().extension() {
        Some(ext) => match ext.to_str() {
            Some("toml") => toml::from_str(&fs::read_to_string(&cli.config).unwrap()).unwrap(),
            _ => panic!("not supported"),
        },
        None => panic!("No extension"),
    };

    // This makes the thread no longer get interupted for signals in our
    // sigset, preventing split-brain issues by letting us respond to the
    // signal as a notification instead of as a special case.
    let mut mask = SigSet::empty();
    mask.add(Signal::SIGCHLD);
    mask.thread_block().unwrap();

    println!("{:#?}", config);

    // initialize our main objects
    let mut registry = Registry::new(&config.service);
    let mut supervisor = Supervisor::new();
    let mut bus_map = HashMap::new();

    // We want to be notified on SIGCHLD
    supervisor.register_signal(Signal::SIGCHLD);

    for srvc in &mut registry {
        supervisor.register_fd(&mut srvc.stdout);
        supervisor.register_fd(&mut srvc.stderr);

        let stdout_loghandler =
            LogHandler::File(FileLogHandler::new(format!("{}.log", srvc.name))?);
        let stdout_bus = Bus::new(vec![stdout_loghandler]);
        bus_map.insert(srvc.stdout.as_raw_fd(), stdout_bus);

        let stderr_bus = Bus::new(vec![]);
        bus_map.insert(srvc.stderr.as_raw_fd(), stderr_bus);
    }

    run(&mut registry, &mut bus_map, &mut supervisor)?;

    Ok(())
}
