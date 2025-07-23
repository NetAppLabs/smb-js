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

use std::collections::{BTreeSet, BTreeMap};
use std::io::Error;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, RwLock};
use bytes::BufMut;

use super::{Result, VFSDirEntry, VFSDirectory, VFSEntryType, VFSFile, VFSStat, Time, VFS};
use crate::get_parent_path_and_name;


macro_rules! using_rwlock {
    ( $rwlock:expr ) => {
      $rwlock.write().unwrap()
    };
}

macro_rules! using_rwlock_read {
    ( $rwlock:expr ) => {
      $rwlock.read().unwrap()
    };
}  
  

#[derive(Debug)]
struct Mocks {
    dirs: BTreeSet<String>,
    files: BTreeMap<String, Vec<u8>>
}

#[derive(Debug, Clone)]
pub(super) struct SMBConnection {
    mocks: Arc<RwLock<Mocks>>,
}

impl SMBConnection {
    pub(super) fn connect(_url: String) -> Result<Box<dyn VFS>> {
        let mut mocks = Mocks{dirs: BTreeSet::new(), files: BTreeMap::new()};
        let _ = mocks.dirs.insert("/".into());
        let _ = mocks.dirs.insert("/first/".into());
        let _ = mocks.dirs.insert("/quatre/".into());
        let _ = mocks.files.insert("/3".into(), Vec::new());
        let _ = mocks.files.insert("/annar".into(), "In order to make sure that this file is exactly 123 bytes in size, I have written this text while watching its chars count.".as_bytes().to_vec());
        let _ = mocks.files.insert("/first/comment".into(), Vec::new());
        let _ = mocks.files.insert("/quatre/points".into(), Vec::new());
        Ok(Box::new(SMBConnection{mocks: Arc::new(RwLock::new(mocks))}))
    }
}

impl VFS for SMBConnection {
    /*fn access(&self, path: &str, mode: u32) -> Result<()> {
        let p = Path::new(path);
        if let Some(name) = p.file_name() {
            if (name != "3" && name != "quatre") || mode & 0o222 != 0 {
                return Ok(());
            }
        }
        Err(Error::new(std::io::ErrorKind::PermissionDenied, "permission denied"))
    }*/

    fn stat(&self, path: &str) -> Result<VFSStat> {
        let mocks = using_rwlock_read!(&self.mocks);
        let size = if let Some(c) = mocks.files.get(&path.to_string()) {
            Some(c.len() as u64)
        } else {
            if !mocks.dirs.contains(&path.to_string()) {
                return Err(Error::new(std::io::ErrorKind::Other, "entry not found"));
            }
            None
        };
        /*let mode = if size.is_some() {
            if path == "/3" { 0o444 } else { 0o664 }
        } else {
            if path == "/quatre" || path == "/quatre/" { 0o555 } else { 0o775 }
        };*/

        Ok(VFSStat{
            ino: Default::default(),
            nlink: Default::default(),
            size: size.unwrap_or_default(),
            atime: 1658159058723,
            mtime: 1658159058723,
            ctime: 1658159058720,
            btime: 1658159058718,
            atime_nsec: Default::default(),
            mtime_nsec: Default::default(),
            ctime_nsec: Default::default(),
            btime_nsec: Default::default(),
        })
    }

    //fn lchmod(&self, _path: &str, _mode: u32) -> Result<()> {
    //    Ok(())
    //}

    fn opendir(&mut self, path: &str) -> Result<Box<dyn VFSDirectory>> {
        let mocks = using_rwlock_read!(&self.mocks);
        if path != "/" && mocks.dirs.get(&path.to_string()).is_none() {
            return Err(Error::new(std::io::ErrorKind::Other, "not found or not a directory"));
        }
        Ok(Box::new(SMBSDirectory2{smb: self.clone(), path: path.to_string(), entries: None, index: 0}))
    }

    fn mkdir(&self, path: &str, _mode: u32) -> Result<()> {
        let mocks = &mut using_rwlock!(self.mocks);
        let _ = mocks.dirs.insert(path.to_string() + "/");
        Ok(())
    }

    fn create(&mut self, path: &str, _flags: u32, _mode: u32) -> Result<Box<dyn VFSFile>> {
        let mocks = &mut using_rwlock!(self.mocks);
        let _ = mocks.files.insert(path.to_string(), Vec::new());
        Ok(Box::new(SMBFile2{smb: self.clone(), path: path.to_string()}))
    }

    fn rmdir(&self, path: &str) -> Result<()> {
        let mocks = &mut using_rwlock!(self.mocks);
        let path = path.to_string() + "/";
        let _ = mocks.dirs.remove(&path);
        Ok(())
    }

    fn unlink(&self, path: &str) -> Result<()> {
        let mocks = &mut using_rwlock!(self.mocks);
        let _ = mocks.files.remove(&path.to_string());
        Ok(())
    }

    fn open(&mut self, path: &str, _flags: u32) -> Result<Box<dyn VFSFile>> {
        let mocks = &mut using_rwlock!(self.mocks);
        if mocks.dirs.get(&path.to_string()).is_some() {
            return Err(Error::new(std::io::ErrorKind::Other, "is a directory"));
        }
        if mocks.files.get(&path.to_string()).is_none() {
            mocks.files.insert(path.to_string(), Vec::new());
        }
        Ok(Box::new(SMBFile2{smb: self.clone(), path: path.to_string()}))
    }

    fn truncate(&self, path: &str, len: u64) -> Result<()> {
        let mocks = &mut using_rwlock!(self.mocks);
        let contents = mocks.files.entry(path.to_string()).or_default();
        contents.resize(len as usize, 0);
        Ok(())
    }

    fn watch(&self, _path: &str, _mode: super::VFSWatchMode, _listen_events: super::VFSFileNotificationOperationFlags, _cb: Box<dyn super::VFSNotifyChangeCallback>, _ready_tx: &Sender<bool>, _cancelled_rx: &Receiver<bool>) {
        todo!("watch unimplemented for mock")
    }
}

#[derive(Debug)]
pub struct SMBSDirectory2 {
    smb: SMBConnection,
    path: String,
    entries: Option<Vec<VFSDirEntry>>,
    index: usize,
}

impl VFSDirectory for SMBSDirectory2 {
}

impl Iterator for SMBSDirectory2 {
    type Item = Result<VFSDirEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.entries.is_none() {
            let mut entries = Vec::new();
            let mocks = using_rwlock_read!(self.smb.mocks);
            // XXX: technically should add '.' and '..' to entries but don't bother since they will be ignored anyway
            for (mock_file, content) in &mocks.files {
                let (parent_path, name) = get_parent_path_and_name(&mock_file);
                if parent_path == self.path {
                        //let mode = if mock_file == "/3" { 0o444 } else { 0o664 };
                        entries.push(VFSDirEntry{
                        path: name,
                        inode: Default::default(),
                        d_type: VFSEntryType::File,
                        size: content.len() as u64,
                        atime: Time{seconds: 1658159058, nseconds: 0},
                        mtime: Time{seconds: 1658159058, nseconds: 0},
                        ctime: Time{seconds: 1658159055, nseconds: 0},
                        btime: Time{seconds: 1658159053, nseconds: 0},
                        nlink: Default::default(),
                        atime_nsec: Default::default(),
                        mtime_nsec: Default::default(),
                        ctime_nsec: Default::default(),
                        btime_nsec: Default::default(),
                    });
                }
            }
            for mock_dir in mocks.dirs.iter().rev() {
                let (parent_path, name) = get_parent_path_and_name(&mock_dir.trim_end_matches('/').into());
                if parent_path == self.path {
                    //let mode = if mock_dir == "/quatre/" { 0o555 } else { 0o775 };
                    entries.push(VFSDirEntry{
                        path: name,
                        inode: Default::default(),
                        d_type: VFSEntryType::Directory,
                        size: Default::default(),
                        atime: Time{seconds: 1658159058, nseconds: 0},
                        mtime: Time{seconds: 1658159058, nseconds: 0},
                        ctime: Time{seconds: 1658159055, nseconds: 0},
                        btime: Time{seconds: 1658159053, nseconds: 0},
                        nlink: Default::default(),
                        atime_nsec: Default::default(),
                        mtime_nsec: Default::default(),
                        ctime_nsec: Default::default(),
                        btime_nsec: Default::default(),
                    });
                }
            }
            self.entries = Some(entries);
            self.index = 0;
        }

        let mut ret = None;
        if let Some(entries) = &self.entries {
            if self.index < entries.len() {
                ret = Some(Ok(entries[self.index].clone()));
                self.index += 1;
            } else {
                self.entries = None;
                self.index = 0;
            }
        }
        ret
    }
}

#[derive(Debug)]
pub struct SMBFile2 {
    smb: SMBConnection,
    path: String,
}

impl VFSFile for SMBFile2 {
    fn fstat(&self) -> Result<VFSStat> {
        let mocks = using_rwlock_read!(self.smb.mocks);
        let size = if let Some(c) = mocks.files.get(&self.path) {
            c.len() as u64
        } else {
            0
        };
        Ok(VFSStat{
            ino: Default::default(),
            nlink: Default::default(),
            size,
            atime: 1658159058723,
            mtime: 1658159058723,
            ctime: 1658159058720,
            btime: 1658159058718,
            atime_nsec: Default::default(),
            mtime_nsec: Default::default(),
            ctime_nsec: Default::default(),
            btime_nsec: Default::default(),
        })
    }

    fn get_max_read_size(&self) -> u64 {
        8*1024*1024 // XXX: mimic samba's (default?) max_read_size of 8 MiB
    }

    fn pread_into(&self, count: u32, offset: u64, buffer: &mut [u8]) -> Result<u32> {
        let mocks = using_rwlock_read!(self.smb.mocks);
        let readlen = if let Some(content) = mocks.files.get(&self.path) {
            let (offset, count, len) = (offset as usize, count as usize, content.len());
            let start = if offset <= len { offset } else { len };
            let end = if start + count <= len { start + count } else { len };
            let data = content.get(start..end).unwrap_or_default();
            buffer.as_mut().put_slice(data);
            data.len() as u32
        } else {
            0
        };
        Ok(readlen)
    }

    fn pwrite(&self, buffer: &[u8], offset: u64) -> Result<u32> {
        let mut mocks = using_rwlock!(self.smb.mocks);
        let contents = mocks.files.entry(self.path.clone()).or_default();
        let offset = offset as usize;
        let writelen = if contents.len() >= offset + buffer.len() {
            contents.splice(offset..(offset + buffer.len()), buffer.iter().cloned());
            buffer.len() as u32
        } else {
            let padlen = offset - contents.len();
            contents.resize(offset, 0);
            contents.append(&mut buffer.to_vec());
            (padlen + buffer.len()) as u32
        };
        Ok(writelen)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_implementation_works() {
        let smb = SMBConnection::connect(String::new());
        match smb {
            Ok(mut smb) => {
                let res = smb.opendir("/");
                assert!(res.is_ok(), "err = {}", res.unwrap_err());
                match res {
                    Ok(dir) => {
                        let mut entries = Vec::new();
                        for entry in dir {
                            let res: Result<VFSDirEntry> = entry;
                            if let Some(e) = res.ok() {
                                entries.push((e.path, e.d_type));
                            }
                        }
                        let expected_entries = vec![
                            ("3".to_string(), VFSEntryType::File),
                            ("annar".to_string(), VFSEntryType::File),
                            ("quatre".to_string(), VFSEntryType::Directory),
                            ("first".to_string(), VFSEntryType::Directory),
                        ];
                        assert_eq!(entries, expected_entries);        
                    },
                    Err(_) => {},
                }
                let res = smb.opendir("/first/");
                assert!(res.is_ok(), "err = {}", res.unwrap_err());
                match res {
                    Ok(subdir) => {
                        let mut subentries = Vec::new();
                        for subentry in subdir {
                            let res: Result<VFSDirEntry> = subentry;
                            if let Some(e) = res.ok() {
                                subentries.push((e.path, e.d_type));
                            }
                        }
                        let expected_subentries = vec![
                            ("comment".to_string(), VFSEntryType::File),
                        ];
                        assert_eq!(subentries, expected_subentries);        
        
                    },
                    Err(_) => {},
                }
            },
            Err(_) => {},
        }
    }
}
