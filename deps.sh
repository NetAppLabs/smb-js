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

if ! command -v add-apt-repository 2>&1 >/dev/null ; then
    if command -v apt-get 2>&1 >/dev/null ; then
        sudo apt-get update
        sudo apt-get -y install software-properties-common
    fi
fi

if command -v lsb_release 2>&1 >/dev/null ; then
    lsb_rel_version=`lsb_release -c | grep '^Codename:' | awk -F ' ' '{print $2}'`
    if [ "${lsb_rel_version}" == "focal" ]; then
        # install backported autoconf 2.71 backported for ubuntu 20.04 / focal
        sudo add-apt-repository ppa:savoury1/build-tools -y
        sudo apt-get -y install autoconf2.71
    fi
fi

if ! command -v yacc 2>&1 >/dev/null ; then
    if command -v brew 2>&1 >/dev/null ; then
        brew install byacc
    elif command -v apt-get 2>&1 >/dev/null ; then
        sudo apt-get update
        sudo apt-get install -y byacc
    else
        echo "please install yacc"
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

PROCS=8
YACC="yacc"
if [ "${OS}" == "Darwin" ]; then
    YACC="/opt/homebrew/bin/byacc"

    if [ ! -f libsmb2/local-install/${TARGET_TRIPLE}/lib/libsmb2.a ]; then
        pushd libsmb2
        CURDIR="$(pwd)"
        INSTALL_DIR="${CURDIR}/local-install/${TARGET_TRIPLE}"
        mkdir -p "${INSTALL_DIR}"
        chmod 775 ./bootstrap
        ./bootstrap
        ./configure \
            --disable-werror \
            --host=${TARGET_TRIPLE_FOR_CC} \
            --prefix="${INSTALL_DIR}" \
            --exec-prefix="${INSTALL_DIR}" \
            CFLAGS="-fsanitize=address -fPIC" \
            LDFLAGS="-framework GSS" \
            YACC="${YACC}"
        make clean
        make -j${PROCS} install
        popd
    fi

elif [ "${OS}" == "Linux" ]; then

    MAIN_CURDIR="$(pwd)"

    EXTRA_CFLAGS=""
    EXTRA_LDFLAGS=""

    HOST_ARCH=`uname -m`
    COMPILE_FOR_ARCH=`echo ${TARGET_TRIPLE} | awk -F '-' '{print $1}'`
    CROSS_COMPILE="false"
    if [ "${HOST_ARCH}" != "${COMPILE_FOR_ARCH}" ]; then
        CROSS_COMPILE="true"
    fi

    if [ "${CROSS_COMPILE}" == "true" ]; then
        OPENSSL_INSTALL_DIR="${MAIN_CURDIR}/openssl/local-install/${TARGET_TRIPLE}"
        EXTRA_CFLAGS="-I${OPENSSL_INSTALL_DIR}/include"
        EXTRA_LDFLAGS="-L${OPENSSL_INSTALL_DIR}/lib"
        if [ ! -e openssl ]; then
            mkdir -p ${OPENSSL_INSTALL_DIR}
            echo "building openssl for cross compile"
            if [ "${HOST_ARCH}" == "x86_64" ]; then
                sudo add-apt-repository -s "deb http://archive.ubuntu.com/ubuntu $(lsb_release -sc) main restricted universe multiverse" -y
            else
                sudo add-apt-repository -s "deb http://ports.ubuntu.com/ubuntu-ports $(lsb_release -sc) main restricted universe multiverse" -y
            fi
            apt-get source openssl
            OPENSSL_VER=`apt-cache showsrc openssl | grep '^Version' | awk '{print $2}' | awk -F '-' '{print $1}'`
            OPENSSL_SRC_DIR="openssl-${OPENSSL_VER}"
            pushd ${OPENSSL_SRC_DIR}
            ./Configure linux-${COMPILE_FOR_ARCH} --prefix=${OPENSSL_INSTALL_DIR} CC=${COMPILE_FOR_ARCH}-linux-gnu-gcc
            make -j${PROCS}
            make -j${PROCS} install
            popd
        fi
    fi

    if [ ! -e krb5 ]; then
        git clone --branch krb5-1.21.3-final https://github.com/krb5/krb5.git
    fi

    KRB5_INSTALL_DIR="${MAIN_CURDIR}/krb5/local-install/${TARGET_TRIPLE}"

    if [ ! -e krb5/local-install/${TARGET_TRIPLE}/lib/krb5 ]; then
        pushd krb5
        CURDIR="$(pwd)"
        INSTALL_DIR="${CURDIR}/local-install/${TARGET_TRIPLE}"
        mkdir -p "${INSTALL_DIR}"
        BUILD_DIR="${CURDIR}/local-build/${TARGET_TRIPLE}"
        mkdir -p "${BUILD_DIR}"
        pushd src
        # for cross compile to work
        export krb5_cv_attr_constructor_destructor=yes
        export ac_cv_func_regcomp=yes
        export ac_cv_printf_positional=yes
        autoreconf --force
        pushd ${BUILD_DIR}
        ../../src/configure \
            --host=${TARGET_TRIPLE_FOR_CC} \
            --prefix="${INSTALL_DIR}" \
            --exec-prefix="${INSTALL_DIR}" \
             --enable-static \
             --disable-shared \
            CFLAGS="-fPIC -fcommon -Wno-cast-align ${EXTRA_CFLAGS}" \
            LDFLAGS="${EXTRA_LDFLAGS}"
        make -j${PROCS}
        make install
        popd
        popd
        popd
    fi

    if [ ! -f libsmb2/local-install/${TARGET_TRIPLE}/lib/libsmb2.a ]; then
        pushd libsmb2
        CURDIR="$(pwd)"
        INSTALL_DIR="${CURDIR}/local-install/${TARGET_TRIPLE}"
        BUILD_DIR="${CURDIR}/local-build/${TARGET_TRIPLE}"
        mkdir -p "${BUILD_DIR}"
        mkdir -p "${INSTALL_DIR}"
        chmod 775 ./bootstrap
        ./bootstrap
        pushd ${BUILD_DIR}
        ../../configure \
            --host=${TARGET_TRIPLE_FOR_CC} \
            --prefix="${INSTALL_DIR}" \
            --exec-prefix="${INSTALL_DIR}" \
            CFLAGS="-fPIC -Wno-cast-align -I${KRB5_INSTALL_DIR}/include -fsanitize=address" \
            LDFLAGS="-L${KRB5_INSTALL_DIR}/lib -fsanitize=address" \
            LIBS="-lasan -lubsan -ldl -lgssapi_krb5 -lkrb5 -lcom_err -lgssrpc -lk5crypto -lkdb5 -lkrad -lkrb5_db2 -lkrb5_k5tls -lkrb5_otp -lkrb5_spake -lkrb5support -lverto -lresolv"
        make -j${PROCS} clean all
        make install
        popd
        popd
    fi

else
    echo "unsupported platform ${OS}"
fi
