use super::{AccessType, Replacer};
use anyhow::Result;

#[derive(Clone, Copy)]
struct MySQLReplacerNode {
    id: usize,
    replaceable: bool,
}

impl From<usize> for MySQLReplacerNode {
    fn from(value: usize) -> Self {
        Self {
            id: value,
            replaceable: true,
        }
    }
}

struct MySQLReplacer {
    total_size: usize,
    max_young_size: usize,
    young_frames: Vec<MySQLReplacerNode>,
    old_frames: Vec<MySQLReplacerNode>,
}

impl MySQLReplacer {
    fn new(size: usize) -> Self {
        let total_size = size;
        // TODO tune this parameter
        let max_young_size = size / 2;
        Self {
            total_size,
            max_young_size,
            young_frames: Vec::with_capacity(max_young_size),
            old_frames: Vec::with_capacity(total_size - max_young_size),
        }
    }
}

impl Replacer for MySQLReplacer {
    type Metadata = AccessType;

    fn is_replaceable(&self, fid: usize) -> Result<bool> {
        if let Some(i) = self.old_frames.iter().position(|&x| x.id == fid) {
            Ok(self.old_frames[i].replaceable)
        } else if let Some(i) = self.young_frames.iter().position(|&x| x.id == fid) {
            Ok(self.young_frames[i].replaceable)
        } else {
            anyhow::bail!("Unable to find frame {} in replacer", fid);
        }
    }

    fn replace(&mut self) -> Option<usize> {
        for i in (0..self.old_frames.len()).rev() {
            if self.old_frames[i].replaceable {
                self.old_frames.remove(i);
                return Some(self.old_frames[i].id);
            }
        }
        for i in (0..self.young_frames.len()).rev() {
            if self.young_frames[i].replaceable {
                self.young_frames.remove(i);
                return Some(self.young_frames[i].id);
            }
        }
        None
    }

    fn record(&mut self, fid: usize, metadata: Self::Metadata) -> Result<()> {
        anyhow::ensure!(
            fid < self.total_size,
            "Received a frame {} larger than the total size {}",
            fid,
            self.total_size
        );

        // If the frame is not in either the young or old frames, insert into the old self.
        // If old frames is full, move the previous head of the old frames into the young frames,
        // and then insert the new frame into the old self.
        // If the frame is already in the replacer, move to the head of the young self.
        if let Some(i) = self.old_frames.iter().position(|&x| x.id == fid) {
            self.old_frames.remove(i);
            match metadata {
                AccessType::Lookup => {
                    self.young_frames.insert(0, fid.into());
                }
                _ => self.old_frames.insert(0, fid.into()),
            }
        } else if let Some(i) = self.young_frames.iter().position(|&x| x.id == fid) {
            self.young_frames.remove(i);
            self.young_frames.insert(0, fid.into());
        } else {
            if self.old_frames.len() >= self.total_size - self.max_young_size {
                self.young_frames.push(self.old_frames[0]);
                self.old_frames.remove(0);
            }
            self.old_frames.insert(0, fid.into());
        }

        // Extra bookkeeping
        if self.young_frames.len() >= self.max_young_size {
            let oldest_young = self.young_frames.pop().unwrap();
            self.old_frames.insert(0, oldest_young);
        }
        assert!(self.young_frames.len() <= self.max_young_size);
        assert!(self.old_frames.len() <= self.total_size - self.max_young_size);

        Ok(())
    }

    fn set_replaceable(&mut self, fid: usize, is_replaceable: bool) -> Result<()> {
        if let Some(i) = self.old_frames.iter().position(|&x| x.id == fid) {
            self.old_frames[i].replaceable = is_replaceable;
            Ok(())
        } else if let Some(i) = self.young_frames.iter().position(|&x| x.id == fid) {
            self.young_frames[i].replaceable = is_replaceable;
            Ok(())
        } else {
            anyhow::bail!("Unable to find frame {} in replacer", fid);
        }
    }

    fn remove(&mut self, fid: usize) -> Result<()> {
        if let Some(i) = self.old_frames.iter().position(|&x| x.id == fid) {
            self.old_frames.remove(i);
            Ok(())
        } else if let Some(i) = self.young_frames.iter().position(|&x| x.id == fid) {
            self.young_frames.remove(i);
            Ok(())
        } else {
            anyhow::bail!("Unable to find frame {} in replacer", fid);
        }
    }

    fn available(&self) -> usize {
        let mut count = 0;
        for old_frame in &self.old_frames {
            if old_frame.replaceable {
                count += 1;
            }
        }
        for young_frame in &self.young_frames {
            if young_frame.replaceable {
                count += 1;
            }
        }
        count
    }
}
