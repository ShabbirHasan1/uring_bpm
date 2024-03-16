use std::{
    ops::{Deref, DerefMut},
    sync::atomic::{fence, AtomicBool, AtomicUsize, Ordering},
};

use crate::rwlock::{RwLock, RwLockReadGuard, RwLockWriteGuard};

const LOCKED: bool = true;
const UNLOCKED: bool = false;

pub struct HybridLock<T> {
    rwlock: RwLock<T>,
    locked_exclusive: AtomicBool,
    version_counter: AtomicUsize,
}

impl<T> HybridLock<T> {
    /// Creates a new instance of [`HybridLock`].
    pub fn new(value: T) -> HybridLock<T> {
        HybridLock {
            rwlock: RwLock::new(value),
            locked_exclusive: AtomicBool::new(UNLOCKED),
            version_counter: AtomicUsize::new(0),
        }
    }

    /// Gets the current version of this lock.
    pub fn current_version(&self) -> usize {
        // TODO is this actually necessary?
        // This `atomic::fence` prevents the reordering of `is_locked_exclusive()` and
        // `self.version.load`.
        // This is necessary as we don't know whether the `RwLock` uses the memory ordering strong
        // enough to prevent such reordering.
        fence(Ordering::Acquire);
        self.version_counter.load(Ordering::Acquire)
    }

    pub fn is_locked_exclusive(&self) -> bool {
        // TODO is this actually necessary?
        fence(Ordering::Acquire);
        self.locked_exclusive.load(Ordering::Acquire)
    }

    /// Locks this hybrid lock with shared read access.
    ///
    /// The calling thread will be blocked until there is no writer which holds the lock.
    pub async fn read(&self) -> HybridRwLockReadGuard<T> {
        let guard = self.rwlock.read().await;
        HybridRwLockReadGuard {
            guard,
            lock_ref: self,
        }
    }

    /// Locks this hybrid lock with exclusive write access.
    ///
    /// The calling thread will be blocked until no readers nor writers hold the lock.
    pub async fn write(&self) -> HybridRwLockWriteGuard<T> {
        let guard = self.rwlock.write().await;

        // This store must be seen by any loads future
        self.locked_exclusive.store(LOCKED, Ordering::Release);

        HybridRwLockWriteGuard {
            guard,
            lock_ref: self,
        }
    }

    /// Runs the given callback without acquiring the lock with fallback mode.
    ///
    /// The calling thread will be blocked when falling back to acquiring a shared access.
    /// This will happen when the optimistic run fails due to a concurrent writer.
    ///
    /// # Safety
    ///
    /// The caller must ensure that they do not create a reference that is
    /// derived from the `*const T` pointer they receive in the closure.
    /// This would break the alias rule that a shared and exclusive reference
    /// could exist at the same time.
    pub async unsafe fn optimistic<F, R>(&self, read_callback: &F) -> R
    where
        F: Fn(*const T) -> R,
    {
        if let Some(result) = self.try_optimistic(read_callback) {
            result
        } else {
            self.fallback(read_callback).await
        }
    }

    /// Runs the given callback without acquiring the lock.
    ///
    /// # Safety
    ///
    /// The caller must ensure that they do not create a reference that is
    /// derived from the `*const T` pointer they receive in the closure.
    /// This would break the alias rule that a shared and exclusive reference
    /// could exist at the same time.
    pub unsafe fn try_optimistic<F, R>(&self, read_callback: &F) -> Option<R>
    where
        F: Fn(*const T) -> R,
    {
        self.is_locked_exclusive().then_some(())?;
        let start_version = self.current_version();

        let result = read_callback(self.rwlock.data_ptr());

        self.is_locked_exclusive().then_some(())?;
        (start_version == self.current_version()).then_some(result)
    }

    async fn fallback<F, R>(&self, read_callback: &F) -> R
    where
        F: Fn(*const T) -> R,
    {
        let guard = self.read().await;
        read_callback(guard.lock_ref.rwlock.data_ptr())
    }
}

/// RAII structure used to release the shared read access of a lock when dropped.
pub struct HybridRwLockReadGuard<'a, T> {
    guard: RwLockReadGuard<'a, T>,
    lock_ref: &'a HybridLock<T>,
}

impl<'a, T> Deref for HybridRwLockReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

/// RAII structure used to release the exclusive write access of a lock when dropped.
pub struct HybridRwLockWriteGuard<'a, T> {
    guard: RwLockWriteGuard<'a, T>,
    lock_ref: &'a HybridLock<T>,
}

impl<'a, T> Deref for HybridRwLockWriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.guard.deref()
    }
}

impl<'a, T> DerefMut for HybridRwLockWriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.guard.deref_mut()
    }
}

impl<T> Drop for HybridRwLockWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.lock_ref.version_counter.fetch_add(1, Ordering::AcqRel);
        self.lock_ref
            .locked_exclusive
            .store(UNLOCKED, Ordering::Release);
    }
}
