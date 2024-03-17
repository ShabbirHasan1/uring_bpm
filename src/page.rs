use std::ops::{Deref, DerefMut};

use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use tokio_uring::buf::fixed::FixedBuf;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PageId {
    id: usize,
}

/// Must always be able to convert into a 64-bit identifier.
impl From<PageId> for u64 {
    fn from(val: PageId) -> Self {
        val.id as u64
    }
}

pub const PAGE_SIZE: usize = 1 << 12; // 4096

pub struct Page {
    /// The unique ID of the logical page of data.
    pid: PageId,
    /// RwLock-protected optional buffer
    data: RwLock<Option<FixedBuf>>,
}

impl Page {
    fn new(pid: PageId) -> Self {
        Self {
            pid,
            data: RwLock::new(None),
        }
    }

    /// Reads a page.
    async fn read(&self) -> ReadPageGuard {
        todo!()
    }

    /// Writes to a page.
    ///
    /// Can use this function for eviction.
    async fn write(&self) -> WritePageGuard {
        todo!()
    }
}

struct ReadPageGuard<'a> {
    guard: RwLockReadGuard<'a, FixedBuf>,
}

impl Deref for ReadPageGuard<'_> {
    type Target = FixedBuf;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

struct WritePageGuard<'a> {
    guard: RwLockWriteGuard<'a, FixedBuf>,
}

impl Deref for WritePageGuard<'_> {
    type Target = FixedBuf;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl DerefMut for WritePageGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.deref_mut()
    }
}