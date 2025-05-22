#!/bin/bash
# Copyright 2025 NetApp Inc. All Rights Reserved.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
# SPDX-License-Identifier: Apache-2.0


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

SMB_JS_LIB_FULL_VER=`cat ${LIBSMB_BASE_LIB_INSTALL_PATH}/pkgconfig/libsmb2.pc | grep '^Version:' | awk '{print $2}'`
SMB_JS_LIB_VER=`echo ${SMB_JS_LIB_FULL_VER} | awk -F '.' '{print $1}'`

export LIBSMB_LIB_PATH="./lib/${NODE_OS}/${NODE_ARCH}"
if [ -n "${NODE_OS_VARIANT}" ]; then
  export LIBSMB_LIB_PATH="./lib/${NODE_OS}/${NODE_ARCH}/${NODE_OS_VARIANT}"
fi
mkdir -p ${LIBSMB_LIB_PATH}
if [ "${NODE_OS}" == "darwin" ]; then
  cp -R ${LIBSMB_BASE_LIB_INSTALL_PATH}/libsmb2.${SMB_JS_LIB_VER}.dylib ${LIBSMB_LIB_PATH}/
  cp -R ${LIBSMB_BASE_LIB_INSTALL_PATH}/libsmb2.dylib ${LIBSMB_LIB_PATH}/
elif [ "${NODE_OS}" == "linux" ]; then
  cp -R ${LIBSMB_BASE_LIB_INSTALL_PATH}/libsmb2.so* ${LIBSMB_LIB_PATH}/
fi

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
  install_name_tool -change ${LIBSMB_BASE_LIB_INSTALL_PATH}/libsmb2.${SMB_JS_LIB_VER}.dylib @loader_path/lib/${NODE_OS}/${NODE_ARCH}/libsmb2.${SMB_JS_LIB_VER}.dylib smb-js.${NODE_OS}-${NODE_ARCH}.node
fi