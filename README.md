
# `uring-bpm`

An asynchronous buffer pool manager built on top of the linux `io_uring` interface.





# TODO

- Convert buffers to `IoSlice`s, which are compatible with `iovec` (it's just a wrapper)
- Figure out how to register buffers
- Figure out how to register file
- Buffer should either be owned by kernel or user, but user should still be able to access it







