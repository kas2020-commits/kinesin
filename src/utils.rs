use nix::fcntl::{fcntl, FcntlArg, OFlag};
use std::os::fd::RawFd;

pub fn set_fd_nonblocking(fd: RawFd) -> nix::Result<()> {
    let bits = fcntl(fd, FcntlArg::F_GETFL)?;
    let prev_flags = OFlag::from_bits_truncate(bits);
    fcntl(fd, FcntlArg::F_SETFL(prev_flags | OFlag::O_NONBLOCK))?;
    Ok(())
}

pub fn set_std_stream(fd: RawFd) -> nix::Result<()> {
    let bits = fcntl(fd, FcntlArg::F_GETFL)?;
    let prev_flags = OFlag::from_bits_truncate(bits);
    fcntl(
        fd,
        FcntlArg::F_SETFL(prev_flags | OFlag::O_WRONLY | OFlag::O_SYNC),
    )?;
    Ok(())
}
