use crate::page::{Page, PageId, PAGE_SIZE};
use anyhow::Result;
use tokio_uring::fs::OpenOptions;

pub struct DiskManager {
    file_path: String,
}

/// TODO update all `open` calls to `O_DIRECT` open versions
impl DiskManager {
    pub async fn new(file_path: String) -> Result<Self> {
        let f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await?;

        f.close().await?;

        Ok(Self { file_path })
    }

    /// Reads a page on disk into a `Frame`, overwriting any data in the input `Frame`
    pub async fn read(&self, pid: PageId, frame: Page) -> Result<Page> {
        let f = OpenOptions::new().read(true).open(&self.file_path).await?;

        let offset = pid.0 * PAGE_SIZE;

        let (res, frame) = f.read_at(frame, offset as u64).await;
        let _n = res?;

        println!("Bytes read at offset {offset}: {_n}");

        f.close().await?;

        Ok(frame)
    }

    /// Writes a `Frame`'s contents out to disk, overwriting data on the disk
    pub async fn write(&self, pid: PageId, frame: Page) -> Result<Page> {
        let f = OpenOptions::new().write(true).open(&self.file_path).await?;

        let offset = pid.0 * PAGE_SIZE;

        let (res, frame) = f.write_at(frame, offset as u64).await;
        let _n = res?;

        println!("Bytes written at offset {offset}: {_n}");

        f.close().await?;

        Ok(frame)
    }
}

#[test]
fn test_dm_basic() {
    tokio_uring::start(async {
        let dm = DiskManager::new("test.txt".to_string())
            .await
            .expect("Unable to create DiskManager");

        let mut frame_0 = Page::default();
        for (i, &b) in b"Hello, World!\n0\n".iter().enumerate() {
            frame_0.buf[i] = b;
        }
        dm.write(PageId(0), frame_0).await.unwrap();

        let mut frame_1 = Page::default();
        for (i, &b) in b"Hello, World!\n1\n".iter().enumerate() {
            frame_1.buf[i] = b;
        }
        dm.write(PageId(1), frame_1).await.unwrap();

        let mut frame_2 = Page::default();
        for (i, &b) in b"Hello, World!\n2\n".iter().enumerate() {
            frame_2.buf[i] = b;
        }
        dm.write(PageId(2), frame_2).await.unwrap();

        let new_frame_0 = dm.read(PageId(0), Page::default()).await.unwrap();
        println!("Page 0 bytes: {:?}", new_frame_0.buf);

        let new_frame_1 = dm.read(PageId(1), Page::default()).await.unwrap();
        println!("Page 1 bytes: {:?}", new_frame_1.buf);
    })
}
