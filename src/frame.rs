use crate::page::PAGE_SIZE;
use tokio_uring::buf::{IoBuf, IoBufMut};

/// A buffer that is shared between the user and the kernel.
pub struct Frame {
    /// Owned buffer to data.
    /// Eventually, we will want to make sure all buffers point to contiguous memory.
    buf: Box<[u8; PAGE_SIZE]>
}


unsafe impl IoBuf for Frame {
    fn stable_ptr(&self) -> *const u8 {
        self.buf.as_ptr()
    }

    fn bytes_init(&self) -> usize {
        PAGE_SIZE
    }

    fn bytes_total(&self) -> usize {
        PAGE_SIZE
    }
}


unsafe impl IoBufMut for Frame {
    fn stable_mut_ptr(&mut self) -> *mut u8 {
        self.buf.as_mut_ptr()
    }

    unsafe fn set_init(&mut self, pos: usize) {
        panic!("Not allowed to change the size of a page")
    }
}
