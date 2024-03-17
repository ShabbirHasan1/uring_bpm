use crate::page::{PageId, PAGE_SIZE};
use anyhow::Result;
use tokio_uring::buf::fixed::FixedBuf;

pub struct DiskManager {
    file_path: String,
}

impl DiskManager {
    /// Creates a new [`DiskManager`] instance.
    pub async fn new(file_path: String) -> Result<Self> {
        todo!()
    }

    /// Reads a page on disk into a `Frame`, overwriting any data in the input `Frame`,
    pub async fn read(&self, pid: PageId, frame: FixedBuf) -> Result<FixedBuf> {
        todo!()
    }

    /// Writes a `Frame`'s contents out to disk, overwriting data on the disk,
    pub async fn write(&self, pid: PageId, frame: FixedBuf) -> Result<FixedBuf> {
        todo!()
    }

    /// Removes a page of memory from the disk, allowing the `PageId` to be used for other things.
    pub fn remove(&self, pid: PageId) -> Result<()> {
        todo!()
    }
}
