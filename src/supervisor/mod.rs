#[cfg(all(feature = "io-uring", target_os = "linux"))]
mod io_uring;

#[cfg(all(feature = "io-uring", target_os = "linux"))]
pub use io_uring::Supervisor;

#[cfg(all(not(feature = "io-uring"), target_os = "linux"))]
mod epoll;

#[cfg(all(not(feature = "io-uring"), target_os = "linux"))]
pub use epoll::Supervisor;
