---
sidebar_position: 22
---

# Post-release manual tests

After a release has been deployed, run these manual tests to make sure everything works as expected. Run these tests after each deployment to

- Nightly cluster
- Production cluster

When the new version is deployed, run checks for all supported platforms:

- Windows
- Linux
- MacOS
- Docker

Run through these steps:

- Install Pyrsia using one of the installers following the instructions on pyrsia.io
- Make sure to configure the node to use `--bootstrap-url http://boot.nightly.pyrsia.link/status` as the bootstrap URL
- Make sure to test both with and without existing data in the pyrsia folder (keypair, artifacts, blocks, log db)
- View your logs and check for anomalies
- Check to see if transparency logs can be inspected (make sure to use the installed pyrsia cli - not a local build)

```sh
pyrsia inspect-log docker --image alpine:3.16.0
```

- check to see if artifacts can be downloaded

```sh
curl http://0.0.0.0:7888/v2/library/alpine/manifests/3.16.0
```

- check to see if new builds can be requested.
  - find an artifact version that is not in the transparency log yet
  - configure your docker client to use pyrsia
  - try to pull the artifact - check the logs that a build has been requested
  - wait a while and check to see if the build was added to the transparency log
- Check the release notes of the new version
