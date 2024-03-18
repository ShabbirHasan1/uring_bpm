use crate::{
    frame::Frame,
    page::{PageId, PAGE_SIZE},
};
use std::io;
use tokio_uring::{
    fs::{File, OpenOptions},
    BufResult,
};

pub struct DiskManager {
    file_path: String,
    fd: File,
}

impl DiskManager {
    /// Creates a new [`DiskManager`] instance.
    pub async fn new(file_path: String) -> io::Result<Self> {
        let fd = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await?;

        // Purposefully leak the file descriptor

        Ok(Self { file_path, fd })
    }

    pub fn file_name(&self) -> &str {
        &self.file_path
    }

    /// Reads a page on disk into a `Frame`, overwriting any data in the input `Frame`,
    pub async fn read(&self, pid: PageId, frame: Frame) -> BufResult<usize, Frame> {
        let page_index: u64 = Into::into(pid);
        let offset = page_index * (PAGE_SIZE as u64);

        self.fd.read_fixed_at(frame, offset).await
    }

    /// Writes a `Frame`'s contents out to disk, overwriting data on the disk,
    pub async fn write(&self, pid: PageId, frame: Frame) -> BufResult<usize, Frame> {
        let page_index: u64 = Into::into(pid);
        let offset = page_index * (PAGE_SIZE as u64);

        self.fd.write_fixed_at(frame, offset).await
    }
}
