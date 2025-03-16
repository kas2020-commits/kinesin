use crate::{conf::ServiceConf, flush_pipe};
use nix::{
    errno::Errno,
    libc,
    unistd::{fork, pipe, ForkResult, Pid},
};
use std::{
    collections::HashMap,
    ffi::CString,
    os::fd::{AsRawFd, IntoRawFd, OwnedFd, RawFd},
    sync::{Arc, Mutex},
};
use which::which;

#[derive(Clone, Debug)]
pub struct ExecArgs {
    pub args: Vec<CString>,
    pub argc: CString,
}

impl ExecArgs {
    pub fn new(service: &ServiceConf) -> Self {
        let self_path = which(&service.cmd).unwrap();

        let argc = CString::new(
            self_path
                .to_str()
                .expect("Path provided can't be turned into a string"),
        )
        .expect("Can't convert to CString");

        let argv0 = std::path::Path::new(&self_path)
            .file_name()
            .expect("Failed to get program name")
            .to_str()
            .expect("Program name not valid UTF-8");

        // Convert args into CStrings
        let mut args: Vec<CString> = service
            .args
            .clone()
            .into_iter()
            .map(|s| CString::new(s).expect("Failed to create CString"))
            .collect();

        args.insert(0, CString::new(argv0).expect("Failed to create CString"));

        Self { argc, args }
    }

    fn to_argv(&self) -> Vec<*const libc::c_char> {
        let mut argv: Vec<*const libc::c_char> = self.args.iter().map(|s| s.as_ptr()).collect();

        argv.push(std::ptr::null());
        argv
    }
}

#[derive(Clone)]
pub struct ServiceDef {
    pub name: String,
    pub conf: ServiceConf,
    pub args: ExecArgs,
}

impl ServiceDef {
    pub fn new(conf: &ServiceConf) -> Self {
        Self {
            name: conf.name.clone(),
            conf: conf.clone(),
            args: ExecArgs::new(conf),
        }
    }
}

pub struct RunningService {
    pub def: ServiceDef,
    pub pid: Pid,
    pub stdout: OwnedFd,
    pub stderr: OwnedFd,
}

impl RunningService {
    pub fn new(def: &ServiceDef) -> Result<Self, Errno> {
        let (stdout_read, stdout_write) = pipe()?;
        let (stderr_read, stderr_write) = pipe()?;
        match unsafe { fork() } {
            Ok(ForkResult::Parent { child: pid }) => {
                unsafe {
                    libc::close(stdout_write.into_raw_fd());
                    libc::close(stderr_write.into_raw_fd());
                }
                Ok(Self {
                    def: def.clone(),
                    pid,
                    stdout: stdout_read,
                    stderr: stderr_read,
                })
            }
            Ok(ForkResult::Child) => {
                // Redirect stdout and stderr to the write ends of the pipes
                unsafe {
                    libc::dup2(stdout_write.into_raw_fd(), libc::STDOUT_FILENO);
                    libc::dup2(stderr_write.into_raw_fd(), libc::STDERR_FILENO);
                    libc::close(stdout_read.into_raw_fd());
                    libc::close(stderr_read.into_raw_fd());
                }

                let prog = def.args.argc.as_ptr();
                let argv = def.args.to_argv();

                match unsafe { libc::execv(prog, argv.as_ptr()) } {
                    -1 => {
                        eprintln!("execv errored");
                        std::process::exit(1);
                    }
                    _ => {
                        unreachable!();
                    }
                }
            }
            Err(e) => Err(e),
        }
    }
}

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
        if let Some(arc_srvc) = self.pid_map.get(&pid) {
            let srvc = arc_srvc.lock().unwrap();
            flush_pipe(&srvc, srvc.stdout.as_raw_fd()).unwrap();
            flush_pipe(&srvc, srvc.stderr.as_raw_fd()).unwrap();
            self.stdout_map.remove(&srvc.stdout.as_raw_fd());
            self.stderr_map.remove(&srvc.stderr.as_raw_fd());
        }
        self.pid_map.remove(&pid);
    }
}
