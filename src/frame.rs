use crate::page::PAGE_SIZE;
use std::{
    mem::ManuallyDrop,
    ops::{Deref, DerefMut, RangeBounds, RangeFull},
};
use tokio_uring::buf::{fixed::FixedBuf, BoundedBuf, BoundedBufMut, IoBuf, IoBufMut, Slice};

pub struct Frame {
    buf: ManuallyDrop<FixedBuf>,
}

impl Deref for Frame {
    type Target = FixedBuf;

    fn deref(&self) -> &Self::Target {
        self.buf.deref()
    }
}

impl DerefMut for Frame {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.buf.deref_mut()
    }
}

/// Safety: The only shared mutable state in the [`FixedBuf`] is the `registry` field inside of it,
/// which is a `Rc<RefCell<dyn FixedBuffers>>`.
/// Since the only method on [`FixedBuf`] that accesses this field is the `drop` implementation, as
/// long as we ensure that we do not drop the [`FixedBuf`] before the end of the program, no state
/// can be shared with any other object, as we own the [`FixedBuf`]. Thus, it is perfectly safe to
/// share this among threads.
unsafe impl Send for Frame {}

/// Safety: The only shared mutable state in the [`FixedBuf`] is the `registry` field inside of it,
/// which is a `Rc<RefCell<dyn FixedBuffers>>`.
/// Since [`FixedBuf`] does not use any interior mutability besides when it is dropped, and all
/// mutations must be performed through an exclusive reference to a [`FixedBuf`], this means that
/// it suffices for [`Frame`] to be `Sync`.
unsafe impl Sync for Frame {}

impl BoundedBuf for Frame {
    type Buf = FixedBuf;

    type Bounds = RangeFull;

    fn slice(self, range: impl RangeBounds<usize>) -> Slice<Self::Buf> {
        ManuallyDrop::into_inner(self.buf).slice(range)
    }

    fn slice_full(self) -> Slice<Self::Buf> {
        ManuallyDrop::into_inner(self.buf).slice_full()
    }

    fn get_buf(&self) -> &Self::Buf {
        &self.buf
    }

    fn bounds(&self) -> Self::Bounds {
        self.buf.bounds()
    }

    fn from_buf_bounds(buf: Self::Buf, _: Self::Bounds) -> Self {
        Self {
            buf: ManuallyDrop::new(buf),
        }
    }

    fn stable_ptr(&self) -> *const u8 {
        BoundedBuf::stable_ptr(self.buf.deref())
    }

    fn bytes_init(&self) -> usize {
        BoundedBuf::bytes_init(self.buf.deref())
    }

    fn bytes_total(&self) -> usize {
        BoundedBuf::bytes_total(self.buf.deref())
    }
}

impl BoundedBufMut for Frame {
    type BufMut = FixedBuf;

    fn stable_mut_ptr(&mut self) -> *mut u8 {
        BoundedBufMut::stable_mut_ptr(self.buf.deref_mut())
    }

    unsafe fn set_init(&mut self, pos: usize) {
        BoundedBufMut::set_init(self.buf.deref_mut(), pos)
    }
}

/// A buffer that is shared between the user and the kernel.
pub struct SharedFrame {
    /// Owned buffer to data.
    /// Eventually, we will want to make sure all buffers point  to contiguous memory.
    buf: Box<[u8; PAGE_SIZE]>,
}

impl Default for SharedFrame {
    fn default() -> Self {
        // Allocate PAGE_SIZE bytes on the heap
        let boxed_slice = vec![0u8; PAGE_SIZE].into_boxed_slice();

        // Turn into a raw pointer
        let ptr = Box::into_raw(boxed_slice) as *mut [u8; PAGE_SIZE];

        // Reconstruct a Box of an array with a fixed size.
        // Safety: We allocated exactly PAGE_SIZE bytes through the vector, so we are able to
        // reconstruct as a boxed array with PAGE_SIZE bytes.
        let buf = unsafe { Box::from_raw(ptr) };

        Self { buf }
    }
}

unsafe impl IoBuf for SharedFrame {
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

unsafe impl IoBufMut for SharedFrame {
    fn stable_mut_ptr(&mut self) -> *mut u8 {
        self.buf.as_mut_ptr()
    }

    unsafe fn set_init(&mut self, _: usize) {
        panic!("Not allowed to change the size of a page")
    }
}
