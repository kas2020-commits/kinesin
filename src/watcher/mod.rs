mod interface;
pub use interface::{AsWatcher, Event};

#[cfg(not(any(feature = "io-uring", feature = "epoll")))]
mod poll;
#[cfg(not(any(feature = "io-uring", feature = "epoll")))]
pub use poll::PollWatcher as Watcher;

#[cfg(all(feature = "epoll", not(feature = "io-uring"), target_os = "linux"))]
mod epoll;
#[cfg(all(feature = "epoll", not(feature = "io-uring"), target_os = "linux"))]
pub use epoll::EpollWatcher as Watcher;

#[cfg(all(feature = "io-uring", target_os = "linux"))]
mod io_uring;
#[cfg(all(feature = "io-uring", not(feature = "epoll"), target_os = "linux"))]
pub use io_uring::IoUringWatcher as Watcher;

#[cfg(all(
    feature = "kqueue",
    any(target_os = "macos", target_os = "freebsd", target_os = "openbsd")
))]
mod kqueue;
#[cfg(all(
    feature = "kqueue",
    any(target_os = "macos", target_os = "freebsd", target_os = "openbsd")
))]
pub use kqueue::KqueueWatcher as Watcher;
