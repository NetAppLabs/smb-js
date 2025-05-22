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

"use strict";
var _a;
Object.defineProperty(exports, "__esModule", { value: true });
exports.SmbWritableFileStream = exports.SmbFileHandle = exports.SmbDirectoryHandle = exports.SmbHandle = void 0;
const index_js_1 = require("./index.js");
class SmbHandle {
    constructor(_jsh) {
        this._jsh = _jsh;
        this.kind = _jsh.kind;
        this.name = _jsh.name;
    }
    isSameEntry(other) {
        return new Promise(async (resolve, reject) => {
            try {
                resolve(this._jsh.isSameEntry(other._jsh || other));
            }
            catch (reason) {
                reject(reason);
            }
        });
    }
    async queryPermission(perm) {
        return this._jsh.queryPermission(perm);
    }
    async requestPermission(perm) {
        return this._jsh.requestPermission(perm);
    }
}
exports.SmbHandle = SmbHandle;
class SmbDirectoryHandle extends SmbHandle {
    constructor(param) {
        const [url, toWrap] = typeof param === 'string' ? [param] : ['', param];
        const _js = toWrap || new index_js_1.JsSmbDirectoryHandle(url);
        super(_js.toHandle());
        this[_a] = this.entries;
        this[Symbol.asyncIterator] = this.entries;
        this._js = _js;
        this.getFile = this.getFileHandle;
        this.getDirectory = this.getDirectoryHandle;
        this.getEntries = this.values;
    }
    async *entries() {
        for await (const [key, value] of this._js.entries()) {
            yield [key, value instanceof index_js_1.JsSmbDirectoryHandle ? new SmbDirectoryHandle(value) : new SmbFileHandle(value)];
        }
    }
    async *keys() {
        for await (const key of this._js.keys()) {
            yield key;
        }
    }
    async *values() {
        for await (const value of this._js.values()) {
            yield value instanceof index_js_1.JsSmbDirectoryHandle ? new SmbDirectoryHandle(value) : new SmbFileHandle(value);
        }
    }
    async getDirectoryHandle(name, options) {
        //console.log("getDirectoryHandle: ", name);
        return new Promise(async (resolve, reject) => {
            await this._js.getDirectoryHandle(name, options)
                .then((handle) => resolve(new SmbDirectoryHandle(handle)))
                .catch((reason) => {
                let errMsg = reason.message;
                if (errMsg !== undefined) {
                    if (errMsg == 'The path supplied exists, but was not an entry of requested type.') {
                        reason.name = 'TypeMismatchError';
                    }
                    else if (errMsg.indexOf('not found') != -1) {
                        reason.name = 'NotFoundError';
                    }
                }
                reject(reason);
            });
        });
    }
    async getFileHandle(name, options) {
        return new Promise(async (resolve, reject) => {
            await this._js.getFileHandle(name, options)
                .then((handle) => resolve(new SmbFileHandle(handle)))
                .catch((reason) => {
                let errMsg = reason.message;
                if (errMsg !== undefined) {
                    if (errMsg == 'The path supplied exists, but was not an entry of requested type.') {
                        reason.name = 'TypeMismatchError';
                    }
                    else if (errMsg.indexOf('not found') != -1) {
                        reason.name = 'NotFoundError';
                    }
                }
                reject(reason);
            });
        });
    }
    async removeEntry(name, options) {
        return this._js.removeEntry(name, options);
    }
    async resolve(possibleDescendant) {
        return this._js.resolve(possibleDescendant._jsh || possibleDescendant);
    }
    watch(callback) {
        return this._js.watch(callback);
    }
}
exports.SmbDirectoryHandle = SmbDirectoryHandle;
_a = Symbol.asyncIterator;
class SmbFileHandle extends SmbHandle {
    constructor(_js) {
        super(_js.toHandle());
        this._js = _js;
    }
    // @ts-ignore
    async createSyncAccessHandle() {
        throw Error('createSyncAccessHandle not implemented');
    }
    async getFile() {
        return this._js.getFile();
    }
    async createWritable(options) {
        return new Promise(async (resolve, reject) => {
            await this._js.createWritable(options)
                .then((stream) => resolve(new SmbWritableFileStream(stream)))
                .catch((reason) => reject(reason));
        });
    }
}
exports.SmbFileHandle = SmbFileHandle;
class SmbWritableFileStream {
    constructor(_js) {
        this._js = _js;
        this.locked = _js.locked;
    }
    async write(data) {
        return new Promise(async (resolve, reject) => {
            if (data instanceof Blob) {
                data = await data.arrayBuffer();
            }
            else {
                const dat = data;
                if (dat.type === 'write' && dat.data instanceof Blob) {
                    dat.data = await dat.data.arrayBuffer();
                }
            }
            try {
                await this._js.write(data)
                    .then(() => resolve())
                    .catch((reason) => reject(reason));
            }
            catch (reason) {
                reject(reason);
            }
        });
    }
    async seek(position) {
        return this._js.seek(position);
    }
    async truncate(size) {
        return this._js.truncate(size);
    }
    async close() {
        return this._js.close();
    }
    async abort(reason) {
        return new Promise(async (resolve, reject) => {
            await this._js.abort(reason)
                .then((_reason) => resolve())
                .catch((reason) => reject(reason));
        });
    }
    getWriter() {
        const writer = this._js.getWriter();
        this.locked = true;
        writer._releaseLock = writer.releaseLock;
        writer.releaseLock = () => {
            writer._releaseLock();
            this._js.releaseLock();
            this.locked = false;
        };
        return writer;
    }
}
exports.SmbWritableFileStream = SmbWritableFileStream;
