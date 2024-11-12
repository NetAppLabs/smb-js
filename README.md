# `@wasmin/smb-js`

![https://github.com/NetAppLabs/smb-js/actions](https://github.com/NetAppLabs/smb-js/actions/workflows/node.js.yml/badge.svg)

> SMB filesystem implementation for JavaScript/TypeScript.

# Usage

```
import { SmbDirectoryHandle, SmbFileHandle } from '@wasmin/smb-js'

let smbURL="smb://myuser:mypassword@127.0.0.1:445/share";
rootDir = new SmbDirectoryHandle(smbURL);
let subPath = "sub-dir";
let subDir = await rootDir.getDirectoryHandle(subPath);
let subFileHandle = await subDir.getFileHandle("sub-file")
let subFile = await subFileHandle.getFile();
const textContents = await subFile.text();
console.log("textContents: ", textContents);
```

## Install this package

```
yarn add @wasmin/smb-js
```

## Support matrix

### Operating Systems

|                  | node14 | node16 | node18 |
| ---------------- | ------ | ------ | ------ |
| macOS x64        | ✓      | ✓      | ✓      |
| macOS arm64      | ✓      | ✓      | ✓      |
| Linux x64 gnu    | ✓      | ✓      | ✓      |
| Linux x64 musl   | ✓      | ✓      | ✓      |
| Linux arm gnu    | ✓      | ✓      | ✓      |
| Linux arm64 gnu  | ✓      | ✓      | ✓      |
| Linux arm64 musl | ✓      | ✓      | ✓      |

## Ability

### Build

After `yarn build/npm run build` command, you can see `smb-js.[darwin|win32|linux].node` file in project root. This is the native addon built from [lib.rs](./src/lib.rs).

### Test

With [ava](https://github.com/avajs/ava), run `yarn test/npm run test` to testing native addon. You can also switch to another testing framework if you want.

### CI

With GitHub actions, every commits and pull request will be built and tested automatically in [`node@14`, `node@16`, `@node18`] x [`macOS`, `Linux`, `Windows`] matrix. You will never be afraid of the native addon broken in these platforms.

## Develop requirements

- Install latest `Rust`
  - Install via e.g. `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Install `Node.js@10+` which fully supported `Node-API`
- Install `yarn@1.x`

## Test in local

- yarn
- yarn build
- yarn test

And you will see:

```bash
$ ava --verbose

  ✔ sync function from native code
  ✔ sleep function from native code (201ms)
  ─

  2 tests passed
✨  Done in 1.12s.
```

## Release package

Ensure you have set you **NPM_TOKEN** in `GitHub` project setting.

In `Settings -> Secrets`, add **NPM_TOKEN** into it.

When you want release package:

```
npm version [<newversion> | major | minor | patch | premajor | preminor | prepatch | prerelease [--preid=<prerelease-id>] | from-git]

git push
```

GitHub actions will do the rest job for you.
