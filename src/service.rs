use crate::conf::ServiceConf;
use crate::exec::execv;
use crate::logging::{FileLogHandler, LogHandler};
use crate::stdio::StdIo;
use nix::{
    errno::Errno,
    fcntl::{fcntl, FcntlArg, OFlag},
    libc,
    unistd::{fork, pipe, ForkResult, Pid},
};
use std::os::fd::{AsRawFd, IntoRawFd, RawFd};

fn set_fd_nonblocking(fd: RawFd) -> nix::Result<()> {
    let bits = fcntl(fd, FcntlArg::F_GETFL)?;
    let prev_flags = OFlag::from_bits_truncate(bits);
    fcntl(fd, FcntlArg::F_SETFL(prev_flags | OFlag::O_NONBLOCK))?;
    Ok(())
}

pub struct Service {
    pub name: String,
    pub pid: Pid,
    pub stdout: StdIo,
    pub stderr: StdIo,
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
                let stdout_file_logger = LogHandler::File(
                    FileLogHandler::new(format!("{}_stdout.log", def.name)).unwrap(),
                );
                let stderr_file_logger = LogHandler::File(
                    FileLogHandler::new(format!("{}_stderr.log", def.name)).unwrap(),
                );
                let stdout = StdIo::new(stdout_read, vec![stdout_file_logger]);
                let stderr = StdIo::new(stderr_read, vec![stderr_file_logger]);
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

impl Drop for Service {
    fn drop(&mut self) {
        match self.stdout.read() {
            Ok(_) => (),
            Err(e) => {
                eprintln!("Flushing pipe failed: {}", e);
            }
        }
        match self.stderr.read() {
            Ok(_) => (),
            Err(e) => {
                eprintln!("Flushing pipe failed: {}", e);
            }
        }
    }
}
