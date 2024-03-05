use tokio_uring::buf::{IoBuf, IoBufMut};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PageId {
    pub id: usize,
}

pub const PAGE_SIZE: usize = 1 << 12; // 4096

#[derive(Clone)]
pub struct Page {
    pub buf: Box<[u8; PAGE_SIZE]>,
}

impl Default for Page {
    fn default() -> Self {
        Self {
            buf: Box::new([0; PAGE_SIZE]),
        }
    }
}

unsafe impl IoBuf for Page {
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

unsafe impl IoBufMut for Page {
    fn stable_mut_ptr(&mut self) -> *mut u8 {
        self.buf.as_mut_ptr()
    }

    unsafe fn set_init(&mut self, pos: usize) {
        assert!(
            pos <= PAGE_SIZE,
            "Frames have a maximum size of {PAGE_SIZE}, tried to make it {pos}"
        );
    }
}
