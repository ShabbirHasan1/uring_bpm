
# `uring-bpm`

An asynchronous buffer pool manager built on top of the linux `io_uring` interface.



This buffer pool manager is likely going to rely heavily on the `FixedBuf` from `tokio_uring`'s master branch.

The `FixedBuf` is registered in the `io_uring` interface, and we can retrieve it by index.


We want to make sure that the hot path is as uninterrupted as possible





