# `@netapplabs/smb-js`

![https://github.com/NetAppLabs/smb-js/actions](https://github.com/NetAppLabs/smb-js/actions/workflows/node.js.yml/badge.svg)

> SMB filesystem implementation for JavaScript/TypeScript.


## Install this package

Add an .npmrc file to your home directory or readable location

```
//npm.pkg.github.com/:_authToken=${GITHUB_TOKEN}
@netapplabs:registry=https://npm.pkg.github.com/
```

```
yarn add @netapplabs/smb-js
```

# Usage

### Example JavasScript usage using Local (NT authentication):

Note: May need to have "?sec=ntlmssp" argument on the URL connection string.

```
import { SmbDirectoryHandle, SmbFileHandle } from '@netapplabs/smb-js'

let smbURL="smb://myuser:mypassword@127.0.0.1:445/share?sec=ntlmssp";
rootDir = new SmbDirectoryHandle(smbURL);
let subPath = "sub-dir";
let subDir = await rootDir.getDirectoryHandle(subPath);
let subFileHandle = await subDir.getFileHandle("sub-file")
let subFile = await subFileHandle.getFile();
const textContents = await subFile.text();
console.log("textContents: ", textContents);
```

### Example JavasScript usage using AD authentication:

Note: May need to have "?sec=krb5cc" argument on the URL connection string.


Needs to have environment variables SMB_USER, SMB_PASSWORD and SMB_DOMAIN set before connecting.

```
import { default as process } from 'node:process'
import { SmbDirectoryHandle, SmbFileHandle } from '@netapplabs/smb-js'

process.env.SMB_USER="<ad-user>";
process.env.SMB_PASSWORD="<ad-password>";
process.env.SMB_DOMAIN="<ad-domain>";

let smbURL="smb://127.0.0.1:445/share?sec=krb5cc";
rootDir = new SmbDirectoryHandle(smbURL);
let subPath = "sub-dir";
let subDir = await rootDir.getDirectoryHandle(subPath);
let subFileHandle = await subDir.getFileHandle("sub-file")
let subFile = await subFileHandle.getFile();
const textContents = await subFile.text();
console.log("textContents: ", textContents);
```

## Support matrix

### Operating Systems

|                  | node18 | node20 | node22 |
| ---------------- | ------ | ------ | ------ |
| macOS x64        | ✓      | ✓      | ✓      |
| macOS arm64      | ✓      | ✓      | ✓      |
| Linux x64 gnu    | ✓      | ✓      | ✓      |
| Linux arm64 gnu  | ✓      | ✓      | ✓      |

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
- Install `Node.js@20+` which fully supported `Node-API`
- C compiler (gcc/clang)
- Install `yarn@1.x`

## Test in local

- yarn
- yarn build
- yarn test

And you will see:

```bash
$ ava --verbose

  ✔ test ...
  ─

  x tests passed
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
