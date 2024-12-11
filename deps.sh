#!/bin/bash

set -ex
TARGET_TRIPLE=$1
TARGET_TRIPLE_FOR_CC=$2

if ! command -v git 2>&1 >/dev/null ; then
    if command -v brew 2>&1 >/dev/null ; then
        brew install git
    elif command -v apt-get 2>&1 >/dev/null ; then
        sudo apt-get update
        sudo apt-get install -y git-all
    else
        echo "please install git"
    fi
fi

git submodule update --init

if ! command -v cargo 2>&1 >/dev/null ; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
fi

if ! command -v automake 2>&1 >/dev/null ; then
    if command -v brew 2>&1 >/dev/null ; then
        brew install automake
    elif command -v apt-get 2>&1 >/dev/null ; then
        sudo apt-get update
        sudo apt-get install -y automake
    else
        echo "please install automake"
    fi
fi

OS=`uname -s`
if [ "${OS}" == "Darwin" ]; then
    if ! command -v glibtool 2>&1 >/dev/null ; then
        if command -v brew 2>&1 >/dev/null ; then
            brew install libtool
        fi
    fi
elif [ "${OS}" == "Linux" ]; then
    if ! command -v make 2>&1 >/dev/null ; then
        if command -v apt-get 2>&1 >/dev/null ; then
            sudo apt-get update
            sudo apt-get install -y make
        else
            echo "please install make"
        fi
    fi
    if ! command -v node 2>&1 >/dev/null ; then
        if command -v apt-get 2>&1 >/dev/null ; then
            sudo apt-get update
            curl -sL https://deb.nodesource.com/setup_22.x -o /tmp/nodesource_setup.sh
            chmod 775 /tmp/nodesource_setup.sh
            sudo /tmp/nodesource_setup.sh
            sudo apt-get install nodejs -y
        else
            echo "please install node"
        fi
    fi
    if ! command -v clang 2>&1 >/dev/null ; then
        if command -v apt-get 2>&1 >/dev/null ; then
            sudo apt-get update
            sudo apt-get install -y clang
        else
            echo "please install clang"
        fi
    fi
    if ! command -v yarn 2>&1 >/dev/null ; then
        sudo npm install -g yarn
    fi
    if ! command -v libtoolize 2>&1 >/dev/null ; then
        if command -v apt-get 2>&1 >/dev/null ; then
            sudo apt-get update
            sudo apt-get install -y libtool
        else
            echo "please install libtool"
        fi
    fi
fi

if [ ! -e krb5 ]; then
     git clone https://github.com/krb5/krb5.git
fi

MAIN_CURDIR="$(pwd)"
KRB5_INSTALL_DIR="${MAIN_CURDIR}/krb5/local-install/${TARGET_TRIPLE}"

if [ ! -e krb5/local-install/${TARGET_TRIPLE}/lib/krb5 ]; then
     pushd krb5
     CURDIR="$(pwd)"
     INSTALL_DIR="${CURDIR}/local-install/${TARGET_TRIPLE}"
     mkdir -p "${INSTALL_DIR}"
     cd src
     autoreconf
     ./configure \
        --host=${TARGET_TRIPLE_FOR_CC} \
        --prefix="${INSTALL_DIR}" \
        --exec-prefix="${INSTALL_DIR}" \
        CFLAGS='-fPIC -Wno-cast-align'
    make clean all
    make install
    cd ..
    popd
fi

if [ "${OS}" == "Darwin" ]; then
    KRB5_INSTALL_DIR="/opt/homebrew/opt/krb5"
fi

if [ ! -f libsmb2/local-install/${TARGET_TRIPLE}/lib/libsmb2.a ]; then
     pushd libsmb2
     CURDIR="$(pwd)"
     INSTALL_DIR="${CURDIR}/local-install/${TARGET_TRIPLE}"
     mkdir -p "${INSTALL_DIR}"
     chmod 775 ./bootstrap
     ./bootstrap
     ./configure \
        --host=${TARGET_TRIPLE_FOR_CC} \
        --prefix="${INSTALL_DIR}" \
        --exec-prefix="${INSTALL_DIR}" \
        CFLAGS="-fPIC -Wno-cast-align -I${KRB5_INSTALL_DIR}/include" \
        LDFLAGS="-L${KRB5_INSTALL_DIR}/lib"
     make clean all
     make install
     popd
fi

