mod interface;
mod notification;

pub use notification::Notification;

pub use interface::SupervisorTrait;

#[cfg(all(feature = "io-uring", target_os = "linux"))]
mod io_uring;

#[cfg(all(feature = "io-uring", target_os = "linux"))]
pub use io_uring::Supervisor;

#[cfg(all(not(feature = "io-uring"), target_os = "linux"))]
mod epoll;

#[cfg(all(not(feature = "io-uring"), target_os = "linux"))]
pub use epoll::Supervisor;

#[cfg(any(target_os = "macos", target_os = "freebsd", target_os = "openbsd"))]
mod kqueue;

#[cfg(any(target_os = "macos", target_os = "freebsd", target_os = "openbsd"))]
pub use kqueue::Supervisor;
