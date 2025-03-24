mod interface;
pub use interface::{AsWatcher, Event};

#[cfg(all(feature = "io-uring", target_os = "linux"))]
mod io_uring;

#[cfg(all(feature = "io-uring", target_os = "linux"))]
pub use io_uring::IoUringDriver as Watcher;

#[cfg(all(not(feature = "io-uring"), target_os = "linux"))]
mod epoll;

#[cfg(all(not(feature = "io-uring"), target_os = "linux"))]
pub use epoll::EpollWatcher as Watcher;

#[cfg(any(target_os = "macos", target_os = "freebsd", target_os = "openbsd"))]
mod kqueue;

#[cfg(any(target_os = "macos", target_os = "freebsd", target_os = "openbsd"))]
pub use kqueue::KqueueWatcher as Watcher;
