#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PageId {
    pub id: usize,
}

pub const PAGE_SIZE: usize = 1 << 12; // 4096
