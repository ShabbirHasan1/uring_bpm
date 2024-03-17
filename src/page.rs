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
    /// If page data is currently in memory as a `FixedBuf`:
    /// - read should take a read lock and directly dereference a pointer
    /// - write should take a write lock and be able to change into None
    /// If data is not in memory:
    /// - Should just be None
    ///
    /// TODO is there a way to check if the data is in memory without taking a read guard?
    /// Specifically, a way via the type system?
    data: RwLock<Option<FixedBuf>>,
}

// impl Page {
//     fn new(pid: PageId) -> Self {
//         Self {
//             pid,
//             inner: None,
//         }
//     }

//     /// Reads a page.
//     async fn read(&self) -> ReadPageGuard {
//         todo!()
//     }

//     /// Writes to a page.
//     ///
//     /// Can use this function for eviction.
//     async fn write(&self) -> WritePageGuard {
//         todo!()
//     }
// }

// struct ReadPageGuard<'a> {
//     guard: RwLockReadGuard<'a, FixedBuf>,
// }

// impl Deref for ReadPageGuard<'_> {
//     type Target = FixedBuf;

//     fn deref(&self) -> &Self::Target {
//         self.guard.deref()
//     }
// }

// struct WritePageGuard<'a> {
//     guard: RwLockWriteGuard<'a, FixedBuf>,
// }

// impl Deref for WritePageGuard<'_> {
//     type Target = FixedBuf;

//     fn deref(&self) -> &Self::Target {
//         self.guard.deref()
//     }
// }

// impl DerefMut for WritePageGuard<'_> {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         self.guard.deref_mut()
//     }
// }