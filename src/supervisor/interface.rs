use std::io;

// use nix::{errno::Errno, sys::signal::Signal};

use nix::sys::signal::Signal;

use crate::buffd::BufFd;

use super::Notification;

pub trait SupervisorTrait {
    // fn new() -> Self;

    fn is_proactive(&self) -> bool;

    fn proactive_result(&self) -> Option<i32>;

    fn is_oneshot(&self) -> bool;

    fn register_signal(&mut self, signal: Signal);

    fn register_fd(&mut self, buf_fd: &mut BufFd);

    fn block_next_notif(&mut self) -> io::Result<Notification>;
}
