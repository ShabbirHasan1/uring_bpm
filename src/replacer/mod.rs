use anyhow::Result;

pub mod mysql;
pub mod queue;

pub enum AccessType {
    Lookup,
    Scan,
    Index,
    Unknown,
}

pub trait Replacer {
    type Metadata;

    fn is_replaceable(&self, fid: usize) -> Result<bool>;

    fn replace(&mut self) -> Option<usize>;

    fn record(&mut self, fid: usize, metadata: Self::Metadata) -> Result<()>;

    fn set_replaceable(&mut self, fid: usize, is_replaceable: bool) -> Result<()>;

    /// Removes a page from the system
    fn remove(&mut self, fid: usize) -> Result<()>;

    /// Total number of available frames (unoccupied)
    fn available(&self) -> usize;
}
