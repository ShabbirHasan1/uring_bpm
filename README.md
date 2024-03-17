# `uring-bpm`

An asynchronous buffer pool manager built on top of the linux `io_uring` interface.

# References

-   [`tokio-uring`](https://github.com/tokio-rs/tokio-uring)
-   [The German's papers](https://www.cs.cit.tum.de/dis/research/leanstore/)
-   [NVMe paper](https://www.vldb.org/pvldb/vol16/p2090-haas.pdf) from the Germans
-   [Evolution of LeanStore](https://dl.gi.de/server/api/core/bitstreams/edd344ab-d765-4454-9dbe-fcfa25c8059c/content)
-   [WATT replacement](https://www.vldb.org/pvldb/vol16/p3323-vohringer.pdf)

# Proposal

Mostly inspired from the NVMe paper.

## Foreground vs Background threads

We will want to avoid background worker threads managing the sync between the buffer pool and the disk,
as this wastes CPU cores and CPU cycles.

Historically, the point of these background worker threads was to asynchronously write things out to disk,
since foreground threads were all synchronous and blocking.

However, since we want to design a fully asynchronous-exposed buffer pool manager, we want the
foreground threads to do the work.

Also, from the NVMe paper:

> Another advantage is that worker threads do not stall waiting for free pages
> due to page providers being too slow.
> Instead, workers naturally start spending more CPU time on eviction if the
> system starts running out of free pages.


## Hybrid Latches

We will want to use a hybrid latch, which supports 3 modes:
Optimistic Read, Pessimistic Read, and Write.
Pessimistic Read and Write are exactly the same as a RwLock
(though it is asynchronous, like `tokio::sync::Rwlock`).

The third mode attempts to read via a closure without grabbing any lock,
and at the end of the read, it will check if anyone has changed the underlying data,
and if so it will restart the read operation / closure.

## Page Replacement Strategy

Since the worker threads are now the ones in charge of evicting pages,
page eviction must now be a distributed and decentralized algorithm.

This immediately rules out any form or FIFO or LRU eviction algorithms,
as all of those algorithms require 1 thread making a decision on a page to evict.
On top of this, we want to parallelize this as much as possible, and 1 global
point of contention will completely remove any parallelism.

We will use a version of the second chance algorithm, as it does not require
any global contention points, and instead relies on random sampling.

If a thread needs a page, and all pages are currently occupied, then it needs to choose
a page (or multiple pages) to evict.

The algorithm runs as follows (inspired by LeanStore Second Chance implementation):

1. Pick a set of 64 random buffer frames and load their status
    - Each frame is either pinned or unpinned (via the number of readers)
    - Each frame is either "hot" or "cool"
2. Grab a hybrid optimistic read lock for every buffer frame
3. Any frame that is "hot" gets atomically changed to "cool" (through a CAS)
    - If the CAS fails, then it is in the "cool" state, and we fallback to step 4
4. If a thread sees a frame that is "cool", it will want to evict it
    - First, we take exclusive write lock by _waiting_ for pessimistic readers to be done with it
        - As long as the inner `RwLock` is fair, optimistic readers will wait
        - TODO should readers be able to preempt this from happening
    - If the "cool" frame was dirty, then we flush to disk first before evicting
    - We bring in the data that we want from disk
5. (Optional) we can continue evicting frames that we see are "cool", but without bringing in data from disk

Separately, whenever a frame is accessed with a read lock, it gets atomically changed to the "hot" state.

There will probably be concurrency bugs in an implementation,
so we will have to make sure this is provably correct.

Regardless, this is a simpler version of the WATT page replacement algorithm,
which means we could implement WATT in a future version of this buffer manager.

---

For now, since we're only dealing with read-only data,
we just need to expose a single function, `read()`. It can give back a
`ReadPageGuard` that prevents writes from happening on the page.

As stated in the previous section, page eviction can just be done with a write.

## Nuts and Bolts

This buffer pool manager is likely going to rely heavily on the `FixedBuf` from `tokio_uring`'s master branch.

The `FixedBuf` is registered in the `io_uring` interface, and we can retrieve it by index.
Each of these `FixedBuf` are essentially static, in that we never release the memory.

So instead of "dropping" these pages when we need to write these out to disk,
and then returning a page handle to

We want to make sure that the hot path is as uninterrupted as possible

For now, assume 1 SSD, so we will have 1 `io_uring` instance for simplicity


# Implications for `eggstrain`

We will want to extend `tokio`'s runtime to do following event loop (also from the NVMe paper):

1. `run_task()`
2. `submit_io()`
3. `eviction()`
4. `poll_io()`

We could modify `tokio`'s runtime directly for the absolute best performance,
but that would be hard and also disable our ability to leverage future improvements to `tokio` in the future.

So we can just make sure that every operation we do here somewhat follows the above steps.


