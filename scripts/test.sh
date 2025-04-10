#!/bin/bash

set -e

./scripts/deps-test.sh

if command -v brew 2>&1 >/dev/null ; then
    SAMBA_DIR=`readlink -f $(brew --prefix samba)`
    echo "SAMBA_DIR: ${SAMBA_DIR}"
    SAMBA_SERVER="${SAMBA_DIR}/sbin/samba-dot-org-smbd"
    PDBEDIT_COMMAND="${SAMBA_DIR}/bin/pdbedit"
else
    SAMBA_SERVER="smbd"
    PDBEDIT_COMMAND="pdbedit"
fi


SAMBA_DEFAULT_USER="guest"
SAMBA_DEFAULT_PASSWORD=""

# SAMBA_DEFAULT_USER="${USER}"
# SAMBA_DEFAULT_PASSWORD="smbpass"

SAMBA_CONFIG_DIR=`mktemp -d`
mkdir -p ${SAMBA_CONFIG_DIR}/private
mkdir -p ${SAMBA_CONFIG_DIR}/locks
chmod 755 ${SAMBA_CONFIG_DIR}/locks
echo "SAMBA_CONFIG_DIR: ${SAMBA_CONFIG_DIR}"

SHARE_DIR_TEMP=`mktemp -d`
SHARE_DIR="${SHARE_DIR_TEMP}/smbtest"

echo "SHARE_DIR: ${SHARE_DIR}"
mkdir -p  ${SHARE_DIR}
chmod 777 ${SHARE_DIR}
./scripts/setup-testdir.sh ${SHARE_DIR}

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
   ;winbindd privileged socket directory = ${SAMBA_CONFIG_DIR}/locks
   ncalrpc dir = ${SAMBA_CONFIG_DIR}/locks
   server role = standalone server
   unix password sync = no
   usershare allow guests = no

[smbtest]
comment = smbtest
path = ${SHARE_DIR}
public = yes
writable = yes
browseable = yes
inherit permissions = yes
vfs mkdir use tmp name = no
EOF

if [ "${SAMBA_DEFAULT_USER}" != "guest" ]; then
    echo -e "${SAMBA_DEFAULT_PASSWORD}\n${SAMBA_DEFAULT_PASSWORD}\n" | ${PDBEDIT_COMMAND} --configfile ${SAMBA_CONFIG_DIR}/smbd.conf -t -a ${SAMBA_DEFAULT_USER}
fi

$SAMBA_SERVER \
    --port=${SAMBA_PORT} \
    -s ${SAMBA_CONFIG_DIR}/smbd.conf \
    -F \
    --debug-stdout \
    -d 3 \
    --no-process-group 2>&1 > ${SAMBA_CONFIG_DIR}/smb-stdout.log &
SAMBA_PID=$!

function kill_samba() {
    EXITCODE=$?
    echo "Stopping samba EXITCODE=$EXITCODE"
    kill -9 $SAMBA_PID || true
    if [ $EXITCODE -ne 0 ]; then
        if [ -f ${SAMBA_CONFIG_DIR}/smb.log ]; then
            cat ${SAMBA_CONFIG_DIR}/smb.log
        elif [ -f ${SAMBA_CONFIG_DIR}/smb-stdout.log ]; then
            cat ${SAMBA_CONFIG_DIR}/smb-stdout.log
        fi
    fi
    exit $EXITCODE
}

trap kill_samba EXIT

function wait_samba() {
    for ((i=1;i<=10;i++)); do
        SAMBA_STATE=$(grep "waiting for connections" ${SAMBA_CONFIG_DIR}/smb-stdout.log || true)
        if [ "$SAMBA_STATE" == "waiting for connections" ]; then
            return
        fi
        if [ $i -lt 10 ]; then
            echo "Waiting for samba to accept connections"
            sleep 1
        fi
    done
    echo "Gave up waiting for samba to accept connections"
    exit 1
}

export RUST_BACKTRACE=1

export SMB_URL="smb://${SAMBA_DEFAULT_USER}:${SAMBA_DEFAULT_PASSWORD}@127.0.0.1:${SAMBA_PORT}/smbtest?sec=ntlmssp"

echo "Test using mocks"
TEST_USING_MOCKS=1 yarn test-ava

echo "Test using SMB via URL ${SMB_URL} via libsmb2"
wait_samba
yarn test-ava
