use crate::frame::Frame;
use tokio_uring::buf::fixed::FixedBufRegistry;

pub struct BufferPool {
    /// The registry of buffers shared between the user and the kernel.
    registry: FixedBufRegistry<Frame>,
}
