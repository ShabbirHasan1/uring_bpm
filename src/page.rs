use crate::{buffer_pool::BufferPool, frame::Frame};
use std::io;
use std::sync::atomic::{AtomicU8, Ordering};
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

const HOT: u8 = 0;
const COOL: u8 = 1;
const COLD: u8 = 2;

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

    /// 3 States: Hot, Cool, and Cold
    /// If it is Hot or Cool, then the page is in memory (Some variant)
    /// If it is Cold, the page is not in memory (None variant)
    state: AtomicU8,

    /// Protected Swip
    swip: RwLock<Swip>,

    /// Pointer back to the Buffer Pool
    bpm: Arc<BufferPool>,
}

#[derive(Default)]
pub struct Swip {
    data: Option<Frame>,
}

impl Page {
    /// Changes state from `COOL` to `HOT. Does not change state if it is `COLD` or `HOT`.
    ///
    /// We don't care if this fails, since if it was already `HOT` then we don't need to to
    /// anything, and if it was `COLD`, we don't want to change it.
    fn make_hot(&self) {
        // Ignore the result
        let _ = self
            .state
            .compare_exchange(COOL, HOT, Ordering::Release, Ordering::Relaxed);
    }

    /// Reads a page.
    pub async fn read(&self) -> ReadPageGuard {
        self.make_hot();

        {
            let guard = self.swip.read().await;
            if guard.data.is_some() {
                // We checked that the data is the `Some` variant, so this will not panic
                return ReadPageGuard::new(guard).unwrap();
            }
        }

        {
            // The page is not in memory, so we need to bring it in
            let mut write_guard = self.swip.write().await;

            if write_guard.data.is_some() {
                // Someone other writer got in front of us and updated for us
                assert_ne!(self.state.load(Ordering::Acquire), COLD);
                self.make_hot();

                return ReadPageGuard::new(write_guard.downgrade()).unwrap();
            }

            assert_eq!(self.state.load(Ordering::Acquire), COLD);

            // We need to go grab a free frame
            let frame = self.bpm.get_free_frame().await;

            // Bring in new data to that frame
            let (res, frame) = self.bpm.disk_manager().read(self.pid, frame).await;
            res.expect("Unable to read in data from disk");

            // Give ownership to the `Swip`
            write_guard.data.replace(frame);

            // Since we only just brought this into memory, it is a hot page
            self.state.store(HOT, Ordering::Release);

            // Return the downgraded guard that now has the valid data
            return ReadPageGuard::new(write_guard.downgrade()).unwrap();
        }
    }

    /// Writes to a page.
    pub async fn write(&self) -> WritePageGuard {
        self.make_hot();

        let mut guard = self.swip.write().await;
        if guard.data.is_some() {
            // We checked that the data is the `Some` variant, so this will not panic
            return WritePageGuard::new(guard).unwrap();
        }

        // The page is not in memory, so we need to go grab a free frame to bring it in
        let frame = self.bpm.get_free_frame().await;

        // Bring in new data to that frame
        let (res, frame) = self.bpm.disk_manager().read(self.pid, frame).await;
        res.expect("Unable to read in data from disk");

        // Give ownership to the `Swip`
        guard.data.replace(frame);

        // Since we only just brought this into memory, it is a hot page
        self.state.store(HOT, Ordering::Release);

        // Return the guard that now has the valid data
        return WritePageGuard::new(guard).unwrap();
    }

    /// Evicts a page and returns back the frame it used to own
    pub async fn evict(&self) -> io::Result<Frame> {
        self.state.store(COLD, Ordering::Release);

        let mut guard = self.swip.write().await;

        if guard.data.is_none() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Page data is not actually in memory",
            ));
        }

        let frame = guard
            .data
            .take()
            .expect("All page guards should have valid data");

        // Write the frame's data back out to disk
        let (res, frame) = self.bpm.disk_manager().write(self.pid, frame).await;

        Ok(frame)
    }
}

#[non_exhaustive]
pub struct ReadPageGuard<'a> {
    inner: RwLockReadGuard<'a, Swip>,
}

impl<'a> ReadPageGuard<'a> {
    /// Creates a [`ReadPageGuard`] from a [`RwLockReadGuard`]
    ///
    /// # Requirements
    ///
    /// The caller must guarantee that the inner [`Swip`]'s optional data is the Some(frame)
    /// variant, and not `None`. A [`ReadPageGuard`] must always point to valid data.
    fn new(guard: RwLockReadGuard<'a, Swip>) -> Option<Self> {
        if guard.data.is_none() {
            eprintln!(
                "Trying to construct a ReadPageGuard with a swip that does not have any data"
            );
            return None;
        }

        Some(Self { inner: guard })
    }
}

impl Deref for ReadPageGuard<'_> {
    type Target = Frame;

    fn deref(&self) -> &Self::Target {
        if let Some(frame) = &self.inner.data {
            return frame;
        }

        // Cannot be the `None` variant while a `ReadPageGuard` exists
        panic!("The ReadPageGuard does not point to valid data");
    }
}

pub struct WritePageGuard<'a> {
    inner: RwLockWriteGuard<'a, Swip>,
}

impl<'a> WritePageGuard<'a> {
    /// Creates a [`WritePageGuard`] from a [`RwLockWriteGuard`]
    ///
    /// # Requirements
    ///
    /// The caller must guarantee that the inner [`Swip`]'s optional data is the Some(frame)
    /// variant, and not `None`. A [`WritePageGuard`] must always point to valid data.
    fn new(guard: RwLockWriteGuard<'a, Swip>) -> Option<Self> {
        if guard.data.is_none() {
            eprintln!(
                "Trying to construct a WritePageGuard with a swip that does not have any data"
            );

            return None;
        }

        Some(Self { inner: guard })
    }
}

impl Deref for WritePageGuard<'_> {
    type Target = Frame;

    fn deref(&self) -> &Self::Target {
        if let Some(frame) = &self.inner.data {
            return frame;
        }

        // Cannot dereference into the `None` variant while a `WritePageGuard` exists.
        panic!("The WritePageGuard does not point to valid data");
    }
}

impl DerefMut for WritePageGuard<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        if let Some(frame) = &mut self.inner.data {
            return frame;
        }

        // Cannot dereference into the `None` variant while a `WritePageGuard` exists.
        panic!("The WritePageGuard does not point to valid data");
    }
}
