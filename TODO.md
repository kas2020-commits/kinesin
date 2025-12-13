## Features
- [x] epoll as an AioDriver
- [x] io_uring as an AioDriver
- [x] kqueue as an AioDriver
- [x] one-to-many producer to consumer model bus
- [x] adding environment variables per-service
- [x] support mandatory and non-mandatory services
- [x] worker threads consume stream data from bus off main thread
- [x] relay appropriate signals to services
- [x] gracefully terminate remaining processes after mandatory shutdown
- [ ] `OnFailure` and `OnSuccess` support allowing tracking
- [ ] defining healthchecks that can be performed periodically
- [ ] restart process with max (total) attempts
- [ ] TCP-based consumer

## Bugs
- [x] efficiently honor when config specifies `stdout = false` or `stderr = false`
