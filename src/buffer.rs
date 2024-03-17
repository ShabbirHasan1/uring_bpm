use crate::page::PageId;
use tokio_uring::buf::fixed::{FixedBuf, FixedBufRegistry};

enum LockState {
    Unloaded,
    Loaded,
    Loading,
    ReadLocked,
    WriteLocked,
}

enum Temperature {
    Cool,
    Hot,
}

enum Swip {
    Id(PageId),
    Ptr(FixedBuf),
}

struct PageHandle {
    state: LockState,
    temperature: Temperature,
    readers: usize,
    id: PageId,
    swip: Option<FixedBuf>,
}

pub struct BufferPool {
    registry: FixedBufRegistry<Vec<u8>>,
}
