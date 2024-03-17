use crate::page::PageId;
use crate::frame::Frame;
use tokio::sync::RwLock;
use tokio_uring::buf::fixed::{FixedBuf, FixedBufRegistry};

struct PageBuf {
    /// An owned, [`RwLock`]-protected fixed buffer of data.
    /// Represents shared memory between the user and the kernel.
    buf: RwLock<FixedBuf>,
}

pub struct Page {
    /// The ID of the logical page of data.
    pid: PageId,
    /// An optional page buffer. If in the `Some` variant, points directly to a [`FixedBuf`]
    /// protected by a [`RwLock`].
    inner: Option<PageBuf>,
}

pub struct BufferPool {
    /// The registry of buffers shared between the user and the kernel.
    registry: FixedBufRegistry<Frame>,
}
