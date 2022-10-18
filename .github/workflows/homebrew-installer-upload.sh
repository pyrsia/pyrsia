#!/usr/bin/env zsh

set -e

SCRIPT_NAME=$(basename "$0")
if [ "$#" -lt 3 ]; then
  echo "Usage: ${SCRIPT_NAME} <build_version_number> <release_type> <arch_type>"
  exit 1
fi

FQBVN=$1
RELTYPE=$2
ARCHTYPE=$3

case $RELTYPE in
  (nightly|stable) ;;
  (*) echo "Invalid RELTYPE. Valid RELTYPE: nightly|stable"; exit 1;;
esac

case $ARCHTYPE in
  (x86_64|arm64) ;;
  (*) echo "Invalid ARCHTYPE. Valid ARCHTYPE: x86_64|arm64"; exit 1;;
esac

gsutil -m cp pyrsia-${FQBVN}.tar.gz  gs://homebrewrepo/${RELTYPE}/${ARCHTYPE}/pyrsia-${FQBVN}.tar.gz
