use std::cell::UnsafeCell;
use tokio::sync::RwLock as TokioRwLock;

pub use tokio::sync::{RwLockReadGuard, RwLockWriteGuard};

pub(crate) struct RwLock<T> {
    rwlock: UnsafeCell<TokioRwLock<T>>,
}

impl<T> RwLock<T> {
    pub(crate) fn new(value: T) -> Self {
        Self {
            rwlock: UnsafeCell::new(TokioRwLock::new(value)),
        }
    }

    fn get_rwlock(&self) -> &TokioRwLock<T> {
        // Safety: We are just getting a shared reference to the underlying
        // `RwLock` type, and we know this pointer is valid because we have a
        // reference to it through `self`.
        // Additionally, no methods on this interface take exclusive access,
        // so they cannot unsafely modify anything under us.
        unsafe { &*self.rwlock.get() }
    }

    pub(crate) fn data_ptr(&self) -> *const T {
        // Safety: We are just giving out a pointer, and we know that this
        // pointer is valid because we have a reference to it through `self`.
        // It is then on the caller to safely use the read-only pointer.
        unsafe { (*self.rwlock.get()).get_mut() as *const T }
    }

    pub(crate) async fn read(&self) -> RwLockReadGuard<'_, T> {
        self.get_rwlock().read().await
    }

    pub(crate) async fn write(&self) -> RwLockWriteGuard<'_, T> {
        self.get_rwlock().write().await
    }
}
