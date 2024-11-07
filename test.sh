#!/bin/bash

set -e
export RUST_BACKTRACE=1

export SMB_URL="smb://pi:pi@raspberrypi.local/tv"
#export SMB_URL="smb://pi:pi@192.168.68.71/pi"

# smb://[<domain>;][<user>@]<server>[:<port>]/<share>[/path][?arg=val[&arg=val]*]

echo "Test using mocks"
TEST_USING_MOCKS=1 yarn test-ava

export SMB_PATH="test"
echo "Test using NFS via libsmb2"
yarn test-ava
