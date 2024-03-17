use crate::frame::Frame;
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};

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

#[derive(Default)]
pub struct Swip {
    data: Option<Frame>,
}

#[derive(Clone)]
pub struct Page {
    /// The unique ID of the logical page of data.
    pid: PageId,
    /// RwLock-protected Swip
    swip: Arc<RwLock<Swip>>,
}

impl Page {
    fn new(pid: PageId) -> Self {
        Self {
            pid,
            swip: Arc::new(RwLock::new(Swip::default())),
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

struct ReadPageGuard {
    guard: OwnedRwLockReadGuard<Frame>,
}

impl Deref for ReadPageGuard {
    type Target = Frame;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

struct WritePageGuard {
    guard: OwnedRwLockWriteGuard<Frame>,
}

impl Deref for WritePageGuard {
    type Target = Frame;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl DerefMut for WritePageGuard {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.deref_mut()
    }
}
