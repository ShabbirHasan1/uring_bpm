use crate::{
    disk_manager::DiskManager,
    frame::{Frame, SharedFrame},
    page::Page,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_uring::buf::fixed::FixedBufRegistry;

pub struct BufferPool {
    /// The disk manager in charge of reading from and writing to disk.
    disk_manager: DiskManager,
    /// The registry of buffers shared between the user and the kernel.
    registry: Mutex<FixedBufRegistry<SharedFrame>>,
    pages: Box<[Arc<Mutex<Option<Page>>>]>,
}

impl BufferPool {
    pub fn new(disk_manager: DiskManager, frames: usize) -> Self {
        let buffers = (0..frames).map(|_| SharedFrame::default());
        let pages = (0..frames)
            .map(|_| Arc::new(Mutex::new(None)))
            .collect::<Vec<_>>()
            .into_boxed_slice();

        Self {
            disk_manager,
            registry: Mutex::new(FixedBufRegistry::new(buffers)),
            pages,
        }
    }

    pub fn disk_manager(&self) -> &DiskManager {
        &self.disk_manager
    }

    async fn replace(&self) -> Page {
        // Looks at self.pages for candidates for eviction
        // Take out of the 
        todo!("Move this into a dedicated Replacer type")
    }

    /// Finds a page that we are allowed to evict
    pub async fn get_free_frame(&self) -> Frame {
        // Find a page to replace through a replacement algorithm
        let page = self.replace().await;

        // Evict a page and get back its frame
        page.evict().await.unwrap()
    }
}
