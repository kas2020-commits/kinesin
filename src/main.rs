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
use nix::sys::signal::SigSet;
use std::collections::HashMap;
use std::{fs, io};

fn get_config() -> Config {
    let cli = Cli::parse();

    let config: Config = match cli.config.as_path().extension() {
        Some(ext) => match ext.to_str() {
            Some("toml") => toml::from_str(&fs::read_to_string(&cli.config).unwrap()).unwrap(),
            _ => panic!("File Extension not supported"),
        },
        None => panic!("No extension"),
    };

    config
}

fn main() -> io::Result<()> {
    // We handle signals in the event loop, so block them all from interupting.
    SigSet::all().thread_block().unwrap();

    let config = get_config();

    // initialize our main objects
    let mut registry = Registry::new(&config.service);
    let mut watcher = Watcher::new();
    let mut bus_map = HashMap::new();

    // Register interest in the fds and their associated busses
    for srvc in &mut registry.services {
        if let Some(stdout) = srvc.stdout {
            watcher.watch_fd(stdout, srvc.def.stdout.read_bufsize);
            bus_map.insert(stdout, Bus::new(srvc.def.stdout.bus_bufsize));
        }
        if let Some(stderr) = srvc.stderr {
            watcher.watch_fd(stderr, srvc.def.stderr.read_bufsize);
            bus_map.insert(stderr, Bus::new(srvc.def.stderr.bus_bufsize));
        }
    }

    // register the consumers into the busses
    for consumer_conf in &config.consumer {
        let consumer = match &consumer_conf.kind {
            conf::ConsumerKind::Log(path) => Consumer::File(FileLogger::new(path)?),
            conf::ConsumerKind::StdOut => Consumer::StdOut,
            conf::ConsumerKind::StdErr => Consumer::StdErr,
        };
        let srvc_name = match &consumer_conf.consumes {
            ProducerConf::StdOut(name) => name,
            ProducerConf::StdErr(name) => name,
        };
        let srvc = registry
            .get_by_name(srvc_name.as_str())
            .expect("consumer defined with improper service name");
        let stream_fd = match &consumer_conf.consumes {
            ProducerConf::StdOut(_) => srvc.stdout,
            ProducerConf::StdErr(_) => srvc.stderr,
        }
        .expect("trying to consume a stream that's switched off");
        let bus = bus_map.get_mut(&stream_fd).expect("bus doesn't exist");
        bus.add_consumer(consumer);
    }

    run(&mut registry, &mut bus_map, &mut watcher)?;

    Ok(())
}
