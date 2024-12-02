#!/bin/bash

mkdir -p npm

#cp .npmrc npm/darwin-arm64
#cp package.json npm npm
#cp index.* npm npm
#cp indax.* npm npm
#cp smb-js.darwin-arm64.node npm/darwin-arm64

#npm login --scope=@netapplabs --auth-type=legacy --registry=https://npm.pkg.github.com
#echo -e ''${GITHUB_USERNAME}'\n'${GITHUB_TOKEN}'' | npm login --scope=@netapplabs --auth-type=legacy --registry=https://npm.pkg.github.com

#npm login --scope=@netapplabs --auth-type=legacy --registry=https://npm.pkg.github.com << EOF
#${GITHUB_USERNAME}
#${GITHUB_TOKEN}
#EOF

npm publish 
