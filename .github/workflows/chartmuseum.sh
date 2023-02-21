#!/usr/bin/env bash

set -e

RELTYPE=$1

if [ "$RELTYPE" == "" ]; then
  RELTYPE="nightly"
fi

WORKSPACE=$PWD
cd installers/helm
mkdir -p repos/$RELTYPE
helm package pyrsia-node
mv pyrsia-node*.tgz repos/$RELTYPE
gsutil -m cp repos/${RELTYPE}/pyrsia-node*.tgz  gs://helmrepo/repos/${RELTYPE}/
listing="$(gsutil ls -lr gs://helmrepo/repos)"
cd repos/$RELTYPE
helm repo index --url https://helmrepo.pyrsia.io/repos/$RELTYPE .
cp ../../pyrsia-node/artifacthub-repo.yaml .
cd ../..
python3 --version
python3 $WORKSPACE/.github/workflows/genlistingsyncoptimized.py "${listing}" gs://helmrepo/repos repos
# Generate pretty directory listing web pages
python3 $WORKSPACE/.github/workflows/genlisting.py -r -p

# copy new public repo to GCS. Excluding all *.tgz files from sync back
gsutil -m rsync -r -x ".*\.tgz$" repos gs://helmrepo/repos
