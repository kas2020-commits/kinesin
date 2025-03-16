use crate::logging::{FileLogHandler, LogHandler};
use crate::service_def::ServiceDef;
use crate::stdio::StdIoBuf;
use nix::{
    errno::Errno,
    fcntl::{fcntl, FcntlArg, OFlag},
    libc,
    unistd::{fork, pipe, read, ForkResult, Pid},
};
use std::io;
use std::os::fd::{AsRawFd, IntoRawFd, OwnedFd, RawFd};

pub struct RunningService {
    pub def: ServiceDef,
    pub pid: Pid,
    pub stdout: OwnedFd,
    pub stderr: OwnedFd,
    pub stdout_buf: StdIoBuf,
    pub stderr_buf: StdIoBuf,
}

fn set_fd_nonblocking(fd: RawFd) -> nix::Result<()> {
    let bits = fcntl(fd, FcntlArg::F_GETFL)?;
    let prev_flags = OFlag::from_bits_truncate(bits);
    fcntl(fd, FcntlArg::F_SETFL(prev_flags | OFlag::O_NONBLOCK))?;
    Ok(())
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
                set_fd_nonblocking(stdout_read.as_raw_fd())?;
                set_fd_nonblocking(stderr_read.as_raw_fd())?;
                let stdout_file_logger = LogHandler::File(
                    FileLogHandler::new(format!("{}_stdout.log", def.name)).unwrap(),
                );
                let stderr_file_logger = LogHandler::File(
                    FileLogHandler::new(format!("{}_stderr.log", def.name)).unwrap(),
                );
                let stdout_buf = StdIoBuf::new(vec![stdout_file_logger]);
                let stderr_buf = StdIoBuf::new(vec![stderr_file_logger]);
                Ok(Self {
                    def: def.clone(),
                    pid,
                    stdout: stdout_read,
                    stderr: stderr_read,
                    stdout_buf,
                    stderr_buf,
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

    fn flush_stdio_pipe(&mut self, fd: RawFd) -> io::Result<()> {
        // Proceed with reading and flushing the pipe
        const BUFSIZE: usize = 1024;
        let mut buffer = [0u8; BUFSIZE]; // Read up to 1024 bytes at a time

        // pick the right buffer based on the fd
        let stdio_buf = match fd {
            val if val == self.stdout.as_raw_fd() => &mut self.stdout_buf,
            val if val == self.stderr.as_raw_fd() => &mut self.stderr_buf,
            _ => unreachable!(),
        };

        loop {
            match read(fd, &mut buffer) {
                Ok(0) => {
                    break;
                }
                Ok(n) => {
                    stdio_buf.write(&buffer[..n])?;
                }
                Err(Errno::EAGAIN) => {
                    // we don't mind no more data because we already epoll the fd.
                    // If we need to read more data we will be called again.
                    break;
                }
                Err(e) => {
                    return Err(io::Error::new(io::ErrorKind::Other, e));
                }
            }
        }
        Ok(())
    }

    pub fn flush_stdout_pipe(&mut self) -> io::Result<()> {
        self.flush_stdio_pipe(self.stdout.as_raw_fd())
    }

    pub fn flush_stderr_pipe(&mut self) -> io::Result<()> {
        self.flush_stdio_pipe(self.stderr.as_raw_fd())
    }
}

impl Drop for RunningService {
    fn drop(&mut self) {
        todo!()
    }
}
