---
sidebar_position: 13
---

# Testing in Pyrsia

## Overview

Depending on the purpose, Pyrsia uses several methods and tools for testing as follows.

| **Goal (What to verify)** | Unit tests | Tests on cloud | Integration tests | Manual tests |
| ------------------------- | ---------- | -------------- | ----------------- | ------------ |
| Features (Comprehensive check)  | ✅ | -  | ✅ | -  |
| Features Backward Compatibility | -  | ✅ | ✅ | ✅ |
| Data Backward Compatibility     | -  | ✅ | -  | -  |
| Cross-platform                  | -  | -  | -  | ✅ |
| Clusters on cloud               | -  | ✅ | -  | -  |

## Tests details and how to run them

### Unit tests in the Pyrsia repository

Unit tests check if a small piece of source code works individually.

They are written in Rust files of the Pyrsia repository,
and executed on GitHub Actions whenever the production code is modified.

### Tests on cloud instances

TODO

### Integration tests

Integration tests verify all basic features are
not broken - [The repository pyrsia/pyrsia-integration-tests](https://github.com/pyrsia/pyrsia-integration-tests). Unlike unit tests, they test combined modules by

Currently, these tests run twice a day regularly using GitHub Actions.

### Manual tests

Manual tests should be done every time a new release is published to find unexpected behavior in all supported platforms.

Refer to [Post-release manual tests](/docs/developers/postrelease_manual_tests.md).
