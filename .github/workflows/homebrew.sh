#!/usr/bin/env zsh

set -e

#Extract version number from Cargo.toml
sed -n 's/\(^version = \"\)\([[:digit:]]\(.[[:digit:]]\+\)\+\)\(.*\)\(\"$\)/\2\4/p' Cargo.toml
