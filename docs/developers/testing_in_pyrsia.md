---
sidebar_position: 13
---

# Testing in Pyrsia

## Overview

Depending on the purpose, Pyrsia uses several methods and tools for testing as follows.

| **Goal (What to verify)** | Unit tests | Tests on Staging | Tests on Production | Integration tests | Manual tests |
| ------------------------- | ---------- | ---------------- | ------------------- | ----------------- | ------------ |
| Features (Comprehensive check)  | ✅ | -  | -  | ✅ | -  |
| Features Backward Compatibility | -  | ✅ | ✅ | ✅ | ✅ |
| Data Backward Compatibility     | -  | ✅ | ✅ | -  | -  |
| Cross-platform                  | -  | -  | -  | -  | ✅ |
| Operations on Cloud Environment | -  | ✅ | ✅ | -  | -  |

## Tests details and how / who to run them

All contributors are required to keep 'unit tests' and 'integration tests' green when making some changes to code base.

Other methods introduced below are mainly operated by the Pyrsia team
(You can reach out in [#pyrsia-team of CD Foundation Slack](https://cdeliveryfdn.slack.com/join/shared_invite/zt-1eryue9cw-9YpgrfIfsTcDS~hGHchURg))
for now.

### Unit tests in the Pyrsia repository

Unit tests check if a small piece of source code works individually.

They are written in Rust files of the Pyrsia repository,
and executed on GitHub Actions whenever the production code is modified.

### Tests on Staging and Production

We have two environments, where authorized nodes run: Production and Staging. They are Kubernetes clusters on AWS and GCP.

- Production - A public environment on which the latest stable version runs for everyone who wants to use Pyrsia.
- Staging - It is for testing new features.

On both environments, a test is executed before and after releases to check functions.
It builds some official libraries and sees inspect-logs of them as simple operation verifications.

Another important part of it is verifying that backward compatibility is kept,
so data like blockchain records and transparency logs should not be purged after each test
as long as the current version is compatible with the previous ones.

### Integration tests

Integration tests verify all basic features are
not broken - [The repository pyrsia/pyrsia-integration-tests](https://github.com/pyrsia/pyrsia-integration-tests).
Unlike unit tests, they test combined modules by actually running Pyrsia nodes and CLI.

Currently, these tests run twice a day regularly using GitHub Actions.

### Manual tests

Manual tests should be done every time a new release is published to find unexpected behavior in all supported platforms.

For more details, refer to [Post-release manual tests](/docs/developers/postrelease_manual_tests.md).
