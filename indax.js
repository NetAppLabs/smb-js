"use strict";
var _a;
Object.defineProperty(exports, "__esModule", { value: true });
exports.SmbWritableFileStream = exports.SmbFileHandle = exports.SmbDirectoryHandle = exports.SmbHandle = void 0;
const index_1 = require("./index");
class SmbHandle {
    constructor(_jsh) {
        this._jsh = _jsh;
        this.kind = _jsh.kind;
        this.name = _jsh.name;
        this.isFile = _jsh.kind == 'file';
        this.isDirectory = _jsh.kind == 'directory';
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
        const _js = toWrap || new index_1.JsSmbDirectoryHandle(url);
        super(_js.toHandle());
        this[_a] = this.entries;
        this[Symbol.asyncIterator] = this.entries;
        this._js = _js;
        this.kind = 'directory';
        this.isFile = false;
        this.isDirectory = true;
        this.getFile = this.getFileHandle;
        this.getDirectory = this.getDirectoryHandle;
        this.getEntries = this.values;
    }
    async *entries() {
        for await (const [key, value] of this._js.entries()) {
            yield [key, value instanceof index_1.JsSmbDirectoryHandle ? new SmbDirectoryHandle(value) : new SmbFileHandle(value)];
        }
    }
    async *keys() {
        for await (const key of this._js.keys()) {
            yield key;
        }
    }
    async *values() {
        for await (const value of this._js.values()) {
            yield value instanceof index_1.JsSmbDirectoryHandle ? new SmbDirectoryHandle(value) : new SmbFileHandle(value);
        }
    }
    async getDirectoryHandle(name, options) {
        return new Promise(async (resolve, reject) => {
            await this._js.getDirectoryHandle(name, options)
                .then((handle) => resolve(new SmbDirectoryHandle(handle)))
                .catch((reason) => {
                if (reason.message == 'The path supplied exists, but was not an entry of requested type.') {
                    reason.name = 'TypeMismatchError';
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
                if (reason.message == 'The path supplied exists, but was not an entry of requested type.') {
                    reason.name = 'TypeMismatchError';
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
}
exports.SmbDirectoryHandle = SmbDirectoryHandle;
_a = Symbol.asyncIterator;
class SmbFileHandle extends SmbHandle {
    constructor(_js) {
        super(_js.toHandle());
        this._js = _js;
        this.kind = 'file';
        this.isFile = true;
        this.isDirectory = false;
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
