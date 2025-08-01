// Copyright 2025 NetApp Inc. All Rights Reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

use core::fmt;
use std::{io::Result, sync::mpsc::{Receiver, Sender}};
use std::fmt::Debug;

mod libsmb;
mod mock;
use enumflags2::{bitflags, BitFlags};
use libsmb2_rs::SmbNotifyChangeCallback;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Time {
    pub seconds: u32,
    pub nseconds: u64,
}

#[bitflags]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum VFSFileNotificationOperation {
    Create      = 1<<1,
    Open        = 1<<2,
    Read        = 1<<3,
    Write       = 1<<4,
    Remove      = 1<<5,
    Rename      = 1<<6,
    ChAttr      = 1<<7,
    RdAttr      = 1<<8,
    Move        = 1<<9,
    CloseWrite  = 1<<10,
}

pub type VFSFileNotificationOperationFlags = BitFlags<VFSFileNotificationOperation>;


impl fmt::Display for VFSFileNotificationOperation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

#[bitflags]
#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u32)]
pub enum VFSWatchMode {
    Default,
    Recursive
}

pub trait VFSNotifyChangeCallback {
    fn call(&self, path: String, action: String, from_path: Option<String>);
}

pub struct NotifyChangeCallback {
    inner: Box<dyn VFSNotifyChangeCallback>,
}

impl SmbNotifyChangeCallback for NotifyChangeCallback {
    fn call(&self, path: String, action: String, from_path: Option<String>) {
        self.inner.call(path, action, from_path);
    }
}

pub trait VFS: Debug + Send + Sync {
    //fn access(&self, path: &str, mode: u32) -> Result<()>;
    fn stat(&self, path: &str) -> Result<VFSStat>;
    //fn lchmod(&self, path: &str, mode: u32) -> Result<()>;
    fn opendir(&mut self, path: &str) -> Result<Box<dyn VFSDirectory>>;
    fn mkdir(&self, path: &str, mode: u32) -> Result<()>;
    fn create(&mut self, path: &str, flags: u32, mode: u32) -> Result<Box<dyn VFSFile>>;
    fn rmdir(&self, path: &str) -> Result<()>;
    fn unlink(&self, path: &str) -> Result<()>;
    fn open(&mut self, path: &str, flags: u32) -> Result<Box<dyn VFSFile>>;
    fn truncate(&self, path: &str, len: u64) -> Result<()>;

    fn watch(&self, path: &str, mode: VFSWatchMode, listen_events: VFSFileNotificationOperationFlags, cb: Box<dyn VFSNotifyChangeCallback>, ready_tx: &Sender<bool>, cancelled_rx: &Receiver<bool>);
}

pub trait VFSDirectory: Debug + Iterator<Item = Result<VFSDirEntry>> {}

pub trait VFSFile: Debug {
    fn fstat(&self) -> Result<VFSStat>;
    fn get_max_read_size(&self) -> u64;
    fn pread_into(&self, count: u32, offset: u64, buffer: &mut [u8]) -> Result<u32>;
    fn pwrite(&self, buffer: &[u8], offset: u64) -> Result<u32>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum VFSEntryType {
    Block,
    Character,
    Directory,
    File,
    NamedPipe,
    Symlink,
    Socket,
}

impl From<u32> for VFSEntryType {
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
pub struct VFSDirEntry {
    pub path: String,
    pub d_type: VFSEntryType,
    pub inode: u64,
    pub nlink: u32,
    pub size: u64,
    pub atime: Time,
    pub mtime: Time,
    pub ctime: Time,
    pub btime: Time,
    pub atime_nsec: u64,
    pub mtime_nsec: u64,
    pub ctime_nsec: u64,
    pub btime_nsec: u64,
}

#[allow(dead_code)]
#[derive(Debug, Copy, Clone)]
pub struct VFSStat {
  pub ino: u64,
  pub nlink: u64,
  pub size: u64,
  pub atime: u64,
  pub mtime: u64,
  pub ctime: u64,
  pub btime: u64,
  pub atime_nsec: u64,
  pub mtime_nsec: u64,
  pub ctime_nsec: u64,
  pub btime_nsec: u64,
}

pub(crate) fn connect(url: String) -> Result<Box<dyn VFS>> {
    if std::env::var("TEST_USING_MOCKS").is_ok() {
        mock::SMBConnection::connect(url)
    } else {
        libsmb::SMBConnection::connect(url)
    }
}
