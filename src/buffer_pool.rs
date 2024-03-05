use crate::page::{Page, PageId};
use crate::replacer::{AccessType, Replacer};
use anyhow::{anyhow, bail, Result};
use std::collections::HashMap;
use std::io::Read;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

struct ReadPageGuard<'a> {
    fid: usize,
    pid: PageId,
    buf: RwLockReadGuard<'a, Page>,
}

impl<'a> Drop for ReadPageGuard<'a> {
    fn drop(&mut self) {
        todo!()
    }
}

struct WritePageGuard<'a> {
    fid: usize,
    pid: PageId,
    buf: RwLockWriteGuard<'a, Page>,
}

impl<'a> Drop for WritePageGuard<'a> {
    fn drop(&mut self) {
        todo!()
    }
}

#[derive(Default)]
struct Frame {
    page: RwLock<Page>,
}

struct FrameManager<R: Replacer> {
    replacer: R,
    free_frames: Vec<usize>,
    page_map: HashMap<PageId, usize>,
}

impl<R: Replacer> FrameManager<R> {
    /// Returns a frame ID that is available
    fn get_free_frame(manager: &mut MutexGuard<'_, Self>) -> Result<usize> {
        if !manager.free_frames.is_empty() {
            return Ok(manager.free_frames.pop().unwrap());
        }

        let fid = manager
            .replacer
            .replace()
            .ok_or(anyhow!("Unable to evict any frames"))?;

        // TODO Evict and flush the page on that frame

        let (&pid, _) = manager
            .page_map
            .iter()
            .find(|(_, &v)| v == fid)
            .expect("Frame was in replacer but not the page map");

        manager
            .page_map
            .remove(&pid)
            .ok_or(anyhow!("Unable to remove page from page map"))?;

        todo!()
    }
}

#[derive(Clone)]
pub struct BufferPool<R: Replacer> {
    frames: &'static [Frame],
    manager: Arc<Mutex<FrameManager<R>>>,
}

impl<R: Replacer<Metadata = AccessType>> BufferPool<R> {
    pub fn new(pool_size: usize, replacer: R) -> Self {
        // Self {
        //     replacer,
        //     frames: vec![Frame::default(); pool_size],
        //     free_frames: (0..pool_size).collect(),
        //     page_map: PageToFrameMap::default(),
        // }
        todo!()
    }

    pub async fn read(&self, pid: PageId, access_type: AccessType) -> Result<ReadPageGuard> {
        let mut manager = self.manager.lock().await;

        let fid = match manager.page_map.get(&pid) {
            Some(&fid) => fid,
            None => FrameManager::get_free_frame(&mut manager)?,
        };

        debug_assert!(fid < self.frames.len());

        let read_guard = self.frames[fid].page.read().await;

        manager.replacer.record(fid, access_type)?;

        Ok(ReadPageGuard {
            fid,
            pid,
            buf: read_guard,
        })
    }

    pub fn write(&self, pid: PageId, access_type: AccessType) -> Result<WritePageGuard> {
        // let map_guard = self.page_map.map.read().expect("Map lock is poisoned");

        // if let Some(&fid) = map_guard.get(&pid) {
        //     debug_assert!(fid < self.frames.len());

        //     let write_guard = self.frames[fid]
        //         .page
        //         .write()
        //         .expect("Unable to read frame {fid}");

        //     Ok(WritePageGuard {
        //         pid,
        //         buf: write_guard,
        //     })
        // } else {
        //     todo!()
        // }
        todo!()
    }
}
