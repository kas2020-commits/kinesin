mod buffd;
mod bus;
mod cli;
mod conf;
mod consumer;
mod registry;
mod runner;
mod service;
mod utils;
mod watcher;
use crate::bus::Bus;
use crate::cli::Cli;
use crate::conf::{Config, ProducerConf};
use crate::consumer::{Consumer, FileLogger};
use crate::registry::Registry;
use crate::runner::run;
use crate::watcher::{AsWatcher, Watcher};
use clap::Parser;
use nix::sys::signal::{SigSet, Signal};
use std::collections::HashMap;
use std::{fs, io};

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    let config: Config = match cli.config.as_path().extension() {
        Some(ext) => match ext.to_str() {
            Some("toml") => toml::from_str(&fs::read_to_string(&cli.config).unwrap()).unwrap(),
            _ => panic!("File Extension not supported"),
        },
        None => panic!("No extension"),
    };

    // This makes the thread no longer get interupted for signals in our
    // sigset, preventing split-brain issues by letting us respond to the
    // signal as a notification instead of as a special case.
    let mut mask = SigSet::empty();
    mask.add(Signal::SIGCHLD);
    mask.thread_block().unwrap();

    // println!("{:#?}", config);
    // println!("{}", getpid());

    // initialize our main objects
    let mut registry = Registry::new(&config.service);
    let mut watcher = Watcher::new();
    let mut bus_map = HashMap::new();

    // We want to be notified on SIGCHLD
    // NOTE: This isn't a user-configurable thing since it's part of PID 1's
    // responsibilities in this scope
    watcher.watch_signal(Signal::SIGCHLD);

    // We register the file descriptors
    for srvc in &mut registry {
        // println!("{}, {}", srvc.stdout.as_raw_fd(), srvc.stderr.as_raw_fd());
        if let Some(stdout) = srvc.stdout {
            watcher.watch_fd(stdout);
        }
        if let Some(stderr) = srvc.stderr {
            watcher.watch_fd(stderr);
        }

        let mut stdout_consumers = Vec::new();
        let mut stderr_consumers = Vec::new();

        if let Some(consumer) = config.consumer.iter().find(|c| match c.consumes.clone() {
            ProducerConf::StdOut(name) => name == srvc.name,
            ProducerConf::StdErr(name) => name == srvc.name,
        }) {
            match consumer.consumes {
                ProducerConf::StdOut(_) => match &consumer.kind {
                    conf::ConsumerKind::Log(path) => {
                        stdout_consumers.push(Consumer::File(FileLogger::new(path)?));
                    }
                    conf::ConsumerKind::StdOut => {
                        stdout_consumers.push(Consumer::StdOut);
                    }
                    conf::ConsumerKind::StdErr => {
                        stdout_consumers.push(Consumer::StdErr);
                    }
                },
                ProducerConf::StdErr(_) => match &consumer.kind {
                    conf::ConsumerKind::Log(path) => {
                        stderr_consumers.push(Consumer::File(FileLogger::new(path)?));
                    }
                    conf::ConsumerKind::StdOut => {
                        stdout_consumers.push(Consumer::StdOut);
                    }
                    conf::ConsumerKind::StdErr => {
                        stdout_consumers.push(Consumer::StdErr);
                    }
                },
            };
        };

        if !stdout_consumers.is_empty() {
            let stdout_bus = Bus::new(stdout_consumers);

            if let Some(stdout) = srvc.stdout {
                bus_map.insert(stdout, stdout_bus);
            }
        }

        if !stderr_consumers.is_empty() {
            let stderr_bus = Bus::new(stderr_consumers);
            if let Some(stderr) = srvc.stderr {
                bus_map.insert(stderr, stderr_bus);
            }
        }
    }

    run(&mut registry, &mut bus_map, &mut watcher)?;

    Ok(())
}
