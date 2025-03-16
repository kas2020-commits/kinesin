use crate::service_def::ServiceDef;
use crate::services::RunningService;
use nix::{
    sys::wait::{waitpid, WaitPidFlag, WaitStatus},
    unistd::Pid,
};
use std::{
    collections::HashMap,
    os::fd::{AsRawFd, RawFd},
    process::exit,
    sync::{Arc, Mutex},
};
type RS = Arc<Mutex<RunningService>>;

pub struct ServiceRegistry {
    pid_map: HashMap<Pid, RS>,
    stdout_map: HashMap<RawFd, RS>,
    stderr_map: HashMap<RawFd, RS>,
}

impl<'a> IntoIterator for &'a ServiceRegistry {
    type Item = &'a RS;
    type IntoIter = std::collections::hash_map::Values<'a, Pid, RS>;
    fn into_iter(self) -> Self::IntoIter {
        self.pid_map.values()
    }
}

impl ServiceRegistry {
    pub fn new(srvcs: &Vec<ServiceDef>) -> Self {
        let cap = srvcs.capacity();
        let mut pid_map: HashMap<Pid, RS> = HashMap::with_capacity(cap);
        let mut stdout_map: HashMap<RawFd, RS> = HashMap::with_capacity(cap);
        let mut stderr_map: HashMap<RawFd, RS> = HashMap::with_capacity(cap);
        // Start all services and map their PIDs for quick lookup
        for def in srvcs {
            match RunningService::new(def) {
                Ok(srvc) => {
                    let dat = Arc::new(Mutex::new(srvc));
                    pid_map.insert(dat.lock().unwrap().pid, dat.clone());
                    stdout_map.insert(dat.lock().unwrap().stdout.as_raw_fd(), dat.clone());
                    stderr_map.insert(dat.lock().unwrap().stderr.as_raw_fd(), dat.clone());
                }
                Err(e) => {
                    eprintln!("Failed to start {}: {:?}", def.conf.name, e);
                }
            }
        }
        Self {
            pid_map,
            stdout_map,
            stderr_map,
        }
    }

    pub fn reap_children(&mut self) {
        loop {
            match waitpid(None, Some(WaitPidFlag::WNOHANG)) {
                Ok(WaitStatus::Exited(pid, status)) => {
                    if status != 0 {
                        eprintln!("Critical Service Failed. Must Terminate...");
                        exit(status);
                    }
                    self.drop(pid);
                }
                Ok(WaitStatus::Signaled(pid, _, _)) => {
                    self.drop(pid);
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

    // pub fn num_services(&self) -> usize {
    //     self.services.len()
    // }

    pub fn is_empty(&self) -> bool {
        self.pid_map.is_empty()
    }

    // pub fn from_pid(&self, pid: Pid) -> Option<RS> {
    //     self.pid_map.get(&pid).cloned()
    // }

    pub fn get_srvc_form_stdout(&self, fd: RawFd) -> Option<RS> {
        self.stdout_map.get(&fd).cloned()
    }

    pub fn get_srvc_from_stderr(&self, fd: RawFd) -> Option<RS> {
        self.stdout_map.get(&fd).cloned()
    }

    pub fn drop(&mut self, pid: Pid) {
        if self.pid_map.contains_key(&pid) {
            {
                let srvc = self.pid_map.get(&pid).unwrap().lock().unwrap();
                self.stdout_map.remove(&srvc.stdout.as_raw_fd());
                self.stderr_map.remove(&srvc.stderr.as_raw_fd());
            }
            self.pid_map.remove(&pid);
        }
    }
}
