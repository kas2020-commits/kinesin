//! Register services and manage them through instance lifetimes.
//!
//! This structure provides a mechanism for bringing up services but more
//! importantly for owning services. Services are brought online and teared down
//! through lifetime semantics, so by simply dropping ownership of the service
//! struct, you initiate the kill sequence. This model treats services as
//! resources that get cleaned up through scope, which is extremely handy.
use crate::service::Service;
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

pub type Registry = Vec<Service>;

fn remove_by_pid(reg: &mut Registry, pid: Pid) -> Option<Service> {
    if let Some(loc) = reg.iter().position(|srvc| srvc.pid == pid) {
        Some(reg.swap_remove(loc))
    } else {
        None
    }
}

pub fn reap_services(reg: &mut Registry) -> Vec<(Service, ServiceCompletionResult)> {
    let mut reaped_children = Vec::new();
    'reaploop: loop {
        match waitpid(None, Some(WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::Exited(pid, status)) => {
                if let Some(srvc) = remove_by_pid(reg, pid) {
                    reaped_children.push((srvc, ServiceCompletionResult::Status(status)));
                }
            }
            Ok(WaitStatus::Signaled(pid, sig, _)) => {
                if let Some(srvc) = remove_by_pid(reg, pid) {
                    reaped_children.push((srvc, ServiceCompletionResult::Signal(sig)));
                }
            }
            Ok(WaitStatus::StillAlive) => break 'reaploop,
            Err(nix::errno::Errno::ECHILD) => break 'reaploop, // No more children
            Err(e) => {
                eprintln!("Error in waitpid: {:?}", e);
                break 'reaploop;
            }
            _ => {}
        }
    }
    reaped_children
}
