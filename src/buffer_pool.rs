use crate::{
    disk_manager::DiskManager,
    frame::{Frame, SharedFrame},
    page::Page,
};
use tokio_uring::buf::fixed::FixedBufRegistry;

pub struct BufferPool {
    disk_manager: DiskManager,
    /// The registry of buffers shared between the user and the kernel.
    registry: FixedBufRegistry<SharedFrame>,
    /// The pages that are currently in memory
    pages: Vec<Option<Page>>,
}

impl BufferPool {
    pub fn new(disk_manager: DiskManager, frames: usize) -> Self {
        let buffers = (0..frames).map(|_| SharedFrame::default());
        Self {
            disk_manager,
            registry: FixedBufRegistry::new(buffers),
            pages: Vec::with_capacity(frames),
        }
    }

    pub fn disk_manager(&self) -> &DiskManager {
        &self.disk_manager
    }

    fn replace(&self) -> Page {
        todo!("Move this into a dedicated Replacer type")
    }

    /// Finds a page that we are allowed to evict
    pub async fn get_free_frame(&self) -> Frame {
        // Find a page to replace through a replacement algorithm
        let page = self.replace();

        // Evict a page and get back its frame
        page.evict().await.unwrap()
    }
}
