/**
 * Copyright 2025 NetApp Inc. All Rights Reserved.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 * SPDX-License-Identifier: Apache-2.0
 */

import {
  JsSmbHandlePermissionDescriptor,
  JsSmbGetDirectoryOptions,
  JsSmbGetFileOptions,
  JsSmbRemoveOptions,
  JsSmbCreateWritableOptions,
  JsSmbHandle,
  JsSmbDirectoryHandle,
  JsSmbFileHandle,
  JsSmbWritableFileStream,
} from './index';

type SmbHandlePermissionDescriptor = JsSmbHandlePermissionDescriptor;
// @ts-ignore
type SmbCreateWritableOptions = FileSystemCreateWritableOptions;
// @ts-ignore
type FileSystemWritableFileStream = FileSystemWritableFileStream;

type TypedArray = Int8Array | Uint8Array | Uint8ClampedArray | Int16Array | Uint16Array | Int32Array | Uint32Array | Float32Array | Float64Array | BigInt64Array | BigUint64Array;

export class SmbHandle implements FileSystemHandle {
  private _jsh: JsSmbHandle
  readonly kind: FileSystemHandleKind
  readonly name: string
  constructor(_jsh: JsSmbHandle) {
    this._jsh = _jsh;
    this.kind = _jsh.kind;
    this.name = _jsh.name;
  }
  isSameEntry(other: FileSystemHandle): Promise<boolean> {
    return new Promise(async (resolve, reject) => {
      try {
        resolve(this._jsh.isSameEntry((other as any)._jsh || other));
      } catch(reason) {
        reject(reason);
      }
    });
  }
  async queryPermission(perm: SmbHandlePermissionDescriptor): Promise<PermissionState> {
    return this._jsh.queryPermission(perm) as Promise<PermissionState>;
  }
  async requestPermission(perm: SmbHandlePermissionDescriptor): Promise<PermissionState> {
    return this._jsh.requestPermission(perm) as Promise<PermissionState>;
  }
}

export class SmbDirectoryHandle extends SmbHandle implements FileSystemDirectoryHandle {
  // @ts-ignore
  [Symbol.asyncIterator]: SmbDirectoryHandle['entries'] = this.entries
  declare readonly kind: 'directory'
  private _js: JsSmbDirectoryHandle
  constructor(url: string);
  constructor(toWrap: JsSmbDirectoryHandle);
  constructor(param: string | JsSmbDirectoryHandle) {
    const [url, toWrap] = typeof param === 'string' ? [param] : ['', param];
    const _js = toWrap || new JsSmbDirectoryHandle(url);
    super(_js.toHandle());
    this[Symbol.asyncIterator] = this.entries;
    this._js = _js;
    this.getFile = this.getFileHandle;
    this.getDirectory = this.getDirectoryHandle;
    this.getEntries = this.values;
  }
  // @ts-ignore
  async *entries(): AsyncIterableIterator<[string, FileSystemDirectoryHandle | FileSystemFileHandle]> {
    for await (const [key, value] of this._js.entries()) {
      yield [key, value instanceof JsSmbDirectoryHandle ? new SmbDirectoryHandle(value) as any as FileSystemDirectoryHandle : new SmbFileHandle(value) as FileSystemFileHandle];
    }
  }
  // @ts-ignore
  async *keys(): AsyncIterableIterator<string> {
    for await (const key of this._js.keys()) {
      yield key;
    }
  }
  // @ts-ignore
  async *values(): AsyncIterableIterator<FileSystemDirectoryHandle | FileSystemFileHandle> {
    for await (const value of this._js.values()) {
      yield value instanceof JsSmbDirectoryHandle ? new SmbDirectoryHandle(value) as any as FileSystemDirectoryHandle : new SmbFileHandle(value) as FileSystemFileHandle;
    }
  }
  async getDirectoryHandle(name: string, options?: FileSystemGetDirectoryOptions): Promise<FileSystemDirectoryHandle> {
    //console.log("getDirectoryHandle: ", name);
    return new Promise(async (resolve, reject) => {
      await this._js.getDirectoryHandle(name, options as JsSmbGetDirectoryOptions)
        .then((handle) => resolve(new SmbDirectoryHandle(handle) as any as FileSystemDirectoryHandle))
        .catch((reason) => {
          let errMsg: string = reason.message;
          if (errMsg !== undefined) {
            if (errMsg == 'The path supplied exists, but was not an entry of requested type.') {
              reason.name = 'TypeMismatchError';
            } else if (errMsg.indexOf('not found') != -1) {
              reason.name = 'NotFoundError';
            }
          }
          reject(reason);
        });
    });
  }
  async getFileHandle(name: string, options?: FileSystemGetFileOptions): Promise<FileSystemFileHandle> {
    return new Promise(async (resolve, reject) => {
      await this._js.getFileHandle(name, options as JsSmbGetFileOptions)
        .then((handle) => resolve(new SmbFileHandle(handle) as FileSystemFileHandle))
        .catch((reason) => {
          let errMsg: string = reason.message;
          if (errMsg !== undefined) {
            if (errMsg == 'The path supplied exists, but was not an entry of requested type.') {
              reason.name = 'TypeMismatchError';
            } else if (errMsg.indexOf('not found') != -1) {
              reason.name = 'NotFoundError';
            }
          }
          reject(reason);
        });
    });
  }
  async removeEntry(name: string, options?: FileSystemRemoveOptions): Promise<void> {
    return this._js.removeEntry(name, options as JsSmbRemoveOptions);
  }
  async resolve(possibleDescendant: FileSystemHandle): Promise<Array<string> | null> {
    return this._js.resolve((possibleDescendant as any)._jsh || possibleDescendant);
  }

  /**
   * @deprecated Old property just for Chromium <=85. Use `.getFileHandle()` in the new API.
   */
  getFile: SmbDirectoryHandle['getFileHandle']
  /**
  * @deprecated Old property just for Chromium <=85. Use `.getDirectoryHandle()` in the new API.
  */
  getDirectory: SmbDirectoryHandle['getDirectoryHandle']
  /**
  * @deprecated Old property just for Chromium <=85. Use `.keys()`, `.values()`, `.entries()`, or the directory itself as an async iterable in the new API.
  */
  getEntries: SmbDirectoryHandle['values']


  watch(callback: (...args: any[]) => any) {
    return this._js.watch(callback)
  }
 }

export class SmbFileHandle extends SmbHandle implements FileSystemFileHandle {
  declare readonly kind: "file";
  private _js: JsSmbFileHandle
  constructor(_js: JsSmbFileHandle) {
    super(_js.toHandle());
    this._js = _js;
  }

  // @ts-ignore
  async createSyncAccessHandle(): Promise<FileSystemSyncAccessHandle> {
    throw Error('createSyncAccessHandle not implemented');
  }

  async getFile(): Promise<File> {
    return this._js.getFile();
  }
  async createWritable(options?: SmbCreateWritableOptions): Promise<FileSystemWritableFileStream> {
    return new Promise(async (resolve, reject) => {
      await this._js.createWritable(options as JsSmbCreateWritableOptions)
        .then((stream) => resolve(new SmbWritableFileStream(stream) as FileSystemWritableFileStream))
        .catch((reason) => reject(reason));
    });
  }
}

interface SmbWritableFileStreamLock { locked: boolean }
export class SmbWritableFileStream implements SmbWritableFileStreamLock {
  private _js: JsSmbWritableFileStream
  readonly locked: boolean
  constructor(_js: JsSmbWritableFileStream) {
    this._js = _js;
    this.locked = _js.locked;
  }
  async write(data: ArrayBuffer | TypedArray | DataView | Blob | String | string | {type: 'write' | 'seek' | 'truncate', data?: ArrayBuffer | TypedArray | DataView | Blob | String | string, position?: number, size?: number}): Promise<void> {
    return new Promise(async (resolve, reject) => {
      if (data instanceof Blob) {
        data = await data.arrayBuffer();
      } else {
        const dat = data as any;
        if (dat.type === 'write' && dat.data instanceof Blob) {
          dat.data = await dat.data.arrayBuffer();
        }
      }

      try {
        await this._js.write(data)
          .then(() => resolve())
          .catch((reason) => reject(reason));
      } catch(reason) {
        reject(reason);
      }
    });
  }
  async seek(position: number): Promise<void> {
    return this._js.seek(position);
  }
  async truncate(size: number): Promise<void> {
    return this._js.truncate(size);
  }
  async close(): Promise<void> {
    return this._js.close();
  }
  async abort(reason: string): Promise<void> {
    return new Promise(async (resolve, reject) => {
      await this._js.abort(reason)
        .then((_reason) => resolve())
        .catch((reason) => reject(reason));
    });
  }
  getWriter(): WritableStreamDefaultWriter {
    const writer = this._js.getWriter();
    (<SmbWritableFileStreamLock>this).locked = true;
    (<WritableStreamDefaultWriterEx>writer)._releaseLock = writer.releaseLock;
    writer.releaseLock = () => {
      (<WritableStreamDefaultWriterEx>writer)._releaseLock();
      this._js.releaseLock();
      (<SmbWritableFileStreamLock>this).locked = false;
    };
    return writer;
  }
}

interface WritableStreamDefaultWriterEx extends WritableStreamDefaultWriter {
  _releaseLock: () => void
}
