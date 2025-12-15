//! Defines a service protocol and manages it from start to finish.
//!
//! The service is concerned with everything relating to per-process management.
//! Open file descriptors, environment variables, the process ID, etc are all
//! managed here.
use crate::conf::ServiceConf;
use crate::utils::{set_fd_nonblocking, set_std_stream};
use nix::sys::signal::SigSet;
use nix::{
    errno::Errno,
    libc,
    unistd::{dup2, execve, fork, pipe, ForkResult, Pid},
};
use std::ffi::CString;
use std::os::fd::{AsRawFd, IntoRawFd, RawFd};

#[derive(Debug)]
pub struct Service {
    pub name: String,
    pub pid: Pid,
    pub stdout: RawFd,
    pub stderr: RawFd,
    pub must_be_up: bool,
}

impl Service {
    pub fn new(def: &ServiceConf) -> Result<Self, Errno> {
        let name = def.name.clone();

        let (rout, wout) = pipe()?;
        let (rerr, werr) = pipe()?;

        match unsafe { fork() } {
            Ok(ForkResult::Parent { child: pid }) => {
                drop(wout);
                drop(werr);

                set_fd_nonblocking(rout.as_raw_fd())?;
                set_fd_nonblocking(rerr.as_raw_fd())?;

                Ok(Self {
                    name,
                    pid,
                    stdout: rout.into_raw_fd(),
                    stderr: rerr.into_raw_fd(),
                    must_be_up: def.must_be_up,
                })
            }
            Ok(ForkResult::Child) => {
                drop(rout);
                drop(rerr);

                // remove the blocking of signals for children.
                SigSet::all().thread_unblock().unwrap();

                set_std_stream(wout.as_raw_fd())?;
                set_std_stream(werr.as_raw_fd())?;

                dup2(wout.as_raw_fd(), libc::STDOUT_FILENO).unwrap();
                dup2(werr.as_raw_fd(), libc::STDERR_FILENO).unwrap();

                let mut env_vars = std::env::vars_os()
                    .map(|(k, v)| {
                        CString::new(format!("{}={}", k.to_string_lossy(), v.to_string_lossy()))
                            .unwrap()
                    })
                    .collect::<Vec<_>>();

                for var in &def.env {
                    env_vars.push(var.clone());
                }

                match execve(&def.exec[0], &def.exec, env_vars.as_slice()) {
                    Ok(_) => unreachable!(),
                    Err(e) => Err(e),
                }
            }
            Err(e) => Err(e),
        }
    }
}
