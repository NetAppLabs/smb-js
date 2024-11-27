#!/bin/bash

set -e

ARG="$1"

./deps.sh

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

LIBSMB_BASE="${SCRIPT_DIR}/libsmb2"
LIBSMB_BASE_INSTALL="${SCRIPT_DIR}/libsmb2/local-install"
export LIBSMB_LIB_PATH="${LIBSMB_BASE}/lib/.libs/"
export LIBSMB_INCLUDE_PATH="${LIBSMB_BASE_INSTALL}/include"

export LIBSMB_LINK_STATIC="true"

export DYLD_LIBRARY_PATH=${LIBSMB_LIB_PATH}:$DYLD_LIBRARY_PATH
export LD_LIBRARY_PATH=${LIBSMB_LIB_PATH}:$LD_LIBRARY_PATH

export RUST_BACKTRACE=1

if [ "$ARG" == "test" ]; then
  cargo test
else
  yarn build-napi
fi
