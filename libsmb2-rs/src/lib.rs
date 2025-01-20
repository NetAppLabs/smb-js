//! LIBsmb is a client library for accessing smb shares over a network
//! smbv3 is the default but smbv4 can be selected either by using the URL argument
//! version=4 or programatically calling smb2_set_version(smb, smb2_V4) before
//! connecting to the server/share.
//!
use libsmb2_sys::*;
use nix::fcntl::OFlag;
use nix::sys::stat::Mode;

use std::ffi::{c_void, CStr, CString};
use std::io::{Error, ErrorKind, Result};
use std::mem::zeroed;
use std::os::raw::c_char;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use bitflags::bitflags;


macro_rules! using_mutex {
    ( $mutex:expr ) => {
      $mutex.0.lock().unwrap()
    };
}  

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct SmbChangeNotifyFlags: u16 {
        const DEFAULT            = 0x0000;
        const WATCH_TREE            = 0x0001;
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct SmbChangeNotifyFileFilter: u32 {
        const CHANGE_FILE_NAME      = 0x00000001;
        const CHANGE_DIR_NAME       = 0x00000002;
        const CHANGE_ATTRIBUTES     = 0x00000004;
        const CHANGE_SIZE           = 0x00000008;
        const CHANGE_LAST_WRITE     = 0x00000010;
        const CHANGE_LAST_ACCESS    = 0x00000020;
        const CHANGE_CREATION       = 0x00000040;
        const CHANGE_EA             = 0x00000080;
        const CHANGE_SECURITY       = 0x00000100;
        const CHANGE_STREAM_NAME    = 0x00000200;
        const CHANGE_STREAM_SIZE    = 0x00000400;
        const CHANGE_STREAM_WRITE   = 0x00000800;
    }
}

#[derive(Clone)]
#[repr(u32)]
pub enum SmbChangeNotifyAction {
    Added = 1,
    Removed,
    Modified,
    RenamedOldName,
    RenamedNewName,
    AddedStream,
    RemovedStream,
    ModifiedStream
}


impl SmbChangeNotifyAction {
    fn from(action: u32) -> Result<SmbChangeNotifyAction> {
        match action {
            libsmb2_sys::SMB2_NOTIFY_CHANGE_FILE_ACTION_ADDED => Ok(SmbChangeNotifyAction::Added),
            libsmb2_sys::SMB2_NOTIFY_CHANGE_FILE_ACTION_REMOVED => Ok(SmbChangeNotifyAction::Removed),
            libsmb2_sys::SMB2_NOTIFY_CHANGE_FILE_ACTION_MODIFIED => Ok(SmbChangeNotifyAction::Modified),
            libsmb2_sys::SMB2_NOTIFY_CHANGE_FILE_ACTION_RENAMED_OLD_NAME => Ok(SmbChangeNotifyAction::RenamedOldName),
            libsmb2_sys::SMB2_NOTIFY_CHANGE_FILE_ACTION_RENAMED_NEW_NAME => Ok(SmbChangeNotifyAction::RenamedNewName),
            libsmb2_sys::SMB2_NOTIFY_CHANGE_FILE_ACTION_ADDED_STREAM => Ok(SmbChangeNotifyAction::AddedStream),
            libsmb2_sys::SMB2_NOTIFY_CHANGE_FILE_ACTION_REMOVED_STREAM => Ok(SmbChangeNotifyAction::RemovedStream),
            libsmb2_sys::SMB2_NOTIFY_CHANGE_FILE_ACTION_MODIFIED_STREAM => Ok(SmbChangeNotifyAction::ModifiedStream),
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Unknown action type: {}", action),
            )),
        }
    }
}


#[derive(Clone)]
struct SmbPtr(Arc<Mutex<*mut smb2_context>>);
// Safe because smb2_context in SmbPtr is enclosed within a Mutex
unsafe impl Send for SmbPtr{}
unsafe impl Sync for SmbPtr{}

impl Drop for SmbPtr {
    fn drop(&mut self) {
        let ctx_ref = using_mutex!(self);
        let ctx = *ctx_ref;
        if !ctx.is_null() {
            unsafe {
                smb2_destroy_context(ctx);
            }
        }
    }
}

fn check_mut_ptr<T>(ptr: *mut T) -> Result<*mut T> {
    if ptr.is_null() {
        Err(Error::last_os_error())
    } else {
        Ok(ptr)
    }
}

fn check_retcode(ctx: *mut smb2_context, code: i32) -> Result<()> {
    if code < 0 {
        unsafe {
            let err_str = smb2_get_error(ctx);
            let e = CStr::from_ptr(err_str).to_string_lossy().into_owned();
            Err(Error::new(ErrorKind::Other, e))
        }
    } else {
        Ok(())
    }
}

#[derive(Clone)]
pub struct Smb {
    context: Arc<SmbPtr>,
    base_path: Option<String>,
}

#[derive(Clone, Debug)]
pub enum EntryType {
    Block,
    Character,
    Directory,
    File,
    NamedPipe,
    Symlink,
    Socket,
}


impl EntryType {
    fn from(smb_type: u32) -> Result<EntryType> {
        match smb_type {
            libsmb2_sys::SMB2_TYPE_DIRECTORY => Ok(EntryType::Directory),
            libsmb2_sys::SMB2_TYPE_FILE => Ok(EntryType::File),
            libsmb2_sys::SMB2_TYPE_LINK => Ok(EntryType::Symlink),
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                format!("Unknown file type: {}", smb_type),
            )),
        }
    }
}


#[derive(Debug, Clone)]
pub struct DirEntry {
    pub path: PathBuf,
    pub inode: u64,
    pub d_type: EntryType,
    //pub mode: Mode,
    pub size: u64,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub nlink: u32,
    pub atime_nsec: u64,
    pub mtime_nsec: u64,
    pub ctime_nsec: u64,
}

#[derive(Clone)]
pub struct SmbDirectory {
    smb: Arc<SmbPtr>,
    handle: *mut smb2dir,
}

impl Drop for SmbDirectory {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                let ctx_ref = using_mutex!(self.smb);
                let ctx = *ctx_ref;
                smb2_closedir(ctx, self.handle);
            }
        }
    }
}

#[derive(Clone)]
pub struct SmbFile {
    smb: Arc<SmbPtr>,
    handle: *mut smb2fh,
}

impl Drop for SmbFile {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                let ctx_ref = using_mutex!(self.smb);
                let ctx = *ctx_ref;
                smb2_close(ctx, self.handle);
            }
        }
    }
}

extern "C" fn smb_notify_change_callback(ctx: *mut smb2_context, status: i32, info_handle: *mut c_void, cb_data: *mut c_void) {
    if status as u32 == SMB2_STATUS_CANCELLED {
        println!("smb_notify_change_callback - cancelled");
        return;
    }

    let cb_ptr = cb_data.cast::<NotifyChangeCallback>();
    let cb = unsafe { Box::from_raw(cb_ptr) };
    let change_handle = info_handle.cast::<smb2_file_notify_change_information>();
    let mut next = change_handle;
    while !next.is_null() {
        if let Ok((path, action)) = get_path_and_action_from_change_info(next) {
            cb.call(path, action);
            next = unsafe { (*next).next };
        }
    }
    unsafe { free_smb2_file_notify_change_information(ctx, change_handle); }
    std::mem::forget(cb); // XXX: prevent execution of NotifyChangeCallback::drop
}

pub trait SmbNotifyChangeCallback {
    fn call(&self, path: String, action: String);
}

struct NotifyChangeCallback {
    inner: Box<dyn SmbNotifyChangeCallback>,
    smb: Arc<SmbPtr>,
    fh: *mut smb2fh,
}

impl Drop for NotifyChangeCallback {
    fn drop(&mut self) {
        if !self.fh.is_null() {
            unsafe {
                let ctx_ref = using_mutex!(self.smb);
                let ctx = *ctx_ref;
                smb2_close(ctx, self.fh);
            }
        }
    }
}

impl SmbNotifyChangeCallback for NotifyChangeCallback {
    fn call(&self, path: String, action: String) {
        self.inner.call(path, action);
    }
}

fn get_path_and_action_from_change_info(change_info: *mut smb2_file_notify_change_information) -> Result<(String, String)> {
    let file_path = unsafe { CStr::from_ptr((*change_info).name) };
    let mut path = file_path.to_string_lossy().into_owned();
    path = path.replace("\\", "/");

    let int_action = unsafe { (*change_info).action };
    let enum_action = SmbChangeNotifyAction::from(int_action)?;
    let action = match enum_action {
        SmbChangeNotifyAction::Added => "create",
        SmbChangeNotifyAction::Removed => "remove",
        SmbChangeNotifyAction::Modified => "write",
        SmbChangeNotifyAction::RenamedOldName => "rename",
        SmbChangeNotifyAction::RenamedNewName => "rename",
        SmbChangeNotifyAction::AddedStream => "write",
        SmbChangeNotifyAction::RemovedStream => "write",
        SmbChangeNotifyAction::ModifiedStream => "write",
    }.to_string();

    Ok((path, action))
}

#[derive(Clone)]
pub struct NotifyChangeInformation {
    pub path: String,
    pub action: SmbChangeNotifyAction,
}

pub struct SmbUrl {
    url: *mut smb2_url,
}


impl Drop for SmbUrl {
    fn drop(&mut self) {
        if !self.url.is_null() {
            unsafe {
                smb2_destroy_url(self.url);
            }
        }
    }
}


impl Smb {
    pub fn new() -> Result<Self> {
        unsafe {
            let ctx = check_mut_ptr(smb2_init_context())?;
            Ok(Smb {
                context: Arc::new(SmbPtr(Arc::new(Mutex::new(ctx)))),
                base_path: None,
            })
        }
    }

    pub fn set_version(&self, version: u32) -> Result<()> {
        unsafe {
            let ctx_ref = using_mutex!(self.context);
            let ctx = *ctx_ref;
            smb2_set_version(ctx, version);
            Ok(())
        }
    }

    /*
    pub fn access(&self, path: &Path, mode: i32) -> Result<()> {
        let path = self.get_path_cstr(path)?;
        unsafe {
            let ctx_ref = using_mutex!(self.context);
            let ctx = *ctx_ref;
            check_retcode(
                ctx,
                smb2_access(ctx, path.as_ptr(), mode),
            )?;
            Ok(())
        }
    }

    pub fn access2(&self, path: &Path) -> Result<()> {
        let path = self.get_path_cstr(path)?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            check_retcode(ctx, smb2_access2(ctx, path.as_ptr()))?;
            Ok(())
        }
    }
    */
    /*
    pub fn chdir(&self, path: &Path) -> Result<()> {
        let path = self.get_path_cstr(path)?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            check_retcode(ctx, smb2_chdir(ctx, path.as_ptr()))?;
            Ok(())
        }
    }
    */
    /* 
    pub fn chown(&self, path: &Path, uid: i32, gid: i32) -> Result<()> {
        let path = self.get_path_cstr(path)?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            check_retcode(
                ctx,
                smb2_chown(ctx, path.as_ptr(), uid, gid),
            )?;
            Ok(())
        }
    }
    */

    /// Supported flags:
    /// O_APPEND
    /// O_SYNC
    /// O_EXCL
    /// O_TRUNC
    pub fn create(&mut self, path: &Path, flags: OFlag, _mode: Mode) -> Result<SmbFile> {
        let path = self.get_resolved_path_cstr(path)?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            let mut smb_flags = flags;
            smb_flags.insert(OFlag::O_CREAT);
            let file_handle = smb2_open(ctx, path.as_ptr(), smb_flags.bits());
            if file_handle.is_null() {
                check_retcode(ctx, -1)?
            }
            Ok(SmbFile {
                smb: Arc::clone(&self.context),
                handle: file_handle,
            })
        }
    }

    pub fn notify_change(&self, path: &Path, notify_flags: SmbChangeNotifyFlags, filter: SmbChangeNotifyFileFilter, cb: Box<dyn SmbNotifyChangeCallback>) {
        let path = self.get_resolved_path_cstr(path).unwrap();
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            let fh = smb2_open(ctx, path.as_ptr(), libc::O_DIRECTORY);
            if fh.is_null() {
                println!("Smb notify_change_async - smb2_open returned null - Error::last_os_error() = {:?}", Error::last_os_error());
                return;
            }
            let cb_data = Box::new(NotifyChangeCallback{inner: cb, smb: Arc::clone(&self.context), fh});
            let cb_data_ptr = Box::into_raw(cb_data);
            smb2_notify_change_filehandle_async(ctx, fh, notify_flags.bits(), filter.bits(), 1, Some(smb_notify_change_callback), cb_data_ptr.cast::<c_void>());

            let pfd = Box::new(libc::pollfd{
                fd: smb2_get_fd(ctx),
                events: 0,
                revents: 0,
            });
            let pfd_ptr = Box::into_raw(pfd);
            loop {
                (*pfd_ptr).events = smb2_which_events(ctx) as libc::c_short;
                let ret = libc::poll(pfd_ptr, 1, -1);
                if ret < 0 {
                    println!("Smb notify_change_async - called libc::poll - ret = {:?}", ret);
                    break;
                }
                if (*pfd_ptr).revents != 0 {
                    let ret = smb2_service(ctx, (*pfd_ptr).revents.into());
                    if ret < 0 {
                        println!("Smb notify_change_async - called smb2_service - ret = {:?}", ret);
                        break;
                    }
                }
            }
        }
    }

    /*
    pub fn getcwd(&self) -> Result<PathBuf> {
        let mut cwd = ptr::null();
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            smb2_getcwd(ctx, &mut cwd);
            let path_tmp = CStr::from_ptr(cwd).to_string_lossy().into_owned();

            Ok(PathBuf::from(path_tmp))
        }
    }
    */

    /* 
    /// Get the maximum supported READ3 size by the server
    pub fn get_readmax(&self) -> Result<u64> {
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            let max = smb2_get_readmax(ctx) as u64;
            Ok(max)
        }
    }

    /// Get the maximum supported WRITE3 size by the server
    pub fn get_writemax(&self) -> Result<u64> {
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            let max = smb2_get_writemax(ctx) as u64;
            Ok(max)
        }
    }
    */
    /*
    pub fn lchmod(&self, path: &Path, mode: Mode) -> Result<()> {
        let path = self.get_path_cstr(path)?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            check_retcode(
                ctx,
                smb2_lchmod(ctx, path.as_ptr(), mode.bits() as c_int),
            )?;
            Ok(())
        }
    }
    
    pub fn lchown(&self, path: &Path, uid: i32, gid: i32) -> Result<()> {
        let path = self.get_path_cstr(path)?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            check_retcode(
                ctx,
                smb2_lchown(ctx, path.as_ptr(), uid, gid),
            )?;
            Ok(())
        }
    }
    */
    /*
    pub fn link(&self, oldpath: &Path, newpath: &Path) -> Result<()> {
        let old_path = CString::new(oldpath.as_os_str().as_bytes())?;
        let new_path = CString::new(newpath.as_os_str().as_bytes())?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;

        unsafe {
            check_retcode(
                ctx,
                smb2_link(ctx, old_path.as_ptr(), new_path.as_ptr()),
            )?;
            Ok(())
        }
    }
    */

    /*
    pub fn lstat64(&self, path: &Path) -> Result<smb2_stat_64> {
        let path = self.get_path_cstr(path)?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            let mut stat_buf: smb2_stat_64 = zeroed();
            check_retcode(
                ctx,
                smb2_lstat64(ctx, path.as_ptr(), &mut stat_buf),
            )?;
            Ok(stat_buf)
        }
    }
    */

    pub fn mkdir(&self, path: &Path) -> Result<()> {
        let path = self.get_resolved_path_cstr(path)?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            check_retcode(ctx, smb2_mkdir(ctx, path.as_ptr()))?;
            Ok(())
        }
    }
    /*
    pub fn mknod(&self, path: &Path, mode: i32, dev: i32) -> Result<()> {
        let path = self.get_path_cstr(path)?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            check_retcode(
                ctx,
                smb2_mknod(ctx, path.as_ptr(), mode, dev),
            )?;
            Ok(())
        }
    }
    */

    pub fn set_user(&self, user: &str) -> Result<()> {
        let user = CString::new(user.as_bytes())?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            smb2_set_user(ctx, user.as_ptr());
            Ok(())
        }
    }

    pub fn set_password(&self, password: &str) -> Result<()> {
        let password = CString::new(password.as_bytes())?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            smb2_set_password(ctx, password.as_ptr());
            Ok(())
        }
    }

    pub fn set_domain(&self, domain: &str) -> Result<()> {
        let domain = CString::new(domain.as_bytes())?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            smb2_set_domain(ctx, domain.as_ptr());
            Ok(())
        }
    }
    
    pub fn connect_share(&self, server: &str, share: &str, user: &str) -> Result<()> {
        let server = CString::new(server.as_bytes())?;
        let share = CString::new(share.as_bytes())?;
        let user = CString::new(user.as_bytes())?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            check_retcode(
                ctx,
                smb2_connect_share(ctx, server.as_ptr(), share.as_ptr(), user.as_ptr()),
            )?;
            Ok(())
        }
    }
    

    /// Supported flags are
    /// O_APPEND
    /// O_RDONLY
    /// O_WRONLY
    /// O_RDWR
    /// O_SYNC
    /// O_TRUNC (Only valid with O_RDWR or O_WRONLY. Ignored otherwise.)
    pub fn open(&mut self, path: &Path, flags: OFlag) -> Result<SmbFile> {

        let path = self.get_resolved_path_cstr(path)?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            let file_handle = smb2_open(
                    ctx,
                    path.as_ptr(),
                    flags.bits(),
                );
            if file_handle.is_null() {
                check_retcode(ctx, -1)?
            }
            Ok(SmbFile {
                smb: Arc::clone(&self.context),
                handle: file_handle,
            })
        }
    }

    pub fn opendir(&mut self, path: &Path) -> Result<SmbDirectory> {
        let cpath = self.get_resolved_path_cstr(path)?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            let dir_handle = check_mut_ptr(smb2_opendir(ctx, cpath.as_ptr()))?;
            if dir_handle.is_null() {
                check_retcode(ctx, -1)?
            }
            Ok(SmbDirectory {
                smb: Arc::clone(&self.context),
                handle: dir_handle,
            })
        }
    }

    /// Parse an smb URL, but do not split path and file. File
    /// in the resulting struct remains NULL.
    pub fn parse_url_dir(&mut self, url: &str) -> Result<SmbUrl> {
        let url = CString::new(url.as_bytes())?;
        unsafe {
            let ctx_ref = using_mutex!(self.context);
            let ctx = *ctx_ref;
            let smb2_url = check_mut_ptr(smb2_parse_url(ctx, url.as_ptr()))?;
            Ok(SmbUrl {
                url: smb2_url,
            })
        }
    }

    /// Parse an smb URL, but do not fail if file, path or even server is missing.
    /// Check elements of the resulting struct for NULL.
    pub fn parse_url_incomplete(&mut self, url: &str) -> Result<SmbUrl> {
        let url: CString = CString::new(url.as_bytes())?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            let smb2_url = check_mut_ptr(smb2_parse_url(ctx, url.as_ptr()))?;
            Ok(SmbUrl {
                url: smb2_url,
            })
        }
    }

    /// URL parsing functions.
    /// These functions all parse a URL of the form
    /// smb://server/path/file?argv=val[&arg=val]*
    /// and returns a smb2_url.
    ///
    /// Apart from parsing the URL the functions will also update
    /// the smb context to reflect settings controlled via url arguments.
    ///
    /// Current URL arguments are :
    /// tcp-syncnt=<int>  : Number of SYNs to send during the seccion establish
    ///                     before failing settin up the tcp connection to the
    ///                     server.
    /// uid=<int>         : UID value to use when talking to the server.
    ///                     default it 65534 on Windows and getuid() on unixen.
    /// gid=<int>         : GID value to use when talking to the server.
    ///                     default it 65534 on Windows and getgid() on unixen.
    /// readahead=<int>   : Enable readahead for files and set the maximum amount
    ///                     of readahead to <int>.
    ///
    /// Parse a complete smb URL including, server, path and
    /// filename. Fail if any component is missing.
    pub fn parse_url_full(&mut self, url: &str) -> Result<SmbUrl> {
        let url = CString::new(url.as_bytes())?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            let smb2_url = check_mut_ptr(smb2_parse_url(ctx, url.as_ptr()))?;
            Ok(SmbUrl {
                url: smb2_url,
            })
        }
    }
    

    pub fn parse_url_mount(&mut self, url: &str, user: Option<String>, password: Option<String>, domain: Option<String>) -> Result<()> {
        unsafe {
            match user {
                Some(user_string) => {
                    let ustr = user_string.as_str();
                    let _ = self.set_user(ustr);
                },
                None => {},
            };
            match password {
                Some(password_string) => {
                    let pstr = password_string.as_str();
                    let _ = self.set_password(pstr);
                },
                None => {},
            };
            match domain {
                Some(domain_string) => {
                    let dstr = domain_string.as_str();
                    let _ = self.set_domain(dstr);
                },
                None => {},
            };
            let n_url = self.parse_url_full(url)?;
            let url = *n_url.url;
            let server = url.server;
            let share = url.share;
            let user = url.user;
            let ctx_ref = using_mutex!(self.context);
            let cpath = url.path;
            if !cpath.is_null() {
                let pathcstr: &CStr = CStr::from_ptr(cpath);
                let pathstr = pathcstr.to_str();
                match pathstr {
                    Ok(pathstr) => {
                        if pathstr != "" {
                            self.base_path = Some(pathstr.into());
                        }        
                    },
                    Err(e) => {
                        return Err(Error::new(ErrorKind::Other, e));
                    },
                }
            }
            let ctx = *ctx_ref;
            check_retcode(
                ctx,
                smb2_connect_share(ctx, server, share, user),
            )?;
            Ok(())
        }
    }

    pub fn get_resolved_path_cstr(&self, path: &Path) -> Result<CString> {
        let mut real_path = path;
        match &self.base_path {
            Some(parent_path) => {
                let path_parent_path = Path::new(parent_path.as_str());
                let real_path_pathbuf = path_parent_path.join(PathBuf::from(path));
                real_path = real_path_pathbuf.as_path();
                let path = CString::new(real_path.as_os_str().as_bytes())?;
                return Ok(path);
    
            },
            None => {},
        }
        let path = CString::new(real_path.as_os_str().as_bytes())?;
        return Ok(path);
    }

    pub fn readlink(&self, path: &Path, buf: &mut [u8]) -> Result<()> {
        let path = self.get_resolved_path_cstr(path)?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;

        unsafe {
            check_retcode(
                ctx,
                smb2_readlink(
                    ctx,
                    path.as_ptr(),
                    buf.as_mut_ptr() as *mut c_char,
                    buf.len() as u32,
                ),
            )?;
            Ok(())
        }
    }

    pub fn rename(&self, oldpath: &Path, newpath: &Path) -> Result<()> {
        let old_path = CString::new(oldpath.as_os_str().as_bytes())?;
        let new_path = CString::new(newpath.as_os_str().as_bytes())?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            check_retcode(
                ctx,
                smb2_rename(ctx, old_path.as_ptr(), new_path.as_ptr()),
            )?;
            Ok(())
        }
    }

    pub fn rmdir(&self, path: &Path) -> Result<()> {
        let path = self.get_resolved_path_cstr(path)?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            check_retcode(ctx, smb2_rmdir(ctx, path.as_ptr()))?;
            Ok(())
        }
    }

    pub fn set_auth(&self, auth: i32) -> Result<()> {
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            // SMB2_SEC_UNDEFINED
            smb2_set_authentication(ctx, auth);
        }
        Ok(())
    }


    pub fn stat64(&self, path: &Path) -> Result<smb2_stat_64> {
        let path = self.get_resolved_path_cstr(path)?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            let mut stat_buf: smb2_stat_64 = zeroed();
            check_retcode(
                ctx,
                smb2_stat(ctx, path.as_ptr(), &mut stat_buf),
            )?;
            Ok(stat_buf)
        }
    }

    pub fn statvfs(&self, path: &Path) -> Result<smb2_statvfs> {
        let path = self.get_resolved_path_cstr(path)?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            let mut stat_buf: smb2_statvfs = zeroed();
            check_retcode(
                ctx,
                smb2_statvfs(ctx, path.as_ptr(), &mut stat_buf),
            )?;
            Ok(stat_buf)
        }
    }

    /*
    pub fn symlink(&self, oldpath: &Path, newpath: &Path) -> Result<()> {
        let old_path = CString::new(oldpath.as_os_str().as_bytes())?;
        let new_path = CString::new(newpath.as_os_str().as_bytes())?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            check_retcode(
                ctx,
                smb2_symlink(ctx, old_path.as_ptr(), new_path.as_ptr()),
            )?;
            Ok(())
        }
    }
    */

    pub fn truncate(&self, path: &Path, len: u64) -> Result<()> {
        let path = self.get_resolved_path_cstr(path)?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            check_retcode(
                ctx,
                smb2_truncate(ctx, path.as_ptr(), len),
            )?;
            Ok(())
        }
    }

    pub fn unlink(&self, path: &Path) -> Result<()> {
        let path = self.get_resolved_path_cstr(path)?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            check_retcode(ctx, smb2_unlink(ctx, path.as_ptr()))?;
            Ok(())
        }
    }

    /*
    // Set the access and modified times
    pub fn utimes(&self, path: &Path, times: &mut [timeval; 2]) -> Result<()> {
        let path = self.get_path_cstr(path)?;
        let ctx_ref = using_mutex!(self.context);
        let ctx = *ctx_ref;
        unsafe {
            check_retcode(
                ctx,
                smb2_utimes(ctx, path.as_ptr(), times.as_mut_ptr()),
            )?;
            Ok(())
        }
    }
    */
}

impl SmbFile {
    /*pub fn fchmod(&self, mode: i32) -> Result<()> {
        let ctx_ref = using_mutex!(self.smb);
        let ctx = *ctx_ref;
        unsafe {
            check_retcode(ctx, smb2_fchmod(ctx, self.handle, mode))?;

            Ok(())
        }
    }

    pub fn fchown(&self, uid: i32, gid: i32) -> Result<()> {
        let ctx_ref = using_mutex!(self.smb);
        let ctx = *ctx_ref;
        unsafe {
            check_retcode(ctx, smb2_fchown(ctx, self.handle, uid, gid))?;
            Ok(())
        }
    }
    */

    pub fn ftruncate(&self, len: u64) -> Result<()> {
        let ctx_ref = using_mutex!(self.smb);
        let ctx = *ctx_ref;
        unsafe {
            check_retcode(ctx, smb2_ftruncate(ctx, self.handle, len))?;
            Ok(())
        }
    }

    /// 64 bit version of fstat. All fields are always 64bit.
    pub fn fstat64(&self) -> Result<smb2_stat_64> {
        let ctx_ref = using_mutex!(self.smb);
        let ctx = *ctx_ref;
        unsafe {
            let mut stat_buf: smb2_stat_64 = zeroed();
            check_retcode(
                ctx,
                smb2_fstat(ctx, self.handle, &mut stat_buf),
            )?;
            Ok(stat_buf)
        }
    }

    pub fn fsync(&self) -> Result<()> {
        let ctx_ref = using_mutex!(self.smb);
        let ctx = *ctx_ref;
        unsafe {
            check_retcode(ctx, smb2_fsync(ctx, self.handle))?;
            Ok(())
        }
    }

    pub fn pread(&self, count: u64, offset: u64) -> Result<Vec<u8>> {
        let mut buffer: Vec<u8> = Vec::with_capacity(count as usize);
        let read_size = self.pread_into(count, offset, &mut buffer)?;
        unsafe {
            buffer.set_len(read_size as usize);
        }
        Ok(buffer)
    }

    pub fn pread_into(&self, count: u64, offset: u64, buffer: &mut [u8]) -> Result<i32> {
        let ctx_ref = using_mutex!(self.smb);
        let ctx = *ctx_ref;
        unsafe {
            let read_size = smb2_pread(
                ctx,
                self.handle,
                buffer.as_mut_ptr() as *mut _,
                count as u32,
                offset,
            );
            check_retcode(ctx, read_size)?;
            Ok(read_size)
        }
    }

    pub fn pwrite(&self, buffer: &[u8], offset: u64) -> Result<i32> {
        let ctx_ref = using_mutex!(self.smb);
        let ctx = *ctx_ref;
        unsafe {
            let write_size = smb2_pwrite(
                ctx,
                self.handle,
                buffer.as_ptr() as *mut _,
                buffer.len() as u32,
                offset,
            );
            check_retcode(ctx, write_size)?;
            Ok(write_size)
        }
    }

    pub fn read(&self, count: u64) -> Result<Vec<u8>> {
        self.pread(count, 0)
    }

    pub fn write(&self, buffer: &[u8]) -> Result<i32> {
        self.pwrite(buffer, 0)
    }

    /*
    pub fn lseek(&self, offset: i64, whence: i32, current_offset: u64) -> Result<()> {
        unsafe {
            check_retcode(ctx.smb, smb2_lseek(*self.smb.context, self.handle, offset, whence, current_offset))?;
            Ok(())
        }
    }
    */
}

impl Iterator for SmbDirectory {
    type Item = Result<DirEntry>;
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let ctx_ref = using_mutex!(self.smb);
            let ctx = *ctx_ref;
            let dirent = smb2_readdir(ctx, self.handle);
            if dirent.is_null() {
                return None;
            }

            let file_name = CStr::from_ptr((*dirent).name);
            let stat = (*dirent).st;

            let d_type = match EntryType::from(stat.smb2_type) {
                Ok(ty) => ty,
                Err(e) => {
                    return Some(Err(e));
                }
            };
            //let mode = Mode::from_bits_truncate(((*dirent).mode as u16).into());
            Some(Ok(DirEntry {
                path: PathBuf::from(file_name.to_string_lossy().into_owned()),
                inode: (stat).smb2_ino,
                d_type,
                //mode,
                size: (stat).smb2_size,
                atime: (stat).smb2_atime,
                mtime: (stat).smb2_mtime,
                ctime: (stat).smb2_ctime,
                nlink: (stat).smb2_nlink,
                atime_nsec: (stat).smb2_atime_nsec,
                mtime_nsec: (stat).smb2_mtime_nsec,
                ctime_nsec: (stat).smb2_ctime_nsec,
            }))
        }
    }
}
