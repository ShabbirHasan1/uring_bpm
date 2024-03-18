use crate::{buffer_pool::BufferPool, frame::Frame};
use std::io;
use std::sync::atomic::{AtomicU8, Ordering};
use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

/// The size of a page of data on disk and in memory, 4KB.
pub const PAGE_SIZE: usize = 1 << 12;

/// An identifier for a logical 4KB page of data.
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

const HOT: u8 = 0;
const COOL: u8 = 1;
const COLD: u8 = 2;

/// 3 States: Hot, Cool, and Cold
/// If it is Hot or Cool, then the page is in memory (Some variant)
/// If it is Cold, the page is not in memory (None variant)
/// When this is Cold,
struct Temperature(AtomicU8);

impl Temperature {
    /// Changes state from `COOL` to `HOT. Does not change state if it is `COLD` or `HOT`.
    ///
    /// We don't care if this fails, since if it was already `HOT` then we don't need to to
    /// anything, and if it was `COLD`, we don't want to change it.
    fn try_hot(&self) {
        // Ignore the result
        let _ = self
            .0
            .compare_exchange(COOL, HOT, Ordering::Release, Ordering::Relaxed);
    }

    /// Loads the value of the [`Temperature`]
    fn load(&self, order: Ordering) -> u8 {
        self.0.load(order)
    }

    /// Sets the Temperature to the input value.
    ///
    /// # Safety
    ///
    /// The caller must ensure that they only store `COLD` or `COOL`.
    /// Additionally, if the caller is attempting to store `COOL`, they must
    /// ensure that the [`Swip`] that they are coupling this value to actually
    /// exists in memory (is the [`Some`] variant).
    ///
    /// There are no requirements for storing COLD.
    ///
    /// This ensures that the only way to make a page `HOT` is by storing `COLD`
    /// and then making it [`HOT`].
    unsafe fn store(&self, val: u8, order: Ordering) {
        assert!(val == COLD || val == COOL);
        self.0.store(val, order)
    }
}

// An owned page handle, with an inner [`RwLock`]-protected [`Swip`].
pub struct Page {
    /// The unique ID of the logical page of data.
    pid: PageId,

    /// The state of the page
    state: Temperature,

    /// Protected Swip
    swip: RwLock<Swip>,

    /// Pointer back to the Buffer Pool
    bpm: Arc<BufferPool>,
}

/// Either an owned memory frame shared with the kernel, or [`None`].
#[derive(Default)]
pub struct Swip {
    data: Option<Frame>,
}

impl Page {
    /// Loads page data in from disk.
    async fn load<'a>(
        &self,
        mut write_guard: RwLockWriteGuard<'a, Swip>,
    ) -> RwLockWriteGuard<'a, Swip> {
        assert_eq!(self.state.load(Ordering::Acquire), COLD);

        // The page is not in memory, so we need to go grab a free frame to bring it in
        let frame = self.bpm.get_free_frame().await;

        // Bring in new data to that frame
        let (res, frame) = self.bpm.disk_manager().read(self.pid, frame).await;
        res.expect("Unable to write data to disk");

        // Give ownership to the `Swip`
        write_guard.data.replace(frame);

        // Safety: We are storing `COOL` while there exists a valid frame in memory,
        // so this is completely safe.
        unsafe {
            // Since we only just brought this into memory, it is a cold page
            self.state.store(COOL, Ordering::Release);
        }

        write_guard
    }

    /// Reads a page.
    pub async fn read(&self) -> ReadPageGuard {
        self.state.try_hot();

        {
            let guard = self.swip.read().await;
            if guard.data.is_some() {
                // We checked that the data is the `Some` variant, so this will not panic
                return ReadPageGuard::new(guard).unwrap();
            }
        }

        // The page is not in memory, so we need to bring it in
        let write_guard = self.swip.write().await;

        if write_guard.data.is_some() {
            // Someone other writer got in front of us and updated for us
            assert_ne!(self.state.load(Ordering::Acquire), COLD);
            self.state.try_hot();

            return ReadPageGuard::new(write_guard.downgrade()).unwrap();
        }

        let write_guard = self.load(write_guard).await;

        // Return the downgraded guard that now has the valid data
        return ReadPageGuard::new(write_guard.downgrade()).unwrap();
    }

    /// Writes to a page.
    pub async fn write(&self) -> WritePageGuard {
        self.state.try_hot();

        let write_guard = self.swip.write().await;

        if write_guard.data.is_some() {
            // We checked that the data is the `Some` variant, so this will not panic
            return WritePageGuard::new(write_guard).unwrap();
        }

        let write_guard = self.load(write_guard).await;

        // Return the guard that now has the valid data
        return WritePageGuard::new(write_guard).unwrap();
    }

    /// Evicts a page and returns back the frame it used to own
    pub async fn evict(&self) -> io::Result<Frame> {
        // Safety: We are storing `COLD`, which has no requirements, so this is safe
        unsafe {
            // We want to signal to other threads that we are evicting this page
            self.state.store(COLD, Ordering::Release);
        }

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
        res.expect("Unable to write data to disk");

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
