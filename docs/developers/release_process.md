# Making a Release for Pyrsia

Once the team decides to tag a release please use the following steps to ensure all parts of the service are updated.

## Before tagging the release in github

- [ ] Make sure you update the version in `Cargo.toml`. Run the PR through the github actions to ensure nothing fails.
This is the PR that will be tagged with the version number that matches the one in the `Cargo.toml`
- [ ] Verify that the rust toolchain version is set to the version we would like to release this version. Since 0.2.2 this is captured in one place - at the top of `Cargo.toml`

## Tagging the release

- [ ] Commit the PR to the main branch
- [ ] Update github actions with the new version number eg. <https://github.com/pyrsia/pyrsia/pull/1349/files>
- [ ] Once all the above steps are completed and verified to be success, tag the release in github. Github also helps you add the release notes - although they could use some context and explaning to make them easy to understand. Fill in that text to simplify the release notes and add the generated release notes in the release.
- [ ] Update release notes, convert the release to the latest release and edit the name of the release to provide high level features that were added eg. <https://github.com/pyrsia/pyrsia/releases/tag/v0.2.1>

## Testing the release

- [ ] Run integration tests and ensure there are no failures - also ensure there is no flakiness observed.
- [ ] Record the results of the integration tests so that we can attach them to the release notes.
- [ ] Run manual confidence tests - this requires to run Pyrsia node as part of the network and exercising the new features manually. We may be able to automate parts of this but not completely since the network itself is complex to build a workflow on.
- [ ] Deploy to nightly cluster
- [ ] Run installer tests connecting to nightly

## Deployment

- [ ] Make sure apt repo and brew repo contain the correct latest release
- [ ] Upload windows MSI to github release
- [ ] Deploy the authorized nodes with this release
- [ ] Deploy to production cluster
- [ ] Run installer tests connecting to production

## Outreach

- [ ] Update documentation to include latest released version
- [ ] Add a blog to promote this release - like <https://pyrsia.io/blog/2022/11/30/pyrsia-0.2.1-released/>
