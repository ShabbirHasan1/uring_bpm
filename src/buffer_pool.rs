use crate::page::{Page, PageId};
use crate::replacer::Replacer;
use anyhow::Result;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

struct ReadPageGuard<'a> {
    pid: PageId,
    buf: RwLockReadGuard<'a, Page>,
}

struct WritePageGuard<'a> {
    pid: PageId,
    buf: RwLockWriteGuard<'a, Page>,
}

#[derive(Default, Clone)]
struct Frame {
    page: Arc<RwLock<Page>>,
}

struct BufferPool<R: Replacer> {
    todo: bool,
    replacer: R,
    frames: Vec<Frame>,
}

impl<R: Replacer> BufferPool<R> {
    pub fn new(size: usize, replacer: R) -> Self {
        Self {
            todo: false,
            replacer,
            frames: vec![Frame::default(); size],
        }
    }

    pub fn read(&self, pid: PageId) -> Result<ReadPageGuard> {
        todo!()
    }

    pub fn write(&self, pid: PageId) -> Result<WritePageGuard> {
        todo!()
    }
}
