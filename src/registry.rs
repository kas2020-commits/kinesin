//! Register services and manage them through instance lifetimes.
//!
//! This structure provides a mechanism for bringing up services but more
//! importantly for owning services. Services are brought online and teared down
//! through lifetime semantics, so by simply dropping ownership of the service
//! struct, you initiate the kill sequence. This model treats services as
//! resources that get cleaned up through scope, which is extremely handy.
use crate::{conf::ServiceConf, service::Service};
use nix::{
    libc::c_int,
    sys::{
        signal::Signal,
        wait::{waitpid, WaitPidFlag, WaitStatus},
    },
    unistd::Pid,
};

#[derive(Debug)]
pub enum ServiceCompletionResult {
    Status(c_int),
    Signal(Signal),
}

pub struct Registry {
    pub services: Vec<Service>,
}

impl Registry {
    pub fn new(services: &[ServiceConf]) -> Self {
        let num_services = services.len();
        let mut services_ = Vec::with_capacity(num_services);
        for def in services {
            match Service::new(def) {
                Ok(srvc) => {
                    services_.push(srvc);
                }
                Err(e) => {
                    panic!("{:?}", e);
                }
            }
        }

        Self {
            services: services_,
        }
    }

    pub fn reap_children(&mut self) -> Vec<(Service, ServiceCompletionResult)> {
        let mut reaped_children = Vec::new();
        loop {
            match waitpid(None, Some(WaitPidFlag::WNOHANG)) {
                Ok(WaitStatus::Exited(pid, status)) => {
                    if let Some(srvc) = self.remove(pid) {
                        reaped_children.push((srvc, ServiceCompletionResult::Status(status)));
                    }
                }
                Ok(WaitStatus::Signaled(pid, sig, _)) => {
                    if let Some(srvc) = self.remove(pid) {
                        reaped_children.push((srvc, ServiceCompletionResult::Signal(sig)));
                    }
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
        reaped_children
    }

    pub fn get_by_name(&self, name: &str) -> Option<&Service> {
        self.services.iter().find(|&srvc| srvc.name == name)
    }

    pub fn remove(&mut self, pid: Pid) -> Option<Service> {
        if let Some(loc) = self.services.iter().position(|srvc| srvc.pid == pid) {
            Some(self.services.swap_remove(loc))
        } else {
            None
        }
    }
}
