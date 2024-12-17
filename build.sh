#!/bin/bash

set -ex

ARG1="$1"

LOCAL_TARGET_TRIPLE=`rustc --version --verbose | grep ^host | awk -F ' ' '{print $2}'`
TARGET_TRIPLE="${LOCAL_TARGET_TRIPLE}"

if [[ "${ARG1}" == "--target" ]]; then
  ARG2="$2"
  if [ -n "${ARG2}" ]; then
    TARGET_TRIPLE="${ARG2}"
  fi
fi

rustup target add ${TARGET_TRIPLE}
TARGET_TRIPLE_FOR_CC=`echo ${TARGET_TRIPLE} | sed 's/-unknown//g'`

if [[ "${TARGET_TRIPLE}" == *"linux"* ]]; then
  export BINDGEN_EXTRA_CLANG_ARGS="-I/usr/${TARGET_TRIPLE_FOR_CC}/include"
elif [[ "${TARGET_TRIPLE}" == *"darwin"* ]]; then
  TARGET_TRIPLE_FOR_CC=`echo ${TARGET_TRIPLE} | sed 's/aarch64/arm64/g'`
fi

./deps.sh ${TARGET_TRIPLE} ${TARGET_TRIPLE_FOR_CC}

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

LIBSMB_BASE="${SCRIPT_DIR}/libsmb2"
LIBSMB_BASE_INSTALL="${SCRIPT_DIR}/libsmb2/local-install/${TARGET_TRIPLE}"

NODE_ARCH=`echo ${TARGET_TRIPLE} | awk -F '-' '{print $1}' | sed 's/aarch64/arm64/g' | sed 's/x86_64/x64/g'`
NODE_PLATFORM=`echo ${TARGET_TRIPLE} | awk -F '-' '{print $2}'`
NODE_OS=`echo ${TARGET_TRIPLE} | awk -F '-' '{print $3}'`
NODE_OS_VARIANT=`echo ${TARGET_TRIPLE} | awk -F '-' '{print $4}'`

LIBSMB_BASE_LIB_INSTALL_PATH="${LIBSMB_BASE_INSTALL}/lib"

export LIBSMB_LIB_PATH="./lib/${NODE_OS}/${NODE_ARCH}"
if [ -n "${NODE_OS_VARIANT}" ]; then
  export LIBSMB_LIB_PATH="./lib/${NODE_OS}/${NODE_ARCH}/${NODE_OS_VARIANT}"
fi
mkdir -p ${LIBSMB_LIB_PATH}
cp ${LIBSMB_BASE_LIB_INSTALL_PATH}/libsmb2* ${LIBSMB_LIB_PATH}/

export LIBSMB_INCLUDE_PATH="${LIBSMB_BASE_INSTALL}/include"

export LIBSMB_LINK_STATIC="false"

export DYLD_LIBRARY_PATH=${LIBSMB_LIB_PATH}:$DYLD_LIBRARY_PATH
export LD_LIBRARY_PATH=${LIBSMB_LIB_PATH}:$LD_LIBRARY_PATH

export RUST_BACKTRACE=1

if [ "$ARG1" == "test" ]; then
  cargo test --release
else
  yarn build-tsc
  yarn build-napi --target ${TARGET_TRIPLE}
fi

if [ "${NODE_OS}" == "darwin" ]; then
  # rewrite dylib search path after build for macos
  install_name_tool -change ${LIBSMB_BASE_LIB_INSTALL_PATH}/libsmb2.4.dylib @loader_path/lib/${NODE_OS}/${NODE_ARCH}/libsmb2.4.dylib smb-js.${NODE_OS}-${NODE_ARCH}.node
fi