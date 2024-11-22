use core::fmt::Debug;
use std::path::Path;
use std::sync::{Arc, RwLock};
use nix::sys::stat::Mode;
use nix::fcntl::OFlag;
use libsmb2_rs::{Smb, SmbChangeNotifyAction, SmbChangeNotifyFileFilter, SmbChangeNotifyFlags, SmbNotifyChangeInformation};
use url::{Url};

use super::{Result, SMBDirEntry, SMBDirectory, SMBFile, SMBFileNotificationInformation, SMBFileNotification, SMBFileNotificationBoxed, SMBFileNotificationOperation, SMBFileNotificationOperationFlags, SMBStat64, SMBWatchMode, Time, SMB};

pub(super) struct SMBConnection {
    smb: Arc<RwLock<Smb>>,
}

impl SMBConnection {
    pub(super) fn connect(url: String) -> Result<Box<dyn SMB>> {
        let mut real_url = url;
        let mut smb = Smb::new().unwrap();
        let mut passwd: Option<String> = None;
        let pre_parse_url = Url::parse(real_url.as_str());
        match pre_parse_url {
            Ok(mut purl) => {
                let opasswd = purl.password();
                match opasswd {
                    Some(p) => {
                        let p_owned = p.to_owned();
                        passwd = Some(p_owned);    
                    },
                    None => {},
                }
                let _ = purl.set_password(None);
                real_url = purl.to_string();
            },
            Err(_) => {},
        }
        let conn_res = smb.parse_url_mount(real_url.as_str(), passwd);
        match conn_res {
            Ok(_) => {
                return Ok(Box::new(SMBConnection{smb: Arc::new(RwLock::new(smb))}));
            },
            Err(e) => {
                return Err(e);
            }
        }
    }
}

impl Debug for SMBConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SMBConnection").finish()
    }
}

impl SMB for SMBConnection {
    /*fn access(&self, path: &str, mode: u32) -> Result<()> {
        let my_smb = self.smb.write().unwrap();
        my_smb.access(Path::new(path), mode as i32).map(|_| ())
    }*/

    fn stat64(&self, path: &str) -> Result<SMBStat64> {
        let smb_path = normalize_smb_path(path);
        let my_smb = self.smb.write().unwrap();
        my_smb.stat64(Path::new(smb_path)).map(|res| SMBStat64{
            ino: res.smb2_ino,
            nlink: res.smb2_nlink.into(),
            size: res.smb2_size,
            atime: res.smb2_atime,
            mtime: res.smb2_mtime,
            ctime: res.smb2_ctime,
            atime_nsec: res.smb2_atime_nsec,
            mtime_nsec: res.smb2_mtime_nsec,
            ctime_nsec: res.smb2_ctime_nsec,
        })
    }

    fn opendir(&mut self, path: &str) -> Result<Box<dyn SMBDirectory>> {
        let smb_path = normalize_smb_path(path);
        let mut my_smb = self.smb.write().unwrap();
        let dir = my_smb.opendir(Path::new(smb_path))?;
        Ok(Box::new(SMBDirectory2{dir}))
    }

    fn mkdir(&self, path: &str, _mode: u32) -> Result<()> { // FIXME: mode
        let smb_path = normalize_smb_path(path);
        let my_smb = self.smb.write().unwrap();
        my_smb.mkdir(Path::new(smb_path))
    }

    fn create(&mut self, path: &str, flags: u32, mode: u32) -> Result<Box<dyn SMBFile>> {
        let smb_path = normalize_smb_path(path);
        let mut my_smb = self.smb.write().unwrap();
        let file = my_smb.create(Path::new(smb_path), OFlag::from_bits_truncate(flags as i32), Mode::from_bits_truncate((mode as u16).into()))?;
        Ok(Box::new(SMBFile2{file}))
    }

    fn rmdir(&self, path: &str) -> Result<()> {
        let smb_path = normalize_smb_path(path);
        let my_smb = self.smb.write().unwrap();
        my_smb.rmdir(Path::new(smb_path))
    }

    fn unlink(&self, path: &str) -> Result<()> {
        let smb_path = normalize_smb_path(path);
        let my_smb = self.smb.write().unwrap();
        my_smb.unlink(Path::new(smb_path))
    }

    fn open(&mut self, path: &str, flags: u32) -> Result<Box<dyn SMBFile>> {
        let smb_path = normalize_smb_path(path);
        let mut my_smb = self.smb.write().unwrap();
        let file = my_smb.open(Path::new(smb_path), OFlag::from_bits_truncate(flags as i32))?;
        Ok(Box::new(SMBFile2{file}))
    }

    fn truncate(&self, path: &str, len: u64) -> Result<()> {
        let smb_path = normalize_smb_path(path);
        let my_smb = self.smb.write().unwrap();
        my_smb.truncate(Path::new(smb_path), len)
    }
    
    fn watch(&self, path: &str, mode: super::SMBWatchMode, listen_events: super::SMBFileNotificationOperationFlags) -> Result<SMBFileNotificationBoxed> {
        let smb_path = normalize_smb_path(path);
        let my_smb = self.smb.write().unwrap();
        let notify_flags = SmbChangeNotifyFlags::my_from(mode);
        let notify_filter = SmbChangeNotifyFileFilter::my_from(listen_events);
        let notify_change = my_smb.notify_change(Path::new(smb_path), notify_flags, notify_filter)?;
        let ret = SMBFileNotificationBoxed::my_from(notify_change);
        return Ok(ret);
    }
}

pub trait MyFrom<T> {
    fn my_from(value: T) -> Self;
}

impl MyFrom<SMBWatchMode> for SmbChangeNotifyFlags {
    fn my_from(value: SMBWatchMode) -> Self {
        match value {
            SMBWatchMode::Default => {
                SmbChangeNotifyFlags::DEFAULT
            },
            SMBWatchMode::Recursive => {
                SmbChangeNotifyFlags::WATCH_TREE
            },
        }
    }
}

impl MyFrom<SMBFileNotificationOperationFlags> for SmbChangeNotifyFileFilter {
    fn my_from(value: SMBFileNotificationOperationFlags) -> Self {
        let mut ret = SmbChangeNotifyFileFilter::empty();
        if value.contains(SMBFileNotificationOperation::Create) {
            ret = ret | SmbChangeNotifyFileFilter::CHANGE_CREATION;
        }
        if value.contains(SMBFileNotificationOperation::Write) {
            ret = ret | SmbChangeNotifyFileFilter::CHANGE_LAST_WRITE;
            ret = ret | SmbChangeNotifyFileFilter::CHANGE_STREAM_SIZE;
        }
        if value.contains(SMBFileNotificationOperation::Remove) {
            ret = ret | SmbChangeNotifyFileFilter::CHANGE_LAST_ACCESS;
            ret = ret | SmbChangeNotifyFileFilter::CHANGE_LAST_ACCESS;

        }
        if value.contains(SMBFileNotificationOperation::ChAttr) {
            ret = ret | SmbChangeNotifyFileFilter::CHANGE_ATTRIBUTES;
            ret = ret | SmbChangeNotifyFileFilter::CHANGE_LAST_ACCESS;
            ret = ret | SmbChangeNotifyFileFilter::CHANGE_CREATION;
            ret = ret | SmbChangeNotifyFileFilter::CHANGE_EA;
        }
        if value.contains(SMBFileNotificationOperation::Rename) {
            ret = ret | SmbChangeNotifyFileFilter::CHANGE_FILE_NAME;
            ret = ret | SmbChangeNotifyFileFilter::CHANGE_DIR_NAME;
        }

        return ret;
    }
}

impl MyFrom<SmbChangeNotifyAction> for SMBFileNotificationOperation {
    fn my_from(value: SmbChangeNotifyAction) -> Self {
        match value {
            SmbChangeNotifyAction::Added => {
                return SMBFileNotificationOperation::Create;
            },
            SmbChangeNotifyAction::Removed => {
                return SMBFileNotificationOperation::Remove;
            },
            SmbChangeNotifyAction::Modified => {
                return SMBFileNotificationOperation::Write;
            },
            SmbChangeNotifyAction::RenamedOldName => {
                return SMBFileNotificationOperation::Rename;
            },
            SmbChangeNotifyAction::RenamedNewName => {
                return SMBFileNotificationOperation::Rename;
            },
            SmbChangeNotifyAction::AddedStream => {
                return SMBFileNotificationOperation::Write;
            },
            SmbChangeNotifyAction::RemovedStream => {
                return SMBFileNotificationOperation::Write;
            },
            SmbChangeNotifyAction::ModifiedStream => {
                return SMBFileNotificationOperation::Write;
            },
        }
    }
}

impl MyFrom<SmbNotifyChangeInformation> for SMBFileNotificationBoxed {
    fn my_from(value: SmbNotifyChangeInformation) -> Self {
        let not = SMBFileNotification2{notification: value };
        return Box::new(not);
    }
}

pub fn normalize_smb_path(path: &str) -> &str {
    let mut real_path = path;
    let real_path_replaced = real_path.strip_prefix("/");
    match real_path_replaced {
        Some(replaced_path) => {
            real_path = replaced_path;
        },
        None => {},
    }
    return real_path;
}

pub struct SMBDirectory2 {
    dir: libsmb2_rs::SmbDirectory,
}

impl SMBDirectory for SMBDirectory2 {
}

impl Debug for SMBDirectory2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SMBDirectory2").finish()
    }
}

impl Iterator for SMBDirectory2 {
    type Item = Result<SMBDirEntry>;

    fn next(&mut self) -> Option<Self::Item> {
        self.dir.next().map(|res| res.map(|entry| SMBDirEntry{
            //path: normalize_smb_path(entry.path.into_os_string().into_string().unwrap().as_str()).into(),
            path: entry.path.into_os_string().into_string().unwrap(),
            inode: entry.inode,
            d_type: (entry.d_type as u32).into(),
            size: entry.size,
            atime: Time{seconds: entry.atime as u32, nseconds: entry.atime_nsec},
            mtime: Time{seconds: entry.mtime as u32, nseconds: entry.mtime_nsec},
            ctime: Time{seconds: entry.ctime as u32, nseconds: entry.ctime_nsec},
            nlink: entry.nlink,
            atime_nsec: entry.atime_nsec,
            mtime_nsec: entry.mtime_nsec,
            ctime_nsec: entry.ctime_nsec,
        }))
    }
}

pub struct SMBFile2 {
    file: libsmb2_rs::SmbFile,
}

impl Debug for SMBFile2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SMBFile2").finish()
    }
}

impl SMBFile for SMBFile2 {
    fn fstat64(&self) -> Result<SMBStat64> {
        self.file.fstat64().map(|res| SMBStat64{
            ino: res.smb2_ino,
            nlink: res.smb2_nlink.into(),
            size: res.smb2_size,
            atime: res.smb2_atime,
            mtime: res.smb2_mtime,
            ctime: res.smb2_ctime,
            atime_nsec: res.smb2_atime_nsec,
            mtime_nsec: res.smb2_mtime_nsec,
            ctime_nsec: res.smb2_ctime_nsec,
        })
    }

    fn pread_into(&self, count: u32, offset: u64, buffer: &mut [u8]) -> Result<u32> {
        self.file.pread_into(count as u64, offset, buffer).map(|res| res as u32)
    }

    fn pwrite(&self, buffer: &[u8], offset: u64) -> Result<u32> {
        self.file.pwrite(buffer, offset).map(|res| res as u32)
    }
}


pub struct SMBFileNotification2 {
    notification: libsmb2_rs::SmbNotifyChangeInformation,
}

impl SMBFileNotification for SMBFileNotification2 {
}

impl Debug for SMBFileNotification2 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SMBFileNotification2").finish()
    }
}

impl Iterator for SMBFileNotification2 {
    type Item = Result<SMBFileNotificationInformation>;

    fn next(&mut self) -> Option<Self::Item> {
        self.notification.next().map(|res| res.map(|entry| SMBFileNotificationInformation{
            path: entry.path,
            operation: SMBFileNotificationOperation::my_from(entry.action),
        }))
    }

}