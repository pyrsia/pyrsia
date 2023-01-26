---
sidebar_position: 21
---

# Creating a Release for Pyrsia

Once the team decides to tag a release please use the following steps to ensure all parts of the service are updated.

To track this release, create a new issue titled with `Release vx.x.x`. Copy and paste the below sections as the issue
description. Next, follow and check the boxes in the issue as you move forward with the release activity.

## Before tagging the release in github

- [ ] Check and make sure the version in Cargo.toml of the main branch (or the commit) is correct and in tandem with
release version.
- [ ] Run the [integration tests](https://github.com/pyrsia/pyrsia-integration-tests/actions). Go to last instance of
successful run and click **Re-run all jobs** button to run integration test on the main branch. Record the run instance
link in the comments of the release issue. Ensure there are no failures - also ensure there is no flakiness
observed.
- [ ] Run [manual confidence tests](/docs/developers/prerelease_manual_tests.md) using a local build from the main branch
  - [ ] macOS (Scope: Authorize Node, Agent Node, Build Node)
  - [ ] Windows (Scope: Agent Node i.e. end user testing only)
  - [ ] Linux (Scope: Authorize Node, Agent Node, Build Node)
  - [ ] Docker (Scope: Agent Node i.e. end user testing only)

## Tagging the release

Once all the above steps are completed and verified to be success, start the release procedure:

- [ ] Go the [GitHub releases](https://github.com/pyrsia/pyrsia/releases) and [Draft a new release](https://github.com/pyrsia/pyrsia/releases/new)
- [ ] Select target branch `main`
- [ ] Click Choose a tag and type the tag name starting with a `v` e.g. `v0.2.2` - select "Create new tag on publish"
- [ ] Name the release: start with the tag, but make sure the title already includes a quick summary of the most important change(s)
- [ ] Click generate release notes. This will generate the technical release notes of all changes.
- [ ] Summarize the changes in a more readable list above the technical release notes - see [0.2.1 as an example](https://github.com/pyrsia/pyrsia/releases/tag/v0.2.1)
- [ ] Check the box for 'Set as pre-release' (for now)
- [ ] Make sure 'Set as latest release' is NOT checked (for now)
- [ ] Hit "Publish release" and wait for the workflow to finish

## Release to Nightly Cluster & Testing

- [ ] Deploy to nightly cluster - Refer [Managing Pyrsia on Kubernetes](https://pyrsia.io/docs/sre/kubernetes-helm/)
- [ ] Run installers + manual confidence tests connecting to nightly
  - [ ] macOS (Scope: Authorize Node, Agent Node, Build Node)
  - [ ] Windows (Scope: Agent Node i.e. end user testing only)
  - [ ] Linux (Scope: Authorize Node, Agent Node, Build Node) - install the latest stable release till #1509 is not closed using the following workaround

    ```shell
    # workaround
    sudo apt-get update
    sudo apt-get install -y wget gnupg
    sudo wget -q -O - https://repo.pyrsia.io/repos/Release.key |  gpg --dearmor  > pyrsia.gpg
    sudo install -o root -g root -m 644 pyrsia.gpg /etc/apt/trusted.gpg.d/
    rm pyrsia.gpg
    sudo su
    echo "deb https://repo.pyrsia.io/repos/stable focal main" >> /etc/apt/sources.list
    exit
    sudo apt-get update
    sudo apt-get install -y pyrsia

    # Some edits need to make to connect to nightly cluster.
    # Edits to set bootstrap-url nightly-cluster
    sudo sed -i '/ExecStart=/ s/$/ --bootstrap-url http:\/\/boot.nightly.pyrsia.link\/status/' $(sudo find /etc -name pyrsia.service)

    # reload & restart
    systemctl daemon-reload
    service pyrsia restart
    ```

  - [ ] Docker (Scope: Agent Node i.e. end user testing only)

## Deployment

- [ ] Make sure [apt repo](https://repo.pyrsia.io/repos/nightly/pool/main/p/pyrsia/) and
[brew repo](https://brewrepo.pyrsia.io/stable/x86_64/) contain the correct latest release
- [ ] Upload Windows MSI to GitHub release. Find the Windows installers from Artifacts section of GitHub action runs for
the release.
- [ ] Deploy the production authorized nodes with this release. Refer [Managing Pyrsia on Kubernetes](https://pyrsia.io/docs/sre/kubernetes-helm/)
- [ ] Run installers + [manual confidence tests](/docs/developers/postrelease_manual_tests.md) connecting to production
  - [ ] Linux
  - [ ] MacOS
  - [ ] Windows
  - [ ] Docker

## Post-release

- [ ] Edit the [GitHub release](https://github.com/pyrsia/pyrsia/releases) and uncheck 'Set as pre-release' and check 'Set as latest release'.
- [ ] Update documentation to point to the latest released version
- [ ] Update the version number to prepare for the next release
  - [ ] Make sure you update the version in `Cargo.toml`
  - [ ] Update github actions with the new version number eg. <https://github.com/pyrsia/pyrsia/pull/1349/files>
  - [ ] Create a PR with the version change and run it through the github actions to ensure nothing fails.
  - [ ] Verify that the rust toolchain version is set to the version we would like to release this version. Since 0.2.2 this is captured in one place - at the top of `Cargo.toml`
  - [ ] Merge the PR to the main branch

## Outreach

- [ ] Add a blog to promote this release - like <https://pyrsia.io/blog/2022/11/30/pyrsia-0.2.1-released/>
- [ ] Record a demo video for the release while showcasing most feature or with any significant learning to share.
