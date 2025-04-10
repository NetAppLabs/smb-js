#!/bin/bash

set -e

S3_TEST_DIR=$1

mkdir -p ${S3_TEST_DIR}/first ${S3_TEST_DIR}/quatre
echo -n "In order to make sure that this file is exactly 123 bytes in size, I have written this text while watching its chars count." > ${S3_TEST_DIR}/annar
touch ${S3_TEST_DIR}/3 ${S3_TEST_DIR}/first/comment ${S3_TEST_DIR}/quatre/points

chmod 777 ${S3_TEST_DIR}/quatre
chmod 777 ${S3_TEST_DIR}/first
chmod 666 ${S3_TEST_DIR}/annar
chmod 666 ${S3_TEST_DIR}/3
chmod 666 ${S3_TEST_DIR}/first/comment
chmod 666 ${S3_TEST_DIR}/quatre/points