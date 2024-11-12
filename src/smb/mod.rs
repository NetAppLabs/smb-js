use std::io::Result;
use std::fmt::Debug;

mod libsmb;
mod mock;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Time {
    pub seconds: u32,
    pub nseconds: u64,
}

pub trait SMB: Debug + Send + Sync {
    //fn access(&self, path: &str, mode: u32) -> Result<()>;
    fn stat64(&self, path: &str) -> Result<SMBStat64>;
    //fn lchmod(&self, path: &str, mode: u32) -> Result<()>;
    fn opendir(&mut self, path: &str) -> Result<Box<dyn SMBDirectory>>;
    fn mkdir(&self, path: &str, mode: u32) -> Result<()>;
    fn create(&mut self, path: &str, flags: u32, mode: u32) -> Result<Box<dyn SMBFile>>;
    fn rmdir(&self, path: &str) -> Result<()>;
    fn unlink(&self, path: &str) -> Result<()>;
    fn open(&mut self, path: &str, flags: u32) -> Result<Box<dyn SMBFile>>;
    fn truncate(&self, path: &str, len: u64) -> Result<()>;
}

pub trait SMBDirectory: Debug + Iterator<Item = Result<SMBDirEntry>> {}

pub trait SMBFile: Debug {
    fn fstat64(&self) -> Result<SMBStat64>;
    fn pread_into(&self, count: u32, offset: u64, buffer: &mut [u8]) -> Result<u32>;
    fn pwrite(&self, buffer: &[u8], offset: u64) -> Result<u32>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum SMBEntryType {
    Block,
    Character,
    Directory,
    File,
    NamedPipe,
    Symlink,
    Socket,
}

impl From<u32> for SMBEntryType {
    fn from(val: u32) -> Self {
        match val {
            0 => Self::Block,
            1 => Self::Character,
            2 => Self::Directory,
            3 => Self::File,
            4 => Self::NamedPipe,
            5 => Self::Symlink,
            6 => Self::Socket,
            _ => panic!("invalid entry type"),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SMBDirEntry {
    pub path: String,
    pub inode: u64,
    pub d_type: SMBEntryType,
    pub size: u64,
    pub atime: Time,
    pub mtime: Time,
    pub ctime: Time,
    pub nlink: u32,
    pub atime_nsec: u64,
    pub mtime_nsec: u64,
    pub ctime_nsec: u64,
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct SMBStat64 {
  pub ino: u64,
  pub nlink: u64,
  pub size: u64,
  pub atime: u64,
  pub mtime: u64,
  pub ctime: u64,
  pub atime_nsec: u64,
  pub mtime_nsec: u64,
  pub ctime_nsec: u64,
}

pub(crate) fn connect(url: String) -> Result<Box<dyn SMB>> {
    if std::env::var("TEST_USING_MOCKS").is_ok() {
        mock::SMBConnection::connect(url)
    } else {
        libsmb::SMBConnection::connect(url)
    }
}
