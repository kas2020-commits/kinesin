## Features
- [x] epoll as an AioDriver
- [x] io_uring as an AioDriver
- [x] kqueue as an AioDriver
- [x] one-to-many producer to consumer model bus
- [x] adding environment variables per-service
- [ ] `OnFailure` and `OnSuccess` support allowing tracking
- [ ] defining healthchecks that can be performed periodically
- [ ] restart process with max (total) attempts
- [ ] priority groups to indicate acceptible deaths
- [ ] TCP-based consumer
- [ ] threadpool executors to run consumers on separate threads
- [ ] handle SIGINT and SIGTRM to customize death sequence

## Bugs
- [ ] efficiently honor when config specifies `stdout = false` or `stderr = false`
