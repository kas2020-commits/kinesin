use nix::sys::signal::Signal;
use std::os::fd::RawFd;

pub enum Notification {
    Signal(Signal),
    File(RawFd),
}
