#!/bin/bash

set -e

./scripts/deps-test.sh

if command -v brew 2>&1 >/dev/null ; then
    SAMBA_DIR=`readlink -f $(brew --prefix samba)`
    echo "SAMBA_DIR: ${SAMBA_DIR}"
    SAMBA_SERVER="${SAMBA_DIR}/sbin/samba-dot-org-smbd"
else
    SAMBA_SERVER="smbd"
fi

SAMBA_CONFIG_DIR=`mktemp -d`
mkdir -p ${SAMBA_CONFIG_DIR}/private
mkdir -p ${SAMBA_CONFIG_DIR}/locks
echo "SAMBA_CONFIG_DIR: ${SAMBA_CONFIG_DIR}"
SHARE_DIR=`mktemp -d`
echo "SHARE_DIR: ${SHARE_DIR}"
mkdir -p  ${SHARE_DIR}/test
./scripts/setup-testdir.sh ${SHARE_DIR}/test

SAMBA_PORT="10445"

cat <<EOF > ${SAMBA_CONFIG_DIR}/smbd.conf
[global]
   workgroup = WORKGROUP
   log file = ${SAMBA_CONFIG_DIR}/smb.log
   max log size = 1000
   logging = file
   private dir = ${SAMBA_CONFIG_DIR}/private
   lock directory = ${SAMBA_CONFIG_DIR}/locks
   cache directory = ${SAMBA_CONFIG_DIR}/private
   pid directory = ${SAMBA_CONFIG_DIR}/locks
   state directory = ${SAMBA_CONFIG_DIR}/locks
   passdb backend = tdbsam:${SAMBA_CONFIG_DIR}/private/passdb.tdb
   winbindd socket directory = ${SAMBA_CONFIG_DIR}/locks
   winbindd privileged socket directory = ${SAMBA_CONFIG_DIR}/locks
   ncalrpc dir = ${SAMBA_CONFIG_DIR}/locks
   server role = standalone server
   unix password sync = no
   map to guest = bad user
   usershare allow guests = yes

[smbtest]
comment = smbtest
path = ${SHARE_DIR}
read only = no
browsable = yes
directory mask = 0777
create mask = 0666
guest ok = yes
EOF

$SAMBA_SERVER \
    --port=${SAMBA_PORT} \
    -s ${SAMBA_CONFIG_DIR}/smbd.conf \
    -F \
    --debug-stdout \
    -d 3 \
    --no-process-group 2>&1 > ${SAMBA_CONFIG_DIR}/smb-stdout.log &
SAMBA_PID=$!

sleep 1

function kill_samba() {
    EXITCODE=$?
        echo "Stopping samba EXITCODE=$EXITCODE"
        kill -9 $SAMBA_PID || true
        exit $EXITCODE
    if [ $EXITCODE -ne 0 ]; then
        cat ${SAMBA_CONFIG_DIR}/smb.log
    fi
}

trap kill_samba EXIT

export RUST_BACKTRACE=1

export SMB_URL="smb://guest@127.0.0.1:${SAMBA_PORT}/smbtest/test"

echo "Test using mocks"
TEST_USING_MOCKS=1 yarn test-ava

echo "Test using SMB via URL ${SMB_URL} via libsmb2"
yarn test-ava

