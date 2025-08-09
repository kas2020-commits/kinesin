//! Register services and manage them through instance lifetimes.
//!
//! This structure provides a mechanism for bringing up services but more
//! importantly for owning services. Services are brought online and teared down
//! through lifetime semantics, so by simply dropping ownership of the service
//! struct, you initiate the kill sequence. This model treats services as
//! resources that get cleaned up through scope, which is extremely handy.
use crate::{conf::ServiceConf, service::Service};
use nix::{
    sys::wait::{waitpid, WaitPidFlag, WaitStatus},
    unistd::Pid,
};
use std::{collections::HashMap, os::fd::RawFd, process::exit};

pub struct Registry {
    service_map: HashMap<String, Service>,
}

impl<'a> IntoIterator for &'a Registry {
    type Item = &'a Service;
    type IntoIter = std::collections::hash_map::Values<'a, String, Service>;
    fn into_iter(self) -> Self::IntoIter {
        self.service_map.values()
    }
}

impl<'a> IntoIterator for &'a mut Registry {
    type Item = &'a mut Service;
    type IntoIter = std::collections::hash_map::ValuesMut<'a, String, Service>;
    fn into_iter(self) -> Self::IntoIter {
        self.service_map.values_mut()
    }
}

impl Registry {
    pub fn new(services: &[ServiceConf]) -> Self {
        let num_services = services.len();
        let mut service_map: HashMap<String, Service> = HashMap::with_capacity(num_services);
        for def in services {
            match Service::new(def) {
                Ok(srvc) => {
                    if let Some(v) = service_map.insert(srvc.name.clone(), srvc) {
                        panic!(
                            "Can't have services with the same key!\nService being replaced: {:#?}",
                            &v
                        );
                    }
                }
                Err(e) => {
                    panic!("Failed to start {}: {:?}", def.name, e);
                }
            }
        }
        Self { service_map }
    }

    pub fn reap_children(&mut self) -> Vec<Service> {
        let mut srvcs = Vec::new();
        loop {
            match waitpid(None, Some(WaitPidFlag::WNOHANG)) {
                Ok(WaitStatus::Exited(pid, status)) => {
                    if status != 0 {
                        eprintln!("Critical Service Failed. Must Terminate...");
                        exit(status);
                    }
                    if let Some(srvc) = self.remove(pid) {
                        srvcs.push(srvc);
                    }
                }
                Ok(WaitStatus::Signaled(pid, _, _)) => {
                    self.remove(pid);
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
        srvcs
    }

    pub fn is_empty(&self) -> bool {
        self.service_map.is_empty()
    }

    pub fn get_by_fd(&self, fd: RawFd) -> Option<&Service> {
        self.service_map.values().find(|&srvc| {
            srvc.stdout.map(|x| fd == x).unwrap_or(false)
                || srvc.stderr.map(|x| fd == x).unwrap_or(false)
        })
    }

    pub fn get_by_fd_mut(&mut self, fd: RawFd) -> Option<&mut Service> {
        self.service_map.values_mut().find(|srvc| {
            srvc.stdout.map(|x| fd == x).unwrap_or(false)
                || srvc.stderr.map(|x| fd == x).unwrap_or(false)
        })
    }

    pub fn remove(&mut self, pid: Pid) -> Option<Service> {
        let mut name: Option<String> = None;
        for srvc in self.service_map.values() {
            if srvc.pid == pid {
                name = Some(srvc.name.clone());
                break;
            }
        }
        if let Some(srvc_name) = name {
            self.service_map.remove(&srvc_name)
        } else {
            None
        }
    }
}
