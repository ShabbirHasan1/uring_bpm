use super::Replacer;
use crate::page::PageId;
use std::collections::VecDeque;

struct QueueReplacer {
    queue: VecDeque<PageId>,
}

impl Replacer for QueueReplacer {
    type Metadata = ();

    fn is_replaceable(&self, fid: usize) -> anyhow::Result<bool> {
        todo!()
    }

    fn replace(&mut self) -> Option<usize> {
        todo!()
    }

    fn record(&mut self, fid: usize, metadata: Self::Metadata) -> anyhow::Result<()> {
        todo!()
    }

    fn set_replaceable(&mut self, fid: usize, is_replaceable: bool) -> anyhow::Result<()> {
        todo!()
    }

    fn remove(&mut self, fid: usize) -> anyhow::Result<()> {
        todo!()
    }

    fn available(&self) -> usize {
        todo!()
    }
}
