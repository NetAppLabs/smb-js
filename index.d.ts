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
import { JsSmbHandlePermissionDescriptor, JsSmbStat, JsSmbHandle, JsSmbDirectoryHandle, JsSmbFileHandle, JsSmbWritableFileStream } from './binding';
type SmbStat = JsSmbStat;
type SmbHandlePermissionDescriptor = JsSmbHandlePermissionDescriptor;
type SmbCreateWritableOptions = FileSystemCreateWritableOptions;
type FileSystemWritableFileStream = FileSystemWritableFileStream;
type TypedArray = Int8Array | Uint8Array | Uint8ClampedArray | Int16Array | Uint16Array | Int32Array | Uint32Array | Float32Array | Float64Array | BigInt64Array | BigUint64Array;
export declare class SmbHandle implements FileSystemHandle {
    private _jsh;
    readonly kind: FileSystemHandleKind;
    readonly name: string;
    constructor(_jsh: JsSmbHandle);
    isSameEntry(other: FileSystemHandle): Promise<boolean>;
    queryPermission(perm: SmbHandlePermissionDescriptor): Promise<PermissionState>;
    requestPermission(perm: SmbHandlePermissionDescriptor): Promise<PermissionState>;
    stat(): Promise<SmbStat>;
}
export declare class SmbDirectoryHandle extends SmbHandle implements FileSystemDirectoryHandle {
    [Symbol.asyncIterator]: SmbDirectoryHandle['entries'];
    readonly kind: 'directory';
    private _js;
    constructor(url: string);
    constructor(toWrap: JsSmbDirectoryHandle);
    entries(): AsyncIterableIterator<[string, FileSystemDirectoryHandle | FileSystemFileHandle]>;
    keys(): AsyncIterableIterator<string>;
    values(): AsyncIterableIterator<FileSystemDirectoryHandle | FileSystemFileHandle>;
    getDirectoryHandle(name: string, options?: FileSystemGetDirectoryOptions): Promise<FileSystemDirectoryHandle>;
    getFileHandle(name: string, options?: FileSystemGetFileOptions): Promise<FileSystemFileHandle>;
    removeEntry(name: string, options?: FileSystemRemoveOptions): Promise<void>;
    resolve(possibleDescendant: FileSystemHandle): Promise<Array<string> | null>;
    /**
     * @deprecated Old property just for Chromium <=85. Use `.getFileHandle()` in the new API.
     */
    getFile: SmbDirectoryHandle['getFileHandle'];
    /**
    * @deprecated Old property just for Chromium <=85. Use `.getDirectoryHandle()` in the new API.
    */
    getDirectory: SmbDirectoryHandle['getDirectoryHandle'];
    /**
    * @deprecated Old property just for Chromium <=85. Use `.keys()`, `.values()`, `.entries()`, or the directory itself as an async iterable in the new API.
    */
    getEntries: SmbDirectoryHandle['values'];
    watch(callback: (...args: any[]) => any): import("./binding").Cancellable;
}
export declare class SmbFileHandle extends SmbHandle implements FileSystemFileHandle {
    readonly kind: "file";
    private _js;
    constructor(_js: JsSmbFileHandle);
    createSyncAccessHandle(): Promise<FileSystemSyncAccessHandle>;
    getFile(): Promise<File>;
    createWritable(options?: SmbCreateWritableOptions): Promise<FileSystemWritableFileStream>;
}
interface SmbWritableFileStreamLock {
    locked: boolean;
}
export declare class SmbWritableFileStream implements SmbWritableFileStreamLock {
    private _js;
    readonly locked: boolean;
    constructor(_js: JsSmbWritableFileStream);
    write(data: ArrayBuffer | TypedArray | DataView | Blob | String | string | {
        type: 'write' | 'seek' | 'truncate';
        data?: ArrayBuffer | TypedArray | DataView | Blob | String | string;
        position?: number;
        size?: number;
    }): Promise<void>;
    seek(position: number): Promise<void>;
    truncate(size: number): Promise<void>;
    close(): Promise<void>;
    abort(reason: string): Promise<void>;
    getWriter(): WritableStreamDefaultWriter;
}
export {};
