//! Defines a service protocol and manages it from start to finish.
//!
//! The service is concerned with everything relating to per-process management.
//! Open file descriptors, environment variables, the process ID, etc are all
//! managed here.
use crate::buffd::BufFd;
use crate::conf::ServiceConf;
use crate::exec::execv;
use crate::utils::set_fd_nonblocking;
use nix::{
    errno::Errno,
    libc,
    unistd::{fork, pipe, ForkResult, Pid},
};
use std::os::fd::{AsRawFd, IntoRawFd};

#[derive(Debug)]
pub struct Service {
    pub name: String,
    pub pid: Pid,
    pub stdout: BufFd,
    pub stderr: BufFd,
}

impl Service {
    pub fn new(def: &ServiceConf) -> Result<Self, Errno> {
        let name = def.name.clone();
        let (stdout_read, stdout_write) = pipe()?;
        let (stderr_read, stderr_write) = pipe()?;
        match unsafe { fork() } {
            Ok(ForkResult::Parent { child: pid }) => {
                unsafe {
                    libc::close(stdout_write.into_raw_fd());
                    libc::close(stderr_write.into_raw_fd());
                }
                set_fd_nonblocking(stdout_read.as_raw_fd())?;
                set_fd_nonblocking(stderr_read.as_raw_fd())?;
                let stdout = BufFd::new(stdout_read);
                let stderr = BufFd::new(stderr_read);
                Ok(Self {
                    name,
                    pid,
                    stdout,
                    stderr,
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
                match execv(&def.exec) {
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
