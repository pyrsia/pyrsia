#!/usr/bin/env bash

set -e
# Download and install sccache in the Linux specific cargo directories
mkdir -p /home/runner/.cargo/bin
curl -o- -sSLf https://github.com/mozilla/sccache/releases/download/v0.2.15/sccache-v0.2.15-x86_64-unknown-linux-musl.tar.gz | tar --strip-components=1 -C /home/runner/.cargo/bin -xzf -
chmod 755 /home/runner/.cargo/bin/sccache
