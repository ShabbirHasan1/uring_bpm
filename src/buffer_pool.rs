use crate::disk_manager::DiskManager;
use crate::page::{PageBuf, PageId};
use crate::replacer::{AccessType, Replacer};
use anyhow::{anyhow, bail, Result};
use std::collections::HashMap;
use std::io::Read;
use std::ops::Deref;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard, RwLock, RwLockReadGuard, RwLockWriteGuard};

struct ReadPageGuard<'a> {
    fid: usize,
    pid: PageId,
    buf: RwLockReadGuard<'a, Option<PageBuf>>,
}

struct WritePageGuard<'a> {
    fid: usize,
    pid: PageId,
    buf: RwLockWriteGuard<'a, Option<PageBuf>>,
}

/// This is NOT copyable, as we only want one of these objects in existance at any point in time
#[derive(PartialEq, Eq)]
struct FrameId {
    id: usize,
}

impl Into<FrameId> for usize {
    fn into(self) -> FrameId {
        FrameId { id: self }
    }
}

#[derive(Default)]
struct Frame {
    page: RwLock<Option<PageBuf>>,
}

/// Since FrameId is not copyable, there can never be duplicates in `free_frames` or `page_map`
struct FrameManager<R: Replacer> {
    replacer: R,
    free_frames: Vec<FrameId>,
    page_map: HashMap<PageId, FrameId>,
}

impl<R: Replacer> FrameManager<R> {
    fn new(replacer: R, size: usize) -> Self {
        Self {
            replacer,
            free_frames: (0..size).map(Into::into).collect(),
            page_map: HashMap::with_capacity(size),
        }
    }

    /// Returns a frame ID that is available
    fn get_free_frame(manager: &mut MutexGuard<'_, Self>) -> Result<usize> {
        let fid = manager
            .replacer
            .replace()
            .ok_or(anyhow!("Unable to evict any frames"))?;

        // TODO Evict and flush the page on that frame

        todo!()
    }
}

#[derive(Clone)]
pub struct BufferPool<R: Replacer> {
    frames: &'static [Frame],
    frame_pages: Arc<[Option<PageId>]>,
    frame_manager: Arc<Mutex<FrameManager<R>>>,
    disk_manager: Arc<DiskManager>,
}

impl<R: Replacer<Metadata = AccessType>> BufferPool<R> {
    pub async fn new(replacer: R, pool_size: usize) -> Result<Self> {
        let frames = (0..pool_size).map(|_| Frame::default()).collect();
        let frames = Vec::leak(frames);

        let frame_pages: Arc<_> = vec![None; pool_size].into();

        let frame_manager = Arc::new(Mutex::new(FrameManager::new(replacer, pool_size)));

        let disk_manager = Arc::new(DiskManager::new("testdb".to_string()).await?);

        Ok(Self {
            frames,
            frame_pages,
            frame_manager,
            disk_manager,
        })
    }

    async fn flush(&self, fid: usize) -> Result<()> {
        let pid = self.frame_pages[fid]
            .ok_or(anyhow!("frame_pages is not synced correctly with fids"))?;

        // At this point, fid is guaranteed to be available for writing since nobody else can access
        // the FrameId, so we can get the frame in write mode
        let mut write_guard = self.frames[fid]
            .page
            .try_write()
            .expect("Nobody else should have access to this");

        // We need to take ownership of the frame in order for us to give ownership to the kernel
        let frame = write_guard.take().expect("This better be there");

        // Read contents of the page on disk into the frame in memory
        let frame = self.disk_manager.write(pid, frame).await?;

        // Once the kernel is done reading, we can get it back
        write_guard.replace(frame);

        Ok(())
    }

    async fn evict(&self, manager: &mut MutexGuard<'_, FrameManager<R>>) -> Result<FrameId> {
        // First, ask the replacer which frame we should evict
        let eviction_frame = manager
            .replacer
            .replace()
            .ok_or(anyhow!("Unable to replace a frame"))?;

        // Find the page associated with this frame in the page map
        let (&pid, _) = manager
            .page_map
            .iter()
            .find(|(_, v)| v.id == eviction_frame)
            .expect("Frame was in replacer but not the page map");

        // Remove from the page map, give ownership of the FrameId to the caller
        let frame_id = manager
            .page_map
            .remove(&pid)
            .ok_or(anyhow!("Unable to remove page from page map"))?;

        self.flush(eviction_frame);

        Ok(frame_id)
    }

    /// Gets a frame ID identifier (as a `usize`) that has the page's data
    async fn get_frame(&self, pid: PageId) -> Result<usize> {
        let mut manager = self.frame_manager.lock().await;

        // There is a frame that already contains the page we want, so return that
        if let Some(fid) = manager.page_map.get(&pid) {
            return Ok(fid.id);
        }

        // The page is not in our buffer pool, so we need to get a free one
        let fid = match manager.free_frames.pop() {
            Some(fid) => fid,
            // There are no free frames available, so we need to evict and flush a frame
            None => self.evict(&mut manager).await?,
        };

        // At this point, fid is guaranteed to be available for writing since nobody else can access
        // the FrameId, so we can get the frame in write mode
        let mut write_guard = self.frames[fid.id]
            .page
            .try_write()
            .expect("Nobody else should have access to this");

        // We need to take ownership of the frame in order for us to give ownership to the kernel
        let frame = write_guard.take().expect("This better be there");

        // Read contents of the page on disk into the frame in memory
        let frame = self.disk_manager.read(pid, frame).await?;

        // Once the kernel is done reading, we can get it back
        write_guard.replace(frame);

        Ok(fid.id)
    }

    async fn replace(&self, fid: usize) -> Result<()> {
        todo!()
    }

    pub async fn read(&self, pid: PageId, access_type: AccessType) -> Result<ReadPageGuard> {
        let mut manager = self.frame_manager.lock().await;

        let fid = self.get_frame(pid).await?;
        debug_assert!(fid < self.frames.len());

        manager.replacer.record(fid, access_type)?;

        let read_guard = self.frames[fid].page.read().await;

        Ok(ReadPageGuard {
            fid,
            pid,
            buf: read_guard,
        })
    }

    pub fn write(&self, pid: PageId, access_type: AccessType) -> Result<WritePageGuard> {
        todo!()
    }
}
