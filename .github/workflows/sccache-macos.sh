#!/usr/bin/env bash

# Download and install sccache in the OS/X specific cargo directories
mkdir -p /Users/runner/.cargo/bin 2>/dev/null
curl -o- -sSLf https://github.com/mozilla/sccache/releases/download/v0.2.15/sccache-v0.2.15-x86_64-apple-darwin.tar.gz | tar xzf -
mv sccache-v0.2.15-x86_64-apple-darwin/sccache /Users/runner/.cargo/bin/sccache 
chmod 755 /Users/runner/.cargo/bin/sccache
