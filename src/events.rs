use nix::libc;
use nix::sys::signal::{sigprocmask, SigSet, SigmaskHow, Signal};
use nix::sys::signalfd::SignalFd;
use std::io;
use std::os::unix::io::AsRawFd;

pub fn setup_fds() -> io::Result<(SignalFd, i32)> {
    // === Setup signal handling ===
    // Block SIGCHLD so it doesn't interrupt other syscalls
    let mut sigset = SigSet::empty();
    sigset.add(Signal::SIGCHLD);
    sigprocmask(SigmaskHow::SIG_BLOCK, Some(&sigset), None)?;

    // Create signalfd for SIGCHLD
    let signal_fd = SignalFd::new(&sigset)?;

    // === Setup epoll ===
    let epoll_fd = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
    if epoll_fd < 0 {
        return Err(std::io::Error::last_os_error().into());
    }

    // Register the signal FD with epoll
    let mut event = libc::epoll_event {
        events: libc::EPOLLIN as u32,
        u64: signal_fd.as_raw_fd() as u64,
    };

    if unsafe {
        libc::epoll_ctl(
            epoll_fd,
            libc::EPOLL_CTL_ADD,
            signal_fd.as_raw_fd(),
            &mut event,
        )
    } < 0
    {
        return Err(std::io::Error::last_os_error().into());
    };

    Ok((signal_fd, epoll_fd))
}
