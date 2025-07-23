#![deny(clippy::all)]
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


use enumflags2::BitFlag;
use napi::{bindgen_prelude::*, threadsafe_function::{ErrorStrategy, ThreadSafeCallContext, ThreadsafeFunction, ThreadsafeFunctionCallMode}, JsArrayBuffer, JsDataView, JsString, JsTypedArray, NapiRaw};
use napi_derive::napi;
use nix::sys::stat::Mode;
use send_wrapper::SendWrapper;
use std::{path::Path, sync::{mpsc::{channel, Receiver, Sender}, Arc, RwLock, RwLockWriteGuard}, thread};

mod smb;
use smb::{VFSEntryType, VFSFileNotificationOperation, VFSNotifyChangeCallback, VFSWatchMode, VFS};

use crate::smb::VFSStat;

/*

See https://wicg.github.io/file-system-access/
And https://developer.mozilla.org/en-US/docs/Web/API/File_System_Access_API
and https://web.dev/file-system-access/


// Example use:
const smb_url = 'smb://1.2.3.4/export?vers=3';
const rootHandle = new SMBDirectoryHandle(smb_url);

for await (const [name, entry] of rootHandle) {
  console.log('FileName: ', name, 'Entry: ', entry);
}

const fileHandle = await rootHandle.getFileHandle('testfile.txt', { create: true });
const wfs = await fileHandle.createWritable({ keepExistingData: false });
await wfs.write('Hello from Javascript');
await wfs.close();

// Interface definition

type FileSystemHandlePermissionMode = 'read' | 'readwrite';
type FileSystemHandleKind = 'directory' | 'file';

interface FileSystemHandlePermissionDescriptor {
    mode: FileSystemHandlePermissionMode;
}

interface FileSystemHandle {
    readonly kind: FileSystemHandleKind;
    readonly name: string;
    isSameEntry(other: FileSystemHandle): boolean;
    queryPermission(perm: FileSystemHandlePermissionDescriptor): Promise<String>;
    requestPermission(perm: FileSystemHandlePermissionDescriptor): Promise<String>;
}

interface FileSystemGetDirectoryOptions {
    create: boolean;
}

interface FileSystemGetFileOptions {
    create: boolean;
}

interface FileSystemRemoveOptions {
    recursive: boolean;
}

interface FileSystemDirectoryHandle extends FileSystemHandle {
    readonly kind: 'directory';
    getDirectoryHandle(name: string, options?: FileSystemGetDirectoryOptions): Promise<FileSystemDirectoryHandle>;
    getFileHandle(name: string, options?: FileSystemGetFileOptions): Promise<FileSystemFileHandle>;
    removeEntry(name: string, options?: FileSystemRemoveOptions): Promise<void>;
    resolve(possibleDescendant: FileSystemHandle): Promise<string[] | null>;
    keys(): AsyncIterableIterator<string>;
    values(): AsyncIterableIterator<FileSystemDirectoryHandle | FileSystemFileHandle>;
    entries(): AsyncIterableIterator<[string, FileSystemDirectoryHandle | FileSystemFileHandle]>;
    [Symbol.asyncIterator]: FileSystemDirectoryHandle['entries'];
}

interface FileSystemCreateWritableOptions {
    keepExistingData: boolean;
}

interface FileSystemFileHandle extends FileSystemHandle {
    readonly kind: 'file';
    getFile(): Promise<File>;
    createWritable(options?: FileSystemCreateWritableOptions): Promise<FileSystemWritableFileStream>;
}

interface FileSystemWritableFileStream extends WritableStream {
    readonly locked: true;                    // from WritableStream
    abort(reason: string): Promise<string>;   // from WritableStream
    close(): Promise<void>;                   // from WritableStream
    getWriter(): WritableStreamDefaultWriter; // from WritableStream
    write(data: ArrayBuffer | TypedArray | DataView | Blob | String | string | {type: 'write' | 'seek' | 'truncate', data?: ArrayBuffer | TypedArray | DataView | Blob | String | string, position?: number, size?: number}): Promise<void>;
    seek(position: number): Promise<void>;
    truncate(size: number): Promise<void>;
}

interface File extends Blob {
    readonly lastModified: number;
    readonly name: string;
}

interface Blob {
    readonly size: number;
    readonly type: string;
    arrayBuffer(): Promise<ArrayBuffer>;
    slice(start?: number, end?: number, contentType?: string): Blob;
    stream(): ReadableStream<Uint8Array>;
    text(): Promise<string>;
}

*/

const FIELD_KIND: &str = "kind";
const FIELD_NAME: &str = "name";
const FIELD_URL: &str = "url";
const FIELD_PATH: &str = "path";
const FIELD_DATA: &str = "data";
const FIELD_TYPE: &str = "type";
const FIELD_SIZE: &str = "size";
const FIELD_CLOSE: &str = "close";
const FIELD_LENGTH: &str = "length";
const FIELD_BUFFER: &str = "buffer";
const FIELD_ENQUEUE: &str = "enqueue";
const FIELD_POSITION: &str = "position";
const FIELD_SUBSTRING: &str = "substring";
const FIELD_BYTE_LENGTH: &str = "byteLength";

const KIND_FILE: &str = "file";
const KIND_DIRECTORY: &str = "directory";

//const PERM_READ: &str = "read";
//const PERM_READWRITE: &str = "readwrite";

const PERM_STATE_GRANTED: &str = "granted";
//const PERM_STATE_DENIED: &str = "denied";
const _PERM_STATE_PROMPT: &str = "prompt";

const WRITE_TYPE_WRITE: &str = "write";
const WRITE_TYPE_SEEK: &str = "seek";
const WRITE_TYPE_TRUNCATE: &str = "truncate";

const DIR_ROOT: &str = "/";

const DIR_CURRENT: &str = ".";
const DIR_PARENT: &str = "..";

const MIME_TYPE_UNKNOWN: &str = "unknown";

const JS_TYPE_BLOB: &str = "Blob";
const JS_TYPE_READABLE_STREAM: &str = "ReadableStream";
const JS_TYPE_WRITABLE_STREAM: &str = "WritableStream";
const JS_TYPE_WRITABLE_STREAM_DEFAULT_WRITER: &str = "WritableStreamDefaultWriter";

const READABLE_STREAM_SOURCE_TYPE_BYTES: &str = "bytes";

macro_rules! using_rwlock {
  ( $rwlock:expr ) => {
    $rwlock.as_ref().expect("error acquiring smb").write().unwrap()
  };
}

#[napi(iterator)]
pub struct JsSmbDirectoryHandleEntries {
  #[napi(js_name="[Symbol.asyncIterator]", ts_type="AsyncIterableIterator<[string, JsSmbDirectoryHandle | JsSmbFileHandle]>")]
  pub _sym: bool, // unused fake member, just to so that generated JsSmbDirectoryHandleEntries class specifies `[Symbol.asyncIterator]: AsyncIterableIterator<[string, JsSmbDirectoryHandle | JsSmbFileHandle]>`
  env: SendWrapper<Env>,
  entries: Vec<JsSmbHandle>,
  count: usize
}

impl Generator for JsSmbDirectoryHandleEntries {

  type Yield = Vec<Unknown>;

  type Next = ();

  type Return = ();

  fn next(&mut self, _: Option<Self::Next>) -> Option<Self::Yield> {
    if self.entries.len() <= self.count {
      return None;
    }
    let entry = &self.entries[self.count];
    let mut res = Vec::new();
    res.push(self.env.create_string(entry.name.as_str()).ok()?.into_unknown());
    match entry.kind.as_str() {
      KIND_DIRECTORY => unsafe { res.push(Unknown::from_napi_value(self.env.raw(), JsSmbDirectoryHandle::from(entry.to_owned()).into_instance(*self.env).ok()?.raw()).ok()?) },
      _ => unsafe { res.push(Unknown::from_napi_value(self.env.raw(), JsSmbFileHandle::from(entry.to_owned()).into_instance(*self.env).ok()?.raw()).ok()?) },
    };
    self.count += 1;
    Some(res)
  }
}

#[napi(iterator)]
pub struct JsSmbDirectoryHandleKeys {
  #[napi(js_name="[Symbol.asyncIterator]", ts_type="AsyncIterableIterator<string>")]
  pub _sym: bool, // unused fake member, just to so that generated JsSmbDirectoryHandleKeys class specifies `[Symbol.asyncIterator]: AsyncIterableIterator<string>`
  entries: Vec<JsSmbHandle>,
  count: usize
}

impl Generator for JsSmbDirectoryHandleKeys {

  type Yield = String;

  type Next = ();

  type Return = ();

  fn next(&mut self, _: Option<Self::Next>) -> Option<Self::Yield> {
    if self.entries.len() <= self.count {
      return None;
    }
    let entry = &self.entries[self.count];
    let res = entry.name.to_owned();
    self.count += 1;
    Some(res)
  }
}

#[napi(iterator)]
pub struct JsSmbDirectoryHandleValues {
  #[napi(js_name="[Symbol.asyncIterator]", ts_type="AsyncIterableIterator<JsSmbDirectoryHandle | JsSmbFileHandle>")]
  pub _sym: bool, // unused fake member, just to so that generated JsSmbDirectoryHandleValues class specifies `[Symbol.asyncIterator]: AsyncIterableIterator<JsSmbDirectoryHandle | JsSmbFileHandle>`
  entries: Vec<JsSmbHandle>,
  count: usize
}

impl Generator for JsSmbDirectoryHandleValues {

  type Yield = Either<JsSmbDirectoryHandle, JsSmbFileHandle>;

  type Next = ();

  type Return = ();

  fn next(&mut self, _: Option<Self::Next>) -> Option<Self::Yield> {
    if self.entries.len() <= self.count {
      return None;
    }
    let entry = &self.entries[self.count];
    let res = match entry.kind.as_str() {
      KIND_DIRECTORY => Either::A(JsSmbDirectoryHandle::from(entry.to_owned())),
      _ => Either::B(JsSmbFileHandle::from(entry.to_owned()))
    };
    self.count += 1;
    Some(res)
  }
}

#[napi(object)]
pub struct JsSmbHandlePermissionDescriptor {
  #[napi(ts_type="'read' | 'readwrite'")]
  pub mode: String
}

/*
impl JsSmbHandlePermissionDescriptor {

  fn to_mode(&self, kind: &str) -> Mode {
    match (kind, self.mode.as_str()) {
      (KIND_DIRECTORY, PERM_READWRITE) => Mode::S_IRWXU | Mode::S_IRWXG, // 770
      (KIND_DIRECTORY, PERM_READ) => Mode::S_IRUSR | Mode::S_IXUSR | Mode::S_IRGRP | Mode::S_IXGRP, // 550
      (KIND_FILE, PERM_READWRITE) => Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IRGRP | Mode::S_IWGRP, // 660
      _ => Mode::S_IRUSR | Mode::S_IRGRP // 440
    }
  }

  fn to_u64(&self, kind: &str) -> u64 {
    self.to_mode(kind).bits().into()
  }
}
*/

#[napi(object)]
pub struct JsSmbGetDirectoryOptions {
  pub create: bool
}

impl Default for JsSmbGetDirectoryOptions {

  fn default() -> Self {
    Self{create: Default::default()}
  }
}

#[napi(object)]
pub struct JsSmbGetFileOptions {
  pub create: bool
}

impl Default for JsSmbGetFileOptions {

  fn default() -> Self {
    Self{create: Default::default()}
  }
}

#[napi(object)]
pub struct JsSmbRemoveOptions {
  pub recursive: bool
}

impl Default for JsSmbRemoveOptions {

  fn default() -> Self {
    Self{recursive: Default::default()}
  }
}

#[napi(object)]
pub struct JsSmbCreateWritableOptions {
  pub keep_existing_data: bool
}

impl Default for JsSmbCreateWritableOptions {

  fn default() -> Self {
    Self{keep_existing_data: Default::default()}
  }
}

#[napi(object)]
pub struct JsSmbStat {
  #[napi(readonly, ts_type="bigint")]
  pub inode: Option<i64>,
  #[napi(readonly, ts_type="bigint")]
  pub size: i64,
  #[napi(readonly, ts_type="bigint")]
  pub creation_time: i64,
  #[napi(readonly, ts_type="bigint")]
  pub modified_time: i64,
  #[napi(readonly, ts_type="bigint")]
  pub accessed_time: i64
}

impl From<VFSStat> for JsSmbStat {
  fn from(value: VFSStat) -> Self {
    JsSmbStat {
      inode: (value.ino != 0).then_some(value.ino as i64),
      size: value.size as i64,
      creation_time: ((value.btime * 1_000_000_000) + value.btime_nsec) as i64,
      modified_time: ((value.mtime * 1_000_000_000) + value.mtime_nsec) as i64,
      accessed_time: ((value.atime * 1_000_000_000) + value.atime_nsec) as i64,
    }
  }
}

#[derive(Clone)]
#[napi]
pub struct JsSmbHandle {
  smb: Option<Arc<RwLock<Box<dyn VFS>>>>,
  url: String,
  path: String,
  #[napi(readonly, ts_type="'directory' | 'file'")]
  pub kind: String,
  #[napi(readonly)]
  pub name: String
}

#[napi]
impl JsSmbHandle {

  pub fn open(url: String) -> Result<Self> {
    Self::open_path(url, DIR_ROOT.into(), KIND_DIRECTORY.into(), DIR_ROOT.into())
  }

  fn open_path(url: String, path: String, kind: String, name: String) -> Result<Self> {
    let conn_res = smb::connect(url.to_owned());
    match conn_res {
      Ok(conn) => {
        return Ok(Self{smb: Some(Arc::new(RwLock::new(conn))), url, path, kind, name});
      },
      Err(e) => {
        return Err(e.into())
      },
    }
  }

  fn clone_with_new_connection(&self) -> Result<Self> {
    Self::open_path(self.url.to_owned(), self.path.to_owned(), self.kind.to_owned(), self.name.to_owned())
  }

  fn is_same(&self, other: &JsSmbHandle) -> bool {
    other.kind == self.kind && other.name == self.name && (other.path.is_empty() || self.path.is_empty() || other.path == self.path)
  }

  #[napi]
  pub fn is_same_entry(&self, other: &JsSmbHandle) -> Result<bool> {
    Ok(self.is_same(other))
  }

  #[napi]
  pub async fn query_permission(&self, _perm: JsSmbHandlePermissionDescriptor) -> Result<String> {
    /*if let Some(smb) = &self.smb {
      let my_smb = using_rwlock!(smb);
      let smb_stat = my_smb.stat64(self.path.as_str())?;
      let perm_u64 = perm.to_u64(self.kind.as_str());
      if smb_stat.mode & perm_u64 == perm_u64 {
        return Ok(PERM_STATE_GRANTED.into());
      }
    }*/
    return Ok(PERM_STATE_GRANTED.into());
    /*if self.smb.is_none() && ((self.name != "3" && self.name != "quatre") || perm.mode != PERM_READWRITE) {
      return Ok(PERM_STATE_GRANTED.into());
    }
    Ok(PERM_STATE_DENIED.into())
    */
  }

  #[napi]
  pub async fn request_permission(&self, perm: JsSmbHandlePermissionDescriptor) -> Result<String> {
    /*if let Some(smb) = &self.smb {
      let my_smb = using_rwlock!(smb);
      let smb_stat = my_smb.stat64(self.path.as_str())?;
      let perm_u64 = perm.to_u64(self.kind.as_str());
      if smb_stat.mode & perm_u64 == perm_u64 {
        return Ok(PERM_STATE_GRANTED.into());
      }
      let mode = perm.to_mode(self.kind.as_str()).union(Mode::from_bits_truncate((smb_stat.mode as u16).into()));
      if !my_smb.lchmod(self.name.as_str(), mode.bits() as u32).is_ok() {
        return Ok(PERM_STATE_DENIED.into());
      }
    }*/
    self.query_permission(perm).await
  }

  #[napi]
  pub async fn stat(&self) -> Result<JsSmbStat> {
    let smb = &self.smb;
    let my_smb = using_rwlock!(smb);
    let smb_stat = my_smb.stat(&self.path)?;
    Ok(smb_stat.into())
  }
}

impl FromNapiValue for JsSmbHandle {

  unsafe fn from_napi_value(env: sys::napi_env, napi_val: sys::napi_value) -> Result<Self> {
    Self::from_napi_ref(env, napi_val)
      .map_or_else(
      |err| {
        if err.status != Status::InvalidArg || err.reason != "Failed to recover `JsSmbHandle` type from napi value" {
          return Err(err);
        }
        let obj = Object::from_napi_value(env, napi_val)?;
        let kind = obj.get::<&str, &str>(FIELD_KIND)?.unwrap_or_default().into();
        let name = obj.get::<&str, &str>(FIELD_NAME)?.unwrap_or_default().into();
        let url = obj.get::<&str, &str>(FIELD_URL)?.unwrap_or_default().into();
        let path = obj.get::<&str, &str>(FIELD_PATH)?.unwrap_or_default().into();
        Ok(Self{smb: None, url, path, kind, name})
      },
      |handle| Ok(handle.to_owned())
    )
  }
}

#[napi]
pub struct JsSmbDirectoryHandle {
  handle: JsSmbHandle,
  #[napi(js_name="[Symbol.asyncIterator]", ts_type="JsSmbDirectoryHandle['entries']")]
  pub _sym: bool, // unused fake member, just to so that generated JsSmbDirectoryHandle class specifies `[Symbol.asyncIterator]: JsSmbDirectoryHandle['entries']`
  #[napi(readonly, ts_type="'directory'")]
  pub kind: String,
  #[napi(readonly)]
  pub name: String
}

#[napi]
impl JsSmbDirectoryHandle {

  #[napi(constructor)]
  pub fn new(url: String) -> Result<Self> {
    let open_res = JsSmbHandle::open(url);
    match open_res {
      Ok(op) => {
        return Ok(op.into());
      },
      Err(e) => {
        println!("err: {:?}", e);
        return Err(e.into())
      },
    }
  }

  #[napi]
  pub fn to_handle(&self) -> Result<JsSmbHandle> {
    Ok(self.handle.clone())
  }

  #[napi]
  pub fn is_same_entry(&self, other: &JsSmbHandle) -> Result<bool> {
    self.handle.is_same_entry(other)
  }

  #[napi]
  pub async fn query_permission(&self, perm: JsSmbHandlePermissionDescriptor) -> Result<String> {
    self.handle.query_permission(perm).await
  }

  #[napi]
  pub async fn request_permission(&self, perm: JsSmbHandlePermissionDescriptor) -> Result<String> {
    self.handle.request_permission(perm).await
  }

  fn smb_entries(&self) -> Result<Vec<JsSmbHandle>> {
    let smb = &self.handle.smb;
    let mut my_smb = using_rwlock!(smb);
    self.smb_entries_guarded(&mut my_smb)
  }

  fn smb_entries_guarded(&self, my_smb: &mut RwLockWriteGuard<Box<dyn VFS>>) -> Result<Vec<JsSmbHandle>> {
    let mut entries = Vec::new();
    let path = self.handle.path.as_str();
    let dir = my_smb.opendir(path)?;
    for entry in dir {
      if let Some(e) = entry.ok() {
        let name = e.path;
        let (kind, path) = match e.d_type {
          VFSEntryType::Directory => (KIND_DIRECTORY.into(), format_dir_path(&self.handle.path, &name)),
          _ => (KIND_FILE.into(), format_file_path(&self.handle.path, &name))
        };
        if kind != KIND_DIRECTORY || (name != DIR_CURRENT && name != DIR_PARENT) {
          entries.push(JsSmbHandle{smb: self.handle.smb.clone(), url: self.handle.url.to_owned(), path, kind, name});
        }
      }
    }
    Ok(entries)
  }

  #[napi(iterator, ts_return_type="AsyncIterableIterator<[string, JsSmbDirectoryHandle | JsSmbFileHandle]>")]
  pub fn entries(&self, env: Env) -> Result<JsSmbDirectoryHandleEntries> {
    Ok(JsSmbDirectoryHandleEntries{entries: self.smb_entries()?, env: SendWrapper::new(env), count: 0, _sym: false})
  }

  #[napi(iterator, ts_return_type="AsyncIterableIterator<string>")]
  pub fn keys(&self) -> Result<JsSmbDirectoryHandleKeys> {
    Ok(JsSmbDirectoryHandleKeys{entries: self.smb_entries()?, count: 0, _sym: false})
  }

  #[napi(iterator, ts_return_type="AsyncIterableIterator<JsSmbDirectoryHandle | JsSmbFileHandle>")]
  pub fn values(&self) -> Result<JsSmbDirectoryHandleValues> {
    Ok(JsSmbDirectoryHandleValues{entries: self.smb_entries()?, count: 0, _sym: false})
  }

  #[napi]
  pub async fn get_directory_handle(&self, name: String, #[napi(ts_arg_type="JsSmbGetDirectoryOptions")] options: Option<JsSmbGetDirectoryOptions>) -> Result<JsSmbDirectoryHandle> {
    for entry in self.smb_entries()? {
      if entry.name == name {
        if entry.kind != KIND_DIRECTORY {
          return Err(Error::new(Status::GenericFailure, "The path supplied exists, but was not an entry of requested type.".to_string()));
        }
        return Ok(entry.into());
      }
    }
    if !options.unwrap_or_default().create {
      return Err(Error::new(Status::GenericFailure, format!("Directory {:?} not found", name)));
    }
    let path = format_dir_path(&self.handle.path, &name);
    let smb = &self.handle.smb;
    let my_smb = using_rwlock!(smb);
    let _ = my_smb.mkdir(path.trim_end_matches('/'), 0o775)?;
    Ok(JsSmbHandle{smb: self.handle.smb.clone(), url: self.handle.url.to_owned(), path, kind: KIND_DIRECTORY.into(), name}.into())
  }

  #[napi]
  pub async fn get_file_handle(&self, name: String, #[napi(ts_arg_type="JsSmbGetFileOptions")] options: Option<JsSmbGetFileOptions>) -> Result<JsSmbFileHandle> {
    for entry in self.smb_entries()? {
      if entry.name == name {
        if entry.kind != KIND_FILE {
          return Err(Error::new(Status::GenericFailure, "The path supplied exists, but was not an entry of requested type.".to_string()));
        }
        return Ok(entry.into());
      }
    }
    if !options.unwrap_or_default().create {
      return Err(Error::new(Status::GenericFailure, format!("File {:?} not found", name)));
    }
    let path = format_file_path(&self.handle.path, &name);
    let smb = &self.handle.smb;
    let mut my_smb = using_rwlock!(smb);
    let _ = my_smb.create(path.as_str(), nix::fcntl::OFlag::O_SYNC.bits() as u32, (Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IRGRP | Mode::S_IWGRP | Mode::S_IROTH | Mode::S_IWOTH).bits() as u32)?; // XXX: change mode value to 0o664?
    Ok(JsSmbHandle{smb: self.handle.smb.clone(), url: self.handle.url.to_owned(), path, kind: KIND_FILE.into(), name}.into())
  }

  fn smb_remove(&self, entry: &JsSmbHandle, recursive: bool) -> Result<()> {
    let smb = &self.handle.smb;
    let mut my_smb = using_rwlock!(smb);
    self.smb_remove_guarded(&mut my_smb, entry, recursive)
  }

  fn smb_remove_guarded(&self, my_smb: &mut RwLockWriteGuard<Box<dyn VFS>>, entry: &JsSmbHandle, recursive: bool) -> Result<()> {
    if entry.kind == KIND_DIRECTORY {
      let subentries = JsSmbDirectoryHandle::from(entry.to_owned()).smb_entries_guarded(my_smb)?;
      if !recursive && subentries.len() > 0 {
        return Err(Error::new(Status::GenericFailure, format!("Directory {:?} is not empty", entry.name)));
      }

      for subentry in subentries {
        let _ = self.smb_remove_guarded(my_smb, &subentry, recursive)?;
      }

      my_smb.rmdir(entry.path.trim_end_matches('/'))?;
    } else {
      my_smb.unlink(entry.path.as_str())?;
    }

    Ok(())
  }

  #[napi]
  pub async fn remove_entry(&self, name: String, #[napi(ts_arg_type="JsSmbRemoveOptions")] options: Option<JsSmbRemoveOptions>) -> Result<()> {
    for entry in self.smb_entries()? {
      if entry.name == name {
        return self.smb_remove(&entry, options.unwrap_or_default().recursive);
      }
    }
    Err(Error::new(Status::GenericFailure, format!("Entry {:?} not found", name)))
  }

  fn smb_resolve(&self, subentries: Vec<JsSmbHandle>, possible_descendant: &JsSmbHandle) -> Result<Vec<String>> {
    for subentry in subentries {
      if subentry.is_same(possible_descendant) {
        return Ok(subentry.path.trim_matches('/').split('/').map(str::to_string).collect());
      }

      if subentry.kind == KIND_DIRECTORY {
        let subdir = JsSmbDirectoryHandle::from(subentry);
        let res = subdir.smb_resolve(subdir.smb_entries()?, possible_descendant);
        if res.is_ok() {
          return res;
        }
      }
    }
    Err(Error::new(Status::GenericFailure, format!("Possible descendant {} {:?} not found", possible_descendant.kind, possible_descendant.name)))
  }

  #[napi(ts_return_type="Promise<Array<string> | null>")]
  pub fn resolve(&self, possible_descendant: JsSmbHandle) -> AsyncTask<JsSmbDirectoryHandleResolve> {
    AsyncTask::new(JsSmbDirectoryHandleResolve{handle: JsSmbDirectoryHandle{handle: self.handle.clone(), kind: self.kind.clone(), name: self.name.clone(), _sym: false}, possible_descendant})
  }

  #[napi]
  pub fn watch(&self, callback: JsFunction) -> Result<Cancellable> {
    let tsfn: ThreadsafeFunction<Result<(String, String, Option<String>)>, ErrorStrategy::Fatal> = callback
      .create_threadsafe_function(0, |ctx: ThreadSafeCallContext<std::prelude::v1::Result<(String, String, Option<String>), Error>>| {
        ctx.value.map(|(path, action, from_path)| {
          vec![JsSmbNotifyChange{path, action, from_path}]
        })
      })?;

    let (ready_tx, ready_rx) = channel();
    let (done_tx, done_rx) = channel();
    let (cancelled_tx, cancelled_rx) = channel();
    let ret = Cancellable{done_rx: Arc::new(RwLock::new(Box::new(done_rx))), cancelled_tx: Arc::new(RwLock::new(Box::new(cancelled_tx)))};
    let mut handle = self.handle.clone();
    handle.smb = None;
    thread::spawn(move || {
      let watch_mode = VFSWatchMode::Recursive;
      let listen_flags = VFSFileNotificationOperation::all();
      while cancelled_rx.try_recv().is_err() { // FIXME: more stringent check? (taking into account dropped sender?)
        let handle = handle.clone_with_new_connection().unwrap();
        let smb = &handle.smb;
        let path = &handle.path;
        let my_smb = using_rwlock!(smb);
        let cb = Box::new(JsSmbDirectoryHandleWatchCallback{tsfn: tsfn.clone()});
        my_smb.watch(path, watch_mode, listen_flags, cb, &ready_tx, &cancelled_rx);
      }
      let _ = done_tx.send(true);
    });
    let _ = ready_rx.recv();
    Ok(ret)
  }
}

#[napi]
pub struct Cancellable {
  done_rx: Arc<RwLock<Box<Receiver<bool>>>>,
  cancelled_tx: Arc<RwLock<Box<Sender<bool>>>>,
}

unsafe impl Send for Cancellable{}
unsafe impl Sync for Cancellable{}

#[napi]
impl Cancellable {
  #[napi]
  pub async fn wait(&self) {
    let done_rx = self.done_rx.read().unwrap();
    let _ = done_rx.recv();
  }

  #[napi]
  pub fn cancel(&self) {
    // XXX: send on cancelled channel twice - once for libc::poll loop and once for loop in thread above
    let cancelled_tx = self.cancelled_tx.write().unwrap();
    let _ = cancelled_tx.send(true);
    let _ = cancelled_tx.send(true);
  }
}

struct JsSmbDirectoryHandleWatchCallback {
  tsfn: ThreadsafeFunction<Result<(String, String, Option<String>)>, ErrorStrategy::Fatal>,
}

impl VFSNotifyChangeCallback for JsSmbDirectoryHandleWatchCallback {
  fn call(&self, path: String, action: String, from_path: Option<String>) {
    self.tsfn.call(Ok((path, action, from_path)), ThreadsafeFunctionCallMode::NonBlocking);
  }
}

impl From<JsSmbHandle> for JsSmbDirectoryHandle {

  fn from(handle: JsSmbHandle) -> Self {
    Self{kind: handle.kind.clone(), name: handle.name.clone(), handle, _sym: false}
  }
}

pub struct JsSmbDirectoryHandleResolve {
  handle: JsSmbDirectoryHandle,
  possible_descendant: JsSmbHandle
}

#[napi]
impl Task for JsSmbDirectoryHandleResolve {

  type Output = Either<Vec<String>, Null>;

  type JsValue = Either<Vec<String>, Null>;

  fn compute(&mut self) -> Result<Self::Output> {
    self.handle.smb_resolve(self.handle.smb_entries()?, &self.possible_descendant)
      .map_or_else(
        |_| Ok(Either::B(Null)),
        |resolved| Ok(Either::A(resolved))
      )
  }

  fn resolve(&mut self, _env: Env, output: Self::Output) -> Result<Self::JsValue> {
    Ok(output)
  }
}

#[napi]
pub struct JsSmbFileHandle {
  handle: JsSmbHandle,
  #[napi(readonly, ts_type="'file'")]
  pub kind: String,
  #[napi(readonly)]
  pub name: String
}

#[napi]
impl JsSmbFileHandle {

  #[napi]
  pub fn to_handle(&self) -> Result<JsSmbHandle> {
    Ok(self.handle.clone())
  }

  #[napi]
  pub fn is_same_entry(&self, other: &JsSmbHandle) -> Result<bool> {
    self.handle.is_same_entry(other)
  }

  #[napi]
  pub async fn query_permission(&self, perm: JsSmbHandlePermissionDescriptor) -> Result<String> {
    self.handle.query_permission(perm).await
  }

  #[napi]
  pub async fn request_permission(&self, perm: JsSmbHandlePermissionDescriptor) -> Result<String> {
    self.handle.request_permission(perm).await
  }

  #[napi(ts_return_type="Promise<File>")]
  pub async fn get_file(&self) -> Result<JsSmbFile> {
    let path = Path::new(self.handle.path.as_str());
    let type_ = mime_guess::from_path(path).first_raw().unwrap_or(MIME_TYPE_UNKNOWN).into();
    let smb = &self.handle.smb;
    let my_smb = using_rwlock!(smb);
    let smb_stat = my_smb.stat(self.handle.path.as_str())?;
    Ok(JsSmbFile{handle: self.handle.clone(), size: smb_stat.size as i64, type_, last_modified: ((smb_stat.mtime * 1000) + (smb_stat.mtime_nsec / 1000000)) as i64, name: self.name.clone()})
  }

  #[napi]
  pub async fn create_writable(&self, #[napi(ts_arg_type="JsSmbCreateWritableOptions")] options: Option<JsSmbCreateWritableOptions>) -> Result<JsSmbWritableFileStream> {
    let position = (!options.unwrap_or_default().keep_existing_data).then(|| 0);
    let smb = &self.handle.smb;
    let my_smb = using_rwlock!(smb);
    let _ = my_smb.stat(self.handle.path.as_str())?; // XXX: stat file so that we get error if file no longer exists
    Ok(JsSmbWritableFileStream{handle: self.handle.clone(), position, locked: false})
  }
}

impl From<JsSmbHandle> for JsSmbFileHandle {
  fn from(handle: JsSmbHandle) -> Self {
    Self{kind: handle.kind.clone(), name: handle.name.clone(), handle}
  }
}

#[napi]
pub struct JsSmbFile {
  handle: JsSmbHandle,
  #[napi(readonly)]
  pub size: i64,
  #[napi(readonly)]
  pub type_: String,
  #[napi(readonly)]
  pub last_modified: i64,
  #[napi(readonly)]
  pub name: String
}

#[napi]
impl JsSmbFile {

  #[napi(ts_return_type="Promise<ArrayBuffer>")]
  pub fn array_buffer(&self) -> AsyncTask<JsSmbFileArrayBuffer> {
    AsyncTask::new(JsSmbFileArrayBuffer(JsSmbFile{handle: self.handle.clone(), size: self.size, type_: self.type_.clone(), last_modified: self.last_modified, name: self.name.clone()}))
  }

  fn get_index_from_optional(&self, pos: Option<i64>, max: i64, def: i64) -> usize {
    pos.and_then(|mut pos| {
      if pos < 0 {
        pos += max;
        if pos < 0 {
          pos = 0;
        }
      } else if pos > max {
        pos = max;
      }
      Some(pos)
    }).unwrap_or(def) as usize
  }

  pub fn smb_slice(&self, start: Option<i64>, end: Option<i64>) -> Result<Vec<u8>> {
    let content = self.smb_bytes()?;
    let len = content.len() as i64;
    let start = self.get_index_from_optional(start, len, 0);
    let end = self.get_index_from_optional(end, len, len);
    Ok(content.get(start..end).unwrap_or_default().to_vec())
  }

  #[napi(ts_return_type="Blob")]
  pub fn slice(&self, env: Env, #[napi(ts_arg_type="number")] start: Option<i64>, #[napi(ts_arg_type="number")] end: Option<i64>, #[napi(ts_arg_type="string")] content_type: Option<String>) -> Result<Object> {
    let sliced = self.smb_slice(start, end)?;
    let mut arg1 = env.create_array_with_length(1)?;
    let _ = arg1.set_element(0, env.create_arraybuffer_with_data(sliced)?.into_raw().coerce_to_object()?)?;
    let mut arg2 = env.create_object()?;
    let _ = arg2.set_named_property(FIELD_TYPE, env.create_string(content_type.unwrap_or_default().as_str())?)?;
    let global = env.get_global()?;
    let constructor = global.get_named_property::<JsFunction>(JS_TYPE_BLOB)?;
    let blob = constructor.new_instance(&[arg1, arg2])?;
    Ok(blob)
  }

  #[napi(ts_return_type="ReadableStream<Uint8Array>")]
  pub fn stream(&self, env: Env) -> Result<Object> {
    let global = env.get_global()?;
    let constructor = global.get_named_property::<JsFunction>(JS_TYPE_READABLE_STREAM)?;
    let arg = JsSmbReadableStreamSource{handle: self.handle.clone(), offset: 0, type_: READABLE_STREAM_SOURCE_TYPE_BYTES.into()}.into_instance(env)?;
    let stream = constructor.new_instance(&[arg])?;
    Ok(stream)
  }

  fn smb_bytes(&self) -> Result<Vec<u8>> {
    let smb = &self.handle.smb;
    let mut my_smb = using_rwlock!(smb);
    let smb_file = my_smb.open(self.handle.path.as_str(), nix::fcntl::OFlag::O_SYNC.bits() as u32)?;
    let smb_stat = smb_file.fstat()?;
    let buffer = &mut vec![0u8; smb_stat.size as usize];
    let _ = smb_file.pread_into(smb_stat.size as u32, 0, buffer)?;
    Ok(buffer.to_vec())
  }

  #[napi]
  pub async fn text(&self) -> Result<String> {
    Ok(std::str::from_utf8(&self.smb_bytes()?).unwrap_or_default().into())
  }
}

pub struct JsSmbFileArrayBuffer(JsSmbFile);

#[napi]
impl Task for JsSmbFileArrayBuffer {

  type Output = Vec<u8>;

  type JsValue = JsArrayBuffer;

  fn compute(&mut self) -> Result<Self::Output> {
    self.0.smb_bytes()
  }

  fn resolve(&mut self, env: Env, output: Self::Output) -> Result<Self::JsValue> {
    Ok(env.create_arraybuffer_with_data(output)?.into_raw())
  }
}

#[napi]
pub struct JsSmbReadableStreamSource {
  handle: JsSmbHandle,
  offset: u64,
  #[napi(readonly, ts_type="'bytes'")]
  pub type_: String
}

#[napi]
impl JsSmbReadableStreamSource {

  #[napi]
  pub fn pull(&mut self, env: Env, #[napi(ts_arg_type="ReadableByteStreamController")] controller: Unknown) -> Result<()> {
    let controller = controller.coerce_to_object()?;
    let smb = &self.handle.smb;
    let mut my_smb = using_rwlock!(smb);
    let smb_file = my_smb.open(self.handle.path.as_str(), nix::fcntl::OFlag::O_SYNC.bits() as u32)?;
    let size = smb_file.fstat()?.size;
    if self.offset < size {
      let max_count = smb_file.get_max_read_size();
      let count = max_count.min(size - self.offset) as u32;
      let mut buffer = vec![0u8; count as usize];
      let bytes_read = smb_file.pread_into(count, self.offset, &mut buffer)?;

      let enqueue = controller.get_named_property::<JsFunction>(FIELD_ENQUEUE)?;
      let arg = env.create_arraybuffer_with_data(buffer)?;
      let arg = arg.into_raw().into_typedarray(TypedArrayType::Uint8, bytes_read as usize, 0)?;
      let _ = enqueue.call(Some(&controller), &[arg]);
      self.offset += bytes_read as u64;
    } else {
      let close = controller.get_named_property::<JsFunction>(FIELD_CLOSE)?;
      let _ = close.call_without_args(Some(&controller))?;
    }
    Ok(())
  }
}

#[napi]
pub struct JsSmbWritableFileStream {
  handle: JsSmbHandle,
  position: Option<i64>,
  #[napi(readonly)]
  pub locked: bool
}

#[napi]
impl JsSmbWritableFileStream {

  fn parse_write_input(&self, input: Unknown) -> Result<JsSmbWritableFileStreamWriteOptions> {
    match input.get_type()? {
      ValueType::String => self.parse_string(input.coerce_to_string()?, None),
      ValueType::Object => self.parse_write_input_object(input.coerce_to_object()?),
      _ => Err(Error::new(Status::InvalidArg, "Writing unsupported type".to_string()))
    }
  }

  fn parse_write_input_object(&self, obj: Object) -> Result<JsSmbWritableFileStreamWriteOptions> {
    if obj.has_named_property(FIELD_TYPE)? {
      let type_ = obj.get_named_property::<Unknown>(FIELD_TYPE)?;
      if type_.get_type()? == ValueType::String {
        match type_.coerce_to_string()?.into_utf8()?.as_str()? {
          WRITE_TYPE_SEEK => return self.parse_seek_options(obj),
          WRITE_TYPE_TRUNCATE => return self.parse_truncate_options(obj),
          WRITE_TYPE_WRITE => return self.parse_write_options(obj),
          _ => ()
        };
      }
    }
    match () {
      _ if is_string_object(&obj)? => self.parse_string(obj.coerce_to_string()?, None),
      _ if is_blob(&obj)? => self.parse_blob(obj, None),
      _ if is_typed_array(&obj)? => self.parse_typed_array(obj, None),
      _ if is_data_view(&obj)? => self.parse_data_view(obj, None),
      _ if is_array_buffer(&obj)? => self.parse_array_buffer(obj, None),
      _ => Err(Error::new(Status::InvalidArg, "Writing unsupported type".to_string()))
    }
  }

  fn parse_seek_options(&self, obj: Object) -> Result<JsSmbWritableFileStreamWriteOptions> {
    if obj.has_named_property(FIELD_POSITION)? {
      let position = obj.get_named_property::<Unknown>(FIELD_POSITION)?;
      if position.get_type()? == ValueType::Number {
        return Ok(JsSmbWritableFileStreamWriteOptions{
          type_: WRITE_TYPE_SEEK.into(),
          data: None,
          position: Some(position.coerce_to_number()?.get_int64()?),
          size: None,
        });
      }
    }
    Err(Error::new(Status::InvalidArg, format!("Property position of type number is required when writing object with type={:?}", WRITE_TYPE_SEEK)))
  }

  fn parse_truncate_options(&self, obj: Object) -> Result<JsSmbWritableFileStreamWriteOptions> {
    if obj.has_named_property(FIELD_SIZE)? {
      let size = obj.get_named_property::<Unknown>(FIELD_SIZE)?;
      if size.get_type()? == ValueType::Number {
        return Ok(JsSmbWritableFileStreamWriteOptions{
          type_: WRITE_TYPE_TRUNCATE.into(),
          data: None,
          position: None,
          size: Some(size.coerce_to_number()?.get_int64()?),
        });
      }
    }
    Err(Error::new(Status::InvalidArg, format!("Property size of type number is required when writing object with type={:?}", WRITE_TYPE_TRUNCATE)))
  }

  fn parse_write_options(&self, obj: Object) -> Result<JsSmbWritableFileStreamWriteOptions> {
    let mut pos = None;
    if obj.has_named_property(FIELD_POSITION)? {
      let position = obj.get_named_property::<Unknown>(FIELD_POSITION)?;
      if position.get_type()? == ValueType::Number {
        pos = Some(position.coerce_to_number()?.get_int64()?);
      }
    }
    if obj.has_named_property(FIELD_DATA)? {
      return self.parse_wrapped_data(obj.get_named_property::<Unknown>(FIELD_DATA)?, pos);
    }
    Err(Error::new(Status::InvalidArg, format!("Property data of type object or string is required when writing object with type={:?}", WRITE_TYPE_WRITE)))
  }

  fn parse_wrapped_data(&self, data: Unknown, position: Option<i64>) -> Result<JsSmbWritableFileStreamWriteOptions> {
    match data.get_type()? {
      ValueType::String => self.parse_string(data.coerce_to_string()?, position),
      ValueType::Object => self.parse_wrapped_data_object(data.coerce_to_object()?, position),
      _ => Err(Error::new(Status::InvalidArg, "Writing unsupported data type".to_string())),
    }
  }

  fn parse_wrapped_data_object(&self, data: Object, position: Option<i64>) -> Result<JsSmbWritableFileStreamWriteOptions> {
    match () {
      _ if is_string_object(&data)? => self.parse_string(data.coerce_to_string()?, position),
      _ if is_blob(&data)? => self.parse_blob(data, position),
      _ if is_typed_array(&data)? => self.parse_typed_array(data, position),
      _ if is_data_view(&data)? => self.parse_data_view(data, position),
      _ if is_array_buffer(&data)? => self.parse_array_buffer(data, position),
      _ => Err(Error::new(Status::InvalidArg, "Writing unsupported data type".to_string()))
    }
  }

  fn parse_string(&self, string: JsString, position: Option<i64>) -> Result<JsSmbWritableFileStreamWriteOptions> {
    self.parsed_write_options(Some(string.into_utf8()?.as_str()?.as_bytes().to_owned()), position)
  }

  fn parse_blob(&self, _blob: Object, position: Option<i64>) -> Result<JsSmbWritableFileStreamWriteOptions> {
    self.parsed_write_options(None, position) // FIXME
  }

  fn parse_typed_array(&self, typed_array: Object, position: Option<i64>) -> Result<JsSmbWritableFileStreamWriteOptions> {
    let bytes_per_type = |t: TypedArrayType| -> usize {
      match t {
        TypedArrayType::BigInt64 | TypedArrayType::BigUint64 | TypedArrayType::Float64 => 8,
        TypedArrayType::Int32 | TypedArrayType::Uint32 | TypedArrayType::Float32 => 4,
        TypedArrayType::Int16 | TypedArrayType::Uint16 => 2,
        _ => 1
      }
    };

    let typed_array_value = JsTypedArray::try_from(typed_array.into_unknown())?.into_value()?;
    let bytes = typed_array_value.arraybuffer.into_value()?.to_owned();
    let start = typed_array_value.byte_offset; // FIXME: should start be multiplied by bytes_per_type?
    let end = start + (typed_array_value.length * bytes_per_type(typed_array_value.typedarray_type));
    self.parsed_write_options(Some(bytes[start..end].to_vec()), position)
  }

  fn parse_data_view(&self, data_view: Object, position: Option<i64>) -> Result<JsSmbWritableFileStreamWriteOptions> {
    let data_view_value = JsDataView::try_from(data_view.into_unknown())?.into_value()?;
    let bytes = data_view_value.arraybuffer.into_value()?.to_owned();
    let start = data_view_value.byte_offset as usize;
    let end = start + (data_view_value.length as usize);
    self.parsed_write_options(Some(bytes[start..end].to_vec()), position)
  }

  fn parse_array_buffer(&self, array_buffer: Object, position: Option<i64>) -> Result<JsSmbWritableFileStreamWriteOptions> {
    self.parsed_write_options(Some(JsArrayBuffer::try_from(array_buffer.into_unknown())?.into_value()?.to_owned()), position)
  }

  fn parsed_write_options(&self, data: Option<Vec<u8>>, position: Option<i64>) -> Result<JsSmbWritableFileStreamWriteOptions> {
    Ok(JsSmbWritableFileStreamWriteOptions{
      type_: WRITE_TYPE_WRITE.into(),
      data,
      position,
      size: None
    })
  }

  fn try_seek_and_write_data(&mut self, options: &JsSmbWritableFileStreamWriteOptions) -> Result<Undefined> {
    let old_position = self.position.clone();
    if let Some(position) = options.position {
      self.smb_seek(position)?;
    }
    let res = self.try_write_data(options);
    if !res.is_ok() {
      self.position = old_position;
    }
    res
  }

  fn try_write_data(&mut self, options: &JsSmbWritableFileStreamWriteOptions) -> Result<Undefined> {
    if let Some(data) = &options.data {
      return self.smb_write(data.as_slice());
    }
    Err(Error::new(Status::InvalidArg, format!("Property data of type object or string is required when writing object with type={:?}", WRITE_TYPE_WRITE)))
  }

  fn smb_write(&mut self, bytes: &[u8]) -> Result<Undefined> {
    let smb = &self.handle.smb;
    let mut my_smb = using_rwlock!(smb);
    //let smb_file = my_smb.open(self.handle.path.as_str(), nix::fcntl::OFlag::O_SYNC.bits() as u32)?;
    let mut flags = nix::fcntl::OFlag::O_RDWR;
    flags.insert(nix::fcntl::OFlag::O_SYNC);
    let smb_file = my_smb.open(self.handle.path.as_str(), flags.bits() as u32)?;  
    let offset = match self.position {
      None => smb_file.fstat()?.size,
      Some(pos) => pos as u64
    };
    let _ = smb_file.pwrite(bytes, offset)?;
    let post_write_pos = (offset as i64) + (bytes.len() as i64);
    self.position = Some(post_write_pos);
    Ok(())
  }

  #[napi(ts_return_type="Promise<void>")]
  pub fn write(&'static mut self, #[napi(ts_arg_type="ArrayBuffer | ArrayBufferView | DataView | Blob | String | string | {type: 'write' | 'seek' | 'truncate', data?: ArrayBuffer | ArrayBufferView | DataView | Blob | String | string, position?: number, size?: number}")] data: Unknown) -> Result<AsyncTask<JsSmbWritableFileStreamWrite>> {
    let options = self.parse_write_input(data)?;
    Ok(AsyncTask::new(JsSmbWritableFileStreamWrite{stream: self, options}))
  }

  fn try_seek(&mut self, options: &JsSmbWritableFileStreamWriteOptions) -> Result<Undefined> {
    if let Some(position) = options.position {
      return self.smb_seek(position);
    }
    Err(Error::new(Status::InvalidArg, format!("Property position of type number is required when writing object with type={:?}", WRITE_TYPE_SEEK)))
  }

  fn smb_seek(&mut self, position: i64) -> Result<Undefined> {
    self.position = Some(position);
    Ok(())
  }

  #[napi(ts_return_type="Promise<void>")]
  pub fn seek(&mut self, position: i64) -> Result<Undefined> {
    self.smb_seek(position)
  }

  fn try_truncate(&mut self, options: &JsSmbWritableFileStreamWriteOptions) -> Result<Undefined> {
    if let Some(size) = options.size {
      return self.smb_truncate(size);
    }
    Err(Error::new(Status::InvalidArg, format!("Property size of type number is required when writing object with type={:?}", WRITE_TYPE_TRUNCATE)))
  }

  fn smb_truncate(&mut self, size: i64) -> Result<Undefined> {
    let smb = &self.handle.smb;
    let my_smb = using_rwlock!(smb);
    let smb_stat = my_smb.stat(self.handle.path.as_str())?;
    my_smb.truncate(self.handle.path.as_str(), size as u64)?;
    let size_before = smb_stat.size as i64;
    if let Some(position) = self.position {
      if position > size || position == size_before {
        self.position = Some(size);
      }
    }
    Ok(())
  }

  #[napi(ts_return_type="Promise<void>")]
  pub fn truncate(&'static mut self, size: i64) -> AsyncTask<JsSmbWritableFileStreamTruncate> {
    AsyncTask::new(JsSmbWritableFileStreamTruncate{stream: self, size})
  }

  #[napi]
  pub async fn close(&self) -> Result<Undefined> {
    Ok(())
  }

  #[napi]
  pub async fn abort(&self, reason: String) -> Result<String> {
    Ok(reason)
  }

  #[napi]
  pub fn release_lock(&mut self) -> Result<Undefined> {
    self.locked = false;
    Ok(())
  }

  #[napi(ts_return_type="WritableStreamDefaultWriter")]
  pub fn get_writer(&'static mut self, env: Env) -> Result<Object> {
    if self.locked {
      return Err(Error::new(Status::GenericFailure, "Invalid state: WritableStream is locked".to_string()));
    }
    let global = env.get_global()?;
    let sink = JsSmbWritableStreamSink{stream: self, closed: false}.into_instance(env)?;
    let stream_constructor = global.get_named_property::<JsFunction>(JS_TYPE_WRITABLE_STREAM)?;
    let arg = stream_constructor.new_instance(&[sink])?;
    let constructor = global.get_named_property::<JsFunction>(JS_TYPE_WRITABLE_STREAM_DEFAULT_WRITER)?;
    Ok(constructor.new_instance(&[arg])?)
  }
}

pub struct JsSmbWritableFileStreamWriteOptions {
  type_: String,
  data: Option<Vec<u8>>,
  position: Option<i64>,
  size: Option<i64>
}

impl Default for JsSmbWritableFileStreamWriteOptions {

  fn default() -> Self {
    Self{type_: Default::default(), data: Default::default(), position: Default::default(), size: Default::default()}
  }
}

pub struct JsSmbWritableFileStreamWrite {
  stream: &'static mut JsSmbWritableFileStream,
  options: JsSmbWritableFileStreamWriteOptions
}

#[napi]
impl Task for JsSmbWritableFileStreamWrite {

  type Output = ();

  type JsValue = ();

  fn compute(&mut self) -> Result<Self::Output> {
    match self.options.type_.as_str() {
      WRITE_TYPE_WRITE => self.stream.try_seek_and_write_data(&self.options),
      WRITE_TYPE_SEEK => self.stream.try_seek(&self.options),
      WRITE_TYPE_TRUNCATE => self.stream.try_truncate(&self.options),
      _ => Err(Error::new(Status::GenericFailure, format!("Unknown write type: {:?}", self.options.type_.as_str())))
    }
  }

  fn resolve(&mut self, _env: Env, _output: Self::Output) -> Result<Self::JsValue> {
    Ok(())
  }
}

pub struct JsSmbWritableFileStreamTruncate {
  stream: &'static mut JsSmbWritableFileStream,
  size: i64
}

#[napi]
impl Task for JsSmbWritableFileStreamTruncate {

  type Output = ();

  type JsValue = ();

  fn compute(&mut self) -> Result<Self::Output> {
    self.stream.smb_truncate(self.size)
  }

  fn resolve(&mut self, _env: Env, _output: Self::Output) -> Result<Self::JsValue> {
    Ok(())
  }
}

#[napi]
pub struct JsSmbWritableStreamSink {
  stream: &'static mut JsSmbWritableFileStream,
  closed: bool
}

#[napi]
impl JsSmbWritableStreamSink {

  #[napi(ts_args_type="controller?: WritableStreamDefaultController", ts_return_type="Promise<void>")]
  pub fn start(&mut self) -> Result<()> {
    self.stream.locked = true;
    Ok(())
  }

  #[napi(ts_return_type="Promise<string>")]
  pub fn abort(&mut self, reason: String) -> Result<String> {
    self.close_stream();
    Ok(reason)
  }

  fn close_stream(&mut self) {
    self.closed = true;
  }

  #[napi(ts_args_type="controller?: WritableStreamDefaultController", ts_return_type="Promise<void>")]
  pub fn close(&mut self) -> Result<()> {
    if self.closed {
      return Err(Error::new(Status::GenericFailure, "Invalid state: WritableStream is closed".to_string()));
    }
    self.close_stream();
    Ok(())
  }

  #[napi(ts_return_type="Promise<void>")]
  pub fn write(&'static mut self, #[napi(ts_arg_type="any")] chunk: Unknown, #[napi(ts_arg_type="WritableStreamDefaultController")] _controller: Option<Unknown>) -> Result<AsyncTask<JsSmbWritableStreamWrite>> {
    if self.closed {
      return Err(Error::new(Status::GenericFailure, "Invalid state: WritableStream is closed".to_string()));
    }
    let options = self.stream.parse_write_input(chunk).unwrap_or_default();
    if options.type_ != WRITE_TYPE_WRITE {
      return Err(Error::new(Status::InvalidArg, "Invalid chunk".to_string()));
    }
    Ok(AsyncTask::new(JsSmbWritableStreamWrite{sink: self, chunk: options.data.unwrap_or_default()}))
  }
}

pub struct JsSmbWritableStreamWrite {
  sink: &'static mut JsSmbWritableStreamSink,
  chunk: Vec<u8>
}

#[napi]
impl Task for JsSmbWritableStreamWrite {

  type Output = ();

  type JsValue = ();

  fn compute(&mut self) -> Result<Self::Output> {
    self.sink.stream.smb_write(self.chunk.as_slice())
  }

  fn resolve(&mut self, _env: Env, _output: Self::Output) -> Result<Self::JsValue> {
    Ok(())
  }
}

fn get_parent_path_and_name(path: &String) -> (String, String) {
  path.rsplit_once('/').map(|res| (res.0.to_string() + "/", res.1.to_string())).unwrap_or_default()
}

fn format_dir_path(parent_path: &String, name: &String) -> String {
  format!("{}{}/", parent_path, name)
}

fn format_file_path(parent_path: &String, name: &String) -> String {
  format!("{}{}", parent_path, name)
}

fn is_string_object(obj: &Object) -> Result<bool> {
  Ok(obj.has_named_property(FIELD_SUBSTRING)?
    && obj.get_named_property::<Unknown>(FIELD_SUBSTRING)?.get_type()? == ValueType::Function)
}

fn is_blob(obj: &Object) -> Result<bool> {
  Ok(obj.has_named_property(FIELD_TYPE)?
    && obj.get_named_property::<Unknown>(FIELD_TYPE)?.get_type()? == ValueType::String)
}

fn is_typed_array(obj: &Object) -> Result<bool> {
  Ok(obj.has_named_property(FIELD_LENGTH)?
    && obj.get_named_property::<Unknown>(FIELD_LENGTH)?.get_type()? == ValueType::Number)
}

fn is_data_view(obj: &Object) -> Result<bool> {
  Ok(obj.has_named_property(FIELD_BUFFER)?
    && obj.get_named_property::<Unknown>(FIELD_BUFFER)?.get_type()? == ValueType::Object)
}

fn is_array_buffer(obj: &Object) -> Result<bool> {
  Ok(obj.has_named_property(FIELD_BYTE_LENGTH)?
    && obj.get_named_property::<Unknown>(FIELD_BYTE_LENGTH)?.get_type()? == ValueType::Number)
}


#[derive(Clone)]
#[napi(object)]
pub struct JsSmbNotifyChange {
  pub path: String,
  pub action: String,
  pub from_path: Option<String>,
}

