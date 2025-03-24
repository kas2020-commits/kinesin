//! Defines a service protocol and manages it from start to finish.
//!
//! The service is concerned with everything relating to per-process management.
//! Open file descriptors, environment variables, the process ID, etc are all
//! managed here.
use crate::conf::ServiceConf;
use crate::utils::{set_fd_nonblocking, set_std_stream};
use nix::{
    errno::Errno,
    fcntl::{open, OFlag},
    libc,
    unistd::{close, dup2, execve, fork, pipe, ForkResult, Pid},
};
use std::ffi::CString;
use std::os::fd::{IntoRawFd, RawFd};

const DEVNULL: &str = "/dev/null";

#[derive(Debug)]
pub struct Service {
    pub name: String,
    pub pid: Pid,
    pub stdout: Option<RawFd>,
    pub stderr: Option<RawFd>,
}

impl Service {
    pub fn new(def: &ServiceConf) -> Result<Self, Errno> {
        let name = def.name.clone();

        let (rout_owned, wout_owned) = pipe()?;
        let (rerr_owned, werr_owned) = pipe()?;

        // println!(
        //     "opened {} (ro), {} (wo), {} (re) and {} (we)",
        //     rout_owned.as_raw_fd(),
        //     wout_owned.as_raw_fd(),
        //     rerr_owned.as_raw_fd(),
        //     werr_owned.as_raw_fd()
        // );

        let stdout = rout_owned.into_raw_fd();
        let stderr = rerr_owned.into_raw_fd();

        let wout = if def.stdout {
            wout_owned.into_raw_fd()
        } else {
            // println!("closing {} (wo) pipe", wout_owned.as_raw_fd());
            close(wout_owned.into_raw_fd()).unwrap();
            open(DEVNULL, OFlag::O_WRONLY, nix::sys::stat::Mode::empty()).unwrap()
        };

        let werr = if def.stderr {
            werr_owned.into_raw_fd()
        } else {
            // println!("closing {} (we) pipe", werr_owned.as_raw_fd());
            close(werr_owned.into_raw_fd()).unwrap();
            open(DEVNULL, OFlag::O_WRONLY, nix::sys::stat::Mode::empty()).unwrap()
        };

        match unsafe { fork() } {
            Ok(ForkResult::Parent { child: pid }) => {
                // println!("closing {} (wo) pipe/devnull [mandatory]", wout);
                // println!("closing {} (we) pipe/devnull [mandatory]", werr);
                close(wout).unwrap();
                close(werr).unwrap();

                if def.stdout {
                    set_fd_nonblocking(stdout)?;
                } else {
                    close(stdout).unwrap();
                }

                if def.stderr {
                    set_fd_nonblocking(stderr)?;
                } else {
                    close(stderr).unwrap();
                }

                Ok(Self {
                    name,
                    pid,
                    stdout: if def.stdout { Some(stdout) } else { None },
                    stderr: if def.stderr { Some(stderr) } else { None },
                })
            }
            Ok(ForkResult::Child) => {
                set_std_stream(wout)?;
                set_std_stream(werr)?;
                dup2(wout, libc::STDOUT_FILENO).unwrap();
                dup2(werr, libc::STDERR_FILENO).unwrap();
                close(stdout).unwrap();
                close(stderr).unwrap();
                let mut env_vars = std::env::vars_os()
                    .map(|(k, v)| {
                        CString::new(format!("{}={}", k.to_string_lossy(), v.to_string_lossy()))
                            .unwrap()
                    })
                    .collect::<Vec<_>>();

                for var in &def.env {
                    env_vars.push(var.clone());
                }

                execve(&def.exec[0], &def.exec, env_vars.as_slice()).unwrap();
                unreachable!()
            }
            Err(e) => Err(e),
        }
    }
}
