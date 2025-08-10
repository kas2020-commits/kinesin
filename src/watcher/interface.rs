//! Abstraction provided to use multiple kernel-backed notification/event
//! systems.
//!
//! Different operating environments provide different notification systems,
//! each with their own semantics which differ from system to system. This
//! interface provides a restricted usage of said notification systems
//! which allows for abstracting the specific backend away from the consumer.
//! The notable limitations to the underlying backends are as follows:
//!
//! 1) kqueue: changelists and eventlists cannot be combined
//!            into a single syscall. This is because the other implementations
//!            don't support this ability
//! 2) io_uring: notifications can't hold the new data which the kernel surfaced
//!              even though it wouldn't require another syscall. The reasoning
//!              is two-fold. First is that it would require really difficult
//!              lifetime semantics and odd references in the driver
//!              implementation itself. The second, and more important issue
//!              is that io_uring is the only backend which supports this.
use nix::sys::signal::Signal;
use std::io;
use std::os::fd::RawFd;

pub enum Event<'a> {
    Signal(Signal),
    File(RawFd, &'a [u8]),
}

pub trait AsWatcher {
    fn watch_signal(&mut self, signal: Signal);

    fn watch_fd(&mut self, fd: RawFd, buffsize: usize);

    fn poll_block(&mut self) -> io::Result<Option<Event>>;

    fn poll_no_block(&mut self) -> io::Result<Option<Event>>;
}
