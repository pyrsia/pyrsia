#!/usr/bin/env zsh

set -e

SCRIPT_NAME=$(basename "$0")
if [ "$#" -lt 3 ]; then
  echo "Usage: ${SCRIPT_NAME} <build_version_number> <release_type> <arch_type>"
  exit 1
fi

#Fully Qualified Build Version Number. E.g. 1.0.1+5678
FQBVN=$1
#Release Type
RELTYPE=$2
#Architecture Type
ARCHTYPE=$3

case $RELTYPE in
  (latest|stable) ;;
  (*) echo "Invalid RELTYPE. Valid RELTYPE: latest|stable"; exit 1;;
esac

case $ARCHTYPE in
  (x86_64|arm64) ;;
  (*) echo "Invalid ARCHTYPE. Valid ARCHTYPE: x86_64|arm64"; exit 1;;
esac

mkdir -p syncdir
gsutil -m cp pyrsia-${FQBVN}.tar.gz  gs://homebrewrepo/${RELTYPE}/${ARCHTYPE}/pyrsia-${FQBVN}.tar.gz
listing="$(gsutil ls -lr gs://homebrewrepo)"
python3 .github/workflows/genlistingsyncoptimized.py ${listing} gs://homebrewrepo syncdir
python3 .github/workflows/genlisting.py syncdir -r -d
# sync back directory to Cloud Bucket excluding all .*.tar.gz files
gsutil -m -o "GSUtil:parallel_process_count=1" rsync -r -x ".*\.tar\.gz$" syncdir gs://homebrewrepo
