#!/bin/bash

set -e

if command -v brew 2>&1 >/dev/null ; then
    if ! readlink -f $(brew --prefix samba) 2>&1 >/dev/null ; then
        echo "brew installing samba"
        brew install samba
    fi
elif command -v apt-get 2>&1 >/dev/null ; then
    if ! command -v smbd 2>&1 >/dev/null ; then
        sudo apt-get update
        sudo apt-get install -y samba
    fi
fi