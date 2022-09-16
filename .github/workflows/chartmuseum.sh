#!/usr/bin/env bash

set -e

RELTYPE=$1

if [ "$RELTYPE" == "" ]; then
  RELTYPE="nightly"
fi

WORKSPACE=$PWD
cd installers/helm
mkdir -p repos/$RELTYPE
gsutil -m rsync -r gs://helmrepo/repos repos
helm package pyrsia-node
mv pyrsia-node*.tgz repos/$RELTYPE
cd repos/$RELTYPE
helm repo index --url https://helmrepo.pyrsia.io/repos/$RELTYPE .
cp ../../pyrsia-node/artifacthub-repo.yaml .
cd ../..

# Generate pretty directory listing web pages
python3 $WORKSPACE/.github/workflows/genlisting.py -r

# copy new public repo to GCS
gsutil -m rsync -r repos gs://helmrepo/repos
