#[cfg(all(feature = "io-uring", target_os = "linux"))]
mod io_uring;

#[cfg(all(feature = "io-uring", target_os = "linux"))]
pub use io_uring::Supervisor;

#[cfg(all(not(feature = "io-uring"), target_os = "linux"))]
mod epoll;

#[cfg(all(not(feature = "io-uring"), target_os = "linux"))]
pub use epoll::Supervisor;

#[cfg(all(any(target_os = "macos", target_os = "freebsd", target_os = "openbsd")))]
mod kqueue;

#[cfg(all(any(target_os = "macos", target_os = "freebsd", target_os = "openbsd")))]
pub use kqueue::Supervisor;
